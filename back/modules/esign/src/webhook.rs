use std::sync::LazyLock;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

use crate::clicksign::{self, SIGNATURE_HEADER};
use crate::runtime::acting_scope;
use crate::service::{self, NoticeOutcome};
use crate::store;
use crate::{TAG, WEBHOOK_SECRET_SETTING, secret};

pub(crate) const INVALID_SIGNATURE: &str = "invalid_webhook_signature";
pub(crate) const INVALID_PAYLOAD: &str = "esign_invalid_webhook_payload";
pub(crate) const PAYLOAD_TOO_LARGE: &str = "esign_webhook_payload_too_large";
pub(crate) const WEBHOOK_NOT_CONFIGURED: &str = "esign_webhook_not_configured";
pub(crate) const OWNER_UNAVAILABLE: &str = "esign_owner_unavailable";

const MAX_BODY_BYTES: usize = 512 * 1024;
const WEBHOOK_LIMIT: u32 = 600;
const MINUTE: Duration = Duration::from_secs(60);

static LIMITER: LazyLock<SharedRateLimiter> =
    LazyLock::new(|| SharedRateLimiter::new("esign.webhook", WEBHOOK_LIMIT, MINUTE));

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(receive))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EsignWebhookResult {
    #[schema(example = "applied")]
    pub(crate) status: String,
}

fn answer(status: &str) -> axum::Json<Data<EsignWebhookResult>> {
    axum::Json(Data {
        data: EsignWebhookResult {
            status: status.to_owned(),
        },
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[utoipa::path(
    post,
    path = "/public/esign/webhook",
    tag = TAG,
    operation_id = "esign_public_webhook",
    description = "Receives Clicksign webhooks. No session: the body must be signed with the webhook secret configured in the module (`Content-Hmac: sha256=<hex(hmac_sha256(secret, raw body))>`). Idempotent by body: repeated deliveries answer `duplicate`. Updates the signer and request status and publishes `esign.signed`, `esign.refused`, `esign.cancelled`, `esign.completed` or `esign.expired` on the deal.",
    request_body(description = "Clicksign webhook payload (up to 512 KB)", content((Object = "application/json"))),
    responses(
        (status = 200, description = "`applied`, `ignored` (event without effect or unknown envelope) or `duplicate`", body = Data<EsignWebhookResult>),
        (status = 401, description = "Missing or invalid `Content-Hmac` (`invalid_webhook_signature`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Body is not JSON (`esign_invalid_webhook_payload`) or above 512 KB (`esign_webhook_payload_too_large`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests (`rate_limited`)", body = ErrorEnvelope),
        (status = 503, description = "Webhook secret not configured (`esign_webhook_not_configured`) or no member able to act for the request (`esign_owner_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn receive(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<axum::Json<Data<EsignWebhookResult>>, ApiError> {
    LIMITER
        .check("webhook", "too many requests, try again later")
        .await?;
    let system = AccessScope::system();
    let ctx = Ctx::from_state(&state, &system).await?;
    if body.len() > MAX_BODY_BYTES {
        return Err(AppError::validation_code(
            PAYLOAD_TOO_LARGE,
            "the webhook body exceeds 512 KB",
        )
        .into());
    }
    let secret = secret(&ctx, WEBHOOK_SECRET_SETTING)
        .await?
        .filter(|secret| !secret.is_empty())
        .ok_or_else(|| {
            AppError::unavailable_code(
                WEBHOOK_NOT_CONFIGURED,
                "the webhook secret is not configured",
            )
        })?;
    let signature = headers
        .get(SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok());
    if !clicksign::verify_signature(&secret, signature, &body) {
        return Err(
            AppError::unauthorized_code(INVALID_SIGNATURE, "invalid webhook signature").into(),
        );
    }
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::validation_code(INVALID_PAYLOAD, "the webhook body must be JSON"))?;
    let body_hash = hex(&Sha256::digest(&body));
    let Some(notice) = clicksign::parse_webhook(&payload) else {
        return Ok(answer("ignored"));
    };
    let Some(request) = store::find_by_reference(ctx.pool(), &notice.references).await? else {
        return Ok(answer("ignored"));
    };
    let scope = acting_scope(&ctx, request.created_by)
        .await?
        .ok_or_else(|| {
            AppError::unavailable_code(
                OWNER_UNAVAILABLE,
                "no active member can act on this signature request",
            )
        })?;
    if notice.kind.is_none() {
        service::record_ignored(&ctx, &body_hash, &notice.event_name).await?;
        return Ok(answer("ignored"));
    }
    match service::apply_notice(&ctx, &scope, request.id, &notice, &body_hash).await? {
        NoticeOutcome::Applied => Ok(answer("applied")),
        NoticeOutcome::Ignored => Ok(answer("ignored")),
        NoticeOutcome::Duplicate => Ok(answer("duplicate")),
    }
}
