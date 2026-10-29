use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_sync::{CreateEmailBlockRequest, EmailBlockResponse};
use crate::service::capture;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "email settings";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_email_blocklist, create_email_block))
        .routes(routes!(delete_email_block))
}

#[utoipa::path(
    get,
    path = "/email-blocklist",
    tag = TAG,
    operation_id = "email_blocklist_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Domains and addresses never imported by the synchronization (a domain also covers its subdomains)", body = Data<Vec<EmailBlockResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_email_blocklist(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<EmailBlockResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let entries = capture::blocklist(&ctx).await?;
    Ok(Json(Data::new(
        entries.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/email-blocklist",
    tag = TAG,
    operation_id = "email_blocklist_create",
    security(("session_cookie" = [])),
    request_body = CreateEmailBlockRequest,
    responses(
        (status = 201, description = "Entry added (value normalized to lowercase); publishes `email_blocklist.added` with audit", body = Data<EmailBlockResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The entry already exists (`email_block_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid kind or value (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_email_block(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateEmailBlockRequest>,
) -> Result<(StatusCode, Json<Data<EmailBlockResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let entry = capture::add_block(&ctx, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(entry.into()))))
}

#[utoipa::path(
    delete,
    path = "/email-blocklist/{entry_id}",
    tag = TAG,
    operation_id = "email_blocklist_delete",
    security(("session_cookie" = [])),
    params(("entry_id" = String, Path, description = "Blocklist entry id")),
    responses(
        (status = 204, description = "Entry removed; publishes `email_blocklist.removed` with audit"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Entry not found (`email_block_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_email_block(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(entry_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    capture::remove_block(&ctx, &entry_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}
