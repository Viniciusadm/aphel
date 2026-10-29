use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::AppState;

use crate::service::{self, INVALID_REQUEST, SendInput, SignerInput};
use crate::store::{self, RequestRow, SignerRow};
use crate::{KEY, SEND, TAG, VIEW, can};

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(list_requests, create_request))
        .routes(routes!(get_request))
        .routes(routes!(cancel_request))
}

fn require_viewer(scope: &AccessScope) -> Result<(), AppError> {
    if !can(scope, &VIEW) {
        scope.require(&SEND)?;
    }
    Ok(())
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw.trim()).map_err(|_| {
        AppError::invalid_fields_code(
            INVALID_REQUEST,
            "invalid identifier",
            vec![FieldViolation::new(field, "must be a valid id")],
        )
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EsignSigner {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) contact_id: Option<String>,
    #[schema(example = "pending")]
    pub(crate) status: String,
    pub(crate) signed_at: Option<String>,
    pub(crate) refused_at: Option<String>,
}

impl From<SignerRow> for EsignSigner {
    fn from(row: SignerRow) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            email: row.email,
            contact_id: row.contact_id.map(|id| id.to_string()),
            status: row.status,
            signed_at: row.signed_at.map(timestamp),
            refused_at: row.refused_at.map(timestamp),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EsignRequest {
    pub(crate) id: String,
    pub(crate) deal_id: String,
    pub(crate) file_id: String,
    pub(crate) file_name: String,
    #[schema(example = "clicksign")]
    pub(crate) provider: String,
    #[schema(example = "sandbox")]
    pub(crate) environment: String,
    #[schema(example = "sent")]
    pub(crate) status: String,
    pub(crate) message: Option<String>,
    pub(crate) signed_file_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
    pub(crate) signers: Vec<EsignSigner>,
}

pub(crate) fn present(row: RequestRow, signers: Vec<SignerRow>) -> EsignRequest {
    EsignRequest {
        id: row.id.to_string(),
        deal_id: row.deal_id.to_string(),
        file_id: row.file_id.to_string(),
        file_name: row.file_name,
        provider: row.provider,
        environment: row.environment,
        status: row.status,
        message: row.message,
        signed_file_id: row.signed_file_id.map(|id| id.to_string()),
        created_by: row.created_by.to_string(),
        created_at: timestamp(row.created_at),
        updated_at: timestamp(row.updated_at),
        completed_at: row.completed_at.map(timestamp),
        signers: signers.into_iter().map(Into::into).collect(),
    }
}

pub(crate) async fn present_many(
    ctx: &Ctx<'_>,
    rows: Vec<RequestRow>,
) -> Result<Vec<EsignRequest>, AppError> {
    let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let mut signers = store::signers_of(ctx.pool(), &ids).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let (own, rest): (Vec<SignerRow>, Vec<SignerRow>) = signers
                .drain(..)
                .partition(|signer| signer.request_id == row.id);
            signers = rest;
            present(row, own)
        })
        .collect())
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EsignStatus {
    #[schema(example = "clicksign")]
    pub(crate) provider: String,
    #[schema(example = "sandbox")]
    pub(crate) environment: String,
    pub(crate) configured: bool,
    pub(crate) webhook_url: String,
    pub(crate) legal_notice: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct EsignListQuery {
    pub(crate) deal_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct EsignSignerInput {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) contact_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateEsignRequest {
    pub(crate) deal_id: String,
    pub(crate) file_id: String,
    pub(crate) signers: Vec<EsignSignerInput>,
    pub(crate) message: Option<String>,
    pub(crate) quote_id: Option<String>,
}

impl CreateEsignRequest {
    pub(crate) fn into_input(self) -> Result<SendInput, AppError> {
        let deal_id = parse_uuid(&self.deal_id, "deal_id")?;
        let file_id = parse_uuid(&self.file_id, "file_id")?;
        let quote_id = self
            .quote_id
            .as_deref()
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
            .map(|raw| parse_uuid(raw, "quote_id"))
            .transpose()?;
        let mut signers = Vec::with_capacity(self.signers.len());
        for (index, signer) in self.signers.into_iter().enumerate() {
            let contact_id = signer
                .contact_id
                .as_deref()
                .map(str::trim)
                .filter(|raw| !raw.is_empty())
                .map(|raw| parse_uuid(raw, &format!("signers.{index}.contact_id")))
                .transpose()?;
            signers.push(SignerInput {
                name: signer.name,
                email: signer.email,
                contact_id,
            });
        }
        Ok(SendInput {
            deal_id,
            file_id,
            signers,
            message: self.message,
            quote_id,
        })
    }
}

#[utoipa::path(
    get,
    path = "/esign/status",
    tag = TAG,
    operation_id = "esign_status",
    security(("session_cookie" = [])),
    description = "Provider, environment, whether the access token is configured (never the token itself) and the webhook URL to register in Clicksign. Requires `esign.view` or `esign.send`.",
    responses(
        (status = 200, description = "Module status", body = Data<EsignStatus>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn status(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<EsignStatus>>, ApiError> {
    require_viewer(&tenant.scope)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data {
        data: EsignStatus {
            provider: crate::clicksign::PROVIDER_KEY.to_owned(),
            environment: service::environment(&ctx).await?,
            configured: service::is_configured(&ctx).await?,
            webhook_url: format!(
                "{}/api/v1/public/{KEY}/webhook",
                state.auth_settings.app_base_url.get().trim_end_matches('/')
            ),
            legal_notice: "Sem validade jurídica própria: a validade da assinatura é a do provedor (Clicksign).".to_owned(),
        },
    }))
}

#[utoipa::path(
    get,
    path = "/esign/requests",
    tag = TAG,
    operation_id = "esign_requests_list",
    security(("session_cookie" = [])),
    description = "Signature requests of a deal, newest first (up to 100). Requires `esign.view` (or `esign.send`) and `deals.read` on the deal.",
    params(EsignListQuery),
    responses(
        (status = 200, description = "Signature requests", body = Data<Vec<EsignRequest>>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid deal id (`esign_invalid_request`)", body = ErrorEnvelope),
    )
)]
async fn list_requests(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<EsignListQuery>, QueryRejection>,
) -> Result<Json<Data<Vec<EsignRequest>>>, ApiError> {
    let Query(query) = query
        .map_err(|rejection| AppError::validation_code(INVALID_REQUEST, rejection.body_text()))?;
    require_viewer(&tenant.scope)?;
    let scope = &tenant.scope;
    scope.require(&Permission::DEALS_READ)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let deal_id = parse_uuid(&query.deal_id, "deal_id")?;
    service::load_deal(&ctx, scope, deal_id).await?;
    let rows = store::list_for_deal(ctx.pool(), deal_id).await?;
    Ok(Json(Data {
        data: present_many(&ctx, rows).await?,
    }))
}

#[utoipa::path(
    post,
    path = "/esign/requests",
    tag = TAG,
    operation_id = "esign_requests_create",
    security(("session_cookie" = [])),
    description = "Queues a PDF attached to the deal for electronic signature and publishes `esign.queued`. The worker reauthorizes the request, sends it to Clicksign and publishes `esign.sent` after confirmation. The initial status is `queued`; `delivery_unknown` means provider confirmation is pending or uncertain and must be checked before submitting another request. `quote_id` (optional) ties the request to a quote version of the deal (usually the PDF generated from it by the proposals module) and goes in the event payload. Requires `esign.send` and access to the deal and the file. No legal validity of its own: the validity is the provider's.",
    request_body = CreateEsignRequest,
    responses(
        (status = 201, description = "Request queued", body = Data<EsignRequest>),
        (status = 403, description = "Missing `esign.send` or no access to the file (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found (`deal_not_found`), file not found or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid request (`esign_invalid_request`), file not attached to the deal (`esign_file_not_attached`), not a PDF (`esign_file_not_pdf`), above 20 MB (`esign_file_too_large`) or refused by the provider (`esign_provider_rejected`)", body = ErrorEnvelope),
        (status = 503, description = "Access token not configured (`esign_not_configured`), refused (`esign_provider_unauthorized`) or provider unavailable (`esign_provider_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn create_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CreateEsignRequest>,
) -> Result<(StatusCode, Json<Data<EsignRequest>>), ApiError> {
    let scope = &tenant.scope;
    scope.require(&SEND)?;
    scope.require(&Permission::DEALS_READ)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let (row, signer_rows) = service::send(&ctx, scope, request.into_input()?).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data {
            data: present(row, signer_rows),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/esign/requests/{request_id}",
    tag = TAG,
    operation_id = "esign_requests_get",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Signature request id")),
    responses(
        (status = 200, description = "Signature request", body = Data<EsignRequest>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`esign_request_not_found`), deal not visible (`deal_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn get_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(request_id): Path<String>,
) -> Result<Json<Data<EsignRequest>>, ApiError> {
    require_viewer(&tenant.scope)?;
    let scope = &tenant.scope;
    scope.require(&Permission::DEALS_READ)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let id = Uuid::parse_str(request_id.trim()).map_err(|_| service::request_not_found())?;
    let row = store::find_request(ctx.pool(), id)
        .await?
        .ok_or_else(service::request_not_found)?;
    service::load_deal(&ctx, scope, row.deal_id).await?;
    let mut items = present_many(&ctx, vec![row]).await?;
    Ok(Json(Data {
        data: items.remove(0),
    }))
}

#[utoipa::path(
    post,
    path = "/esign/requests/{request_id}/cancel",
    tag = TAG,
    operation_id = "esign_requests_cancel",
    security(("session_cookie" = [])),
    description = "Cancels a queued request locally or queues cancellation of a sent request (`cancel_pending`, `esign.cancel_queued`). The worker publishes `esign.cancelled` after Clicksign confirms it. Requires `esign.send`.",
    params(("request_id" = String, Path, description = "Signature request id")),
    responses(
        (status = 200, description = "Request cancelled locally or cancellation queued", body = Data<EsignRequest>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`esign_request_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 409, description = "Request already closed (`esign_request_closed`)", body = ErrorEnvelope),
        (status = 422, description = "Refused by the provider (`esign_provider_rejected`)", body = ErrorEnvelope),
        (status = 503, description = "Provider not configured or unavailable (`esign_not_configured`, `esign_provider_unauthorized`, `esign_provider_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn cancel_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(request_id): Path<String>,
) -> Result<Json<Data<EsignRequest>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&SEND)?;
    scope.require(&Permission::DEALS_READ)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let id = Uuid::parse_str(request_id.trim()).map_err(|_| service::request_not_found())?;
    let row = service::cancel(&ctx, scope, id).await?;
    let mut items = present_many(&ctx, vec![row]).await?;
    Ok(Json(Data {
        data: items.remove(0),
    }))
}
