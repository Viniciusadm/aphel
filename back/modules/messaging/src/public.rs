use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{self, Value, json};
use sha2::Sha256;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::channels::{self, APP_SECRET, ChannelKind, VERIFY_TOKEN, parse_webhook};
use crate::errors::{
    WEBHOOK_INVALID_PAYLOAD, WEBHOOK_PAYLOAD_TOO_LARGE, WHATSAPP_INVALID_SIGNATURE,
    WHATSAPP_INVALID_VERIFY_TOKEN, channel_not_found,
};
use crate::store::{self, StoredChannel};
use crate::text::{constant_time_eq, decode_hex};
use crate::{TAG, jobs, secret};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::ErrorEnvelope;
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::ClientInfo;
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
const VERIFY_LIMIT: u32 = 30;
const WEBHOOK_LIMIT: u32 = 1200;
const MINUTE: Duration = Duration::from_secs(60);
const SIGNATURE_HEADER: &str = "x-hub-signature-256";

static VERIFY_LIMITER: LazyLock<SharedRateLimiter> =
    LazyLock::new(|| SharedRateLimiter::new("messaging.verify", VERIFY_LIMIT, MINUTE));
static WEBHOOK_LIMITER: LazyLock<SharedRateLimiter> =
    LazyLock::new(|| SharedRateLimiter::new("messaging.webhook", WEBHOOK_LIMIT, MINUTE));

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(verify_webhook, receive_webhook))
}

async fn limit(limiter: &SharedRateLimiter, key: &str) -> Result<(), AppError> {
    limiter
        .check(key, "too many requests, try again later")
        .await
}

async fn resolve(ctx: &Ctx<'_>, channel_id: &str) -> Result<StoredChannel, AppError> {
    let id = Uuid::parse_str(channel_id).map_err(|_| channel_not_found())?;
    store::find_channel_for_webhook(
        ctx.pool(),
        id,
        ChannelKind::parse("whatsapp").expect("channel key"),
    )
    .await?
    .ok_or_else(channel_not_found)
}

async fn channel_secret(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
    name: &str,
) -> Result<Option<String>, AppError> {
    Ok(secret(ctx, &channels::secret_name(channel.id, name))
        .await?
        .filter(|value| !value.is_empty()))
}

fn plain_text(body: String) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[utoipa::path(
    get,
    path = "/public/messaging/whatsapp/{channel_id}",
    tag = TAG,
    operation_id = "messaging_whatsapp_verify",
    params(
        ("channel_id" = String, Path, description = "WhatsApp channel id (shown on the channel screen)"),
        ("hub.mode" = String, Query, description = "Always `subscribe`"),
        ("hub.verify_token" = String, Query, description = "The verify token configured on the channel"),
        ("hub.challenge" = String, Query, description = "Value echoed back on success"),
    ),
    responses(
        (status = 200, description = "Webhook verification by Meta: echoes `hub.challenge` as text when the verify token matches. No authentication", content_type = "text/plain", body = String),
        (status = 403, description = "Wrong verify token or mode (`whatsapp_invalid_verify_token`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown channel or module disabled (`messaging_channel_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn verify_webhook(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(channel_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    limit(
        &VERIFY_LIMITER,
        &format!("verify:{}", client.ip_or_unknown()),
    )
    .await?;
    let scope = AccessScope::system();
    let ctx = Ctx::from_state(&state, &scope).await?;
    let channel = resolve(&ctx, &channel_id).await?;
    let expected = channel_secret(&ctx, &channel, VERIFY_TOKEN).await?;
    let provided = params
        .get("hub.verify_token")
        .map(String::as_str)
        .unwrap_or_default();
    let challenge = params
        .get("hub.challenge")
        .map(String::as_str)
        .unwrap_or_default();
    let valid = params.get("hub.mode").map(String::as_str) == Some("subscribe")
        && expected
            .as_deref()
            .is_some_and(|expected| constant_time_eq(expected.as_bytes(), provided.as_bytes()))
        && !challenge.is_empty()
        && challenge.len() <= 128
        && challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !valid {
        return Err(AppError::forbidden_code(
            WHATSAPP_INVALID_VERIFY_TOKEN,
            "invalid verify token",
        )
        .into());
    }
    Ok(plain_text(challenge.to_owned()))
}

fn signature_valid(secret: &str, headers: &HeaderMap, body: &[u8]) -> bool {
    let Some(signature) = headers
        .get(SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("sha256="))
        .and_then(|hex| decode_hex(&hex.to_ascii_lowercase()))
    else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&signature).is_ok()
}

#[utoipa::path(
    post,
    path = "/public/messaging/whatsapp/{channel_id}",
    tag = TAG,
    operation_id = "messaging_whatsapp_webhook",
    params(("channel_id" = String, Path, description = "WhatsApp channel id")),
    request_body(description = "WhatsApp Cloud API webhook (`messages` field), up to 1 MB, signed with `X-Hub-Signature-256` (HMAC-SHA256 of the raw body with the app secret)", content(
        (Object = "application/json")
    )),
    responses(
        (status = 200, description = "Accepted. Each new message is queued for the job `messaging.inbound` (idempotent by the provider message id); delivery statuses update the sent messages. Payloads of an inactive channel are acknowledged and ignored"),
        (status = 401, description = "Missing or wrong signature (`whatsapp_invalid_signature`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown channel or module disabled (`messaging_channel_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Body above 1 MB (`messaging_payload_too_large`) or not JSON (`messaging_invalid_payload`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests for the channel (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn receive_webhook(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<axum::Json<Value>, ApiError> {
    limit(&WEBHOOK_LIMITER, &format!("webhook:{channel_id}")).await?;
    if body.len() > MAX_PAYLOAD_BYTES {
        return Err(
            AppError::validation_code(WEBHOOK_PAYLOAD_TOO_LARGE, "payload above 1 MB").into(),
        );
    }
    let scope = AccessScope::system();
    let ctx = Ctx::from_state(&state, &scope).await?;
    let channel = resolve(&ctx, &channel_id).await?;
    let secret = channel_secret(&ctx, &channel, APP_SECRET).await?;
    if !secret
        .as_deref()
        .is_some_and(|secret| signature_valid(secret, &headers, &body))
    {
        return Err(
            AppError::unauthorized_code(WHATSAPP_INVALID_SIGNATURE, "invalid signature").into(),
        );
    }
    if !channel.is_active {
        return Ok(axum::Json(json!({ "status": "ignored" })));
    }
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::validation_code(WEBHOOK_INVALID_PAYLOAD, "invalid JSON"))?;
    let phone_number_id = channel
        .config
        .get("phone_number_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let webhook = parse_webhook(&payload, phone_number_id);
    let pool = ctx.pool().clone();
    for message in &webhook.messages {
        if !store::message_exists(&pool, channel.id, &message.external_id).await? {
            jobs::enqueue_inbound(&ctx, &channel, message).await?;
        }
    }
    for status in &webhook.statuses {
        store::apply_delivery_status(
            &pool,
            &channel,
            &status.external_id,
            &status.status,
            status.at,
            status.error.as_deref(),
        )
        .await?;
    }
    Ok(axum::Json(json!({ "status": "ok" })))
}
