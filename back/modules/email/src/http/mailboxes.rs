use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_sync::{
    CreateMailboxRequest, MailProviderResponse, MailboxResponse, UpdateMailboxRequest,
};
use crate::service::mailboxes as service;
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "mailboxes";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_mail_providers))
        .routes(routes!(list_mailboxes, create_mailbox))
        .routes(routes!(get_mailbox, update_mailbox, delete_mailbox))
        .routes(routes!(test_mailbox))
        .routes(routes!(sync_mailbox))
}

#[utoipa::path(
    get,
    path = "/mailboxes/providers",
    tag = TAG,
    operation_id = "mailboxes_providers",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Mail providers available in the organization: the core `imap` provider (credentials form) and the providers of enabled modules (`oauth` with the page that starts the consent)", body = Data<Vec<MailProviderResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_mail_providers(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<MailProviderResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let providers = service::providers(&ctx)?;
    Ok(Json(Data::new(
        providers.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/mailboxes",
    tag = TAG,
    operation_id = "mailboxes_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "The user's own mailboxes with the synchronization state. Credentials never come back, only `credentials_configured`", body = Data<Vec<MailboxResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_mailboxes(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<MailboxResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mailboxes = service::list(&ctx).await?;
    Ok(Json(Data::new(
        mailboxes.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/mailboxes",
    tag = TAG,
    operation_id = "mailboxes_create",
    security(("session_cookie" = [])),
    request_body = CreateMailboxRequest,
    responses(
        (status = 201, description = "Mailbox connected for the current user (the provider validates the settings and credentials and answers the address). Credentials go to the secret vault; publishes `mailbox.connected` with audit and queues the first synchronization", body = Data<MailboxResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 409, description = "The user already connected this address (`mailbox_already_connected`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid input (`validation_error`), unavailable provider, or the provider refused the connection (`mail_provider_authentication`, `mail_provider_unavailable`, `mail_provider_rejected`, `mail_provider_unsupported`)", body = ErrorEnvelope),
    )
)]
async fn create_mailbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateMailboxRequest>,
) -> Result<(StatusCode, Json<Data<MailboxResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mailbox = service::create(&ctx, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(mailbox.into()))))
}

#[utoipa::path(
    get,
    path = "/mailboxes/{mailbox_id}",
    tag = TAG,
    operation_id = "mailboxes_get",
    security(("session_cookie" = [])),
    params(("mailbox_id" = String, Path, description = "Mailbox id")),
    responses(
        (status = 200, description = "One of the user's mailboxes", body = Data<MailboxResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Mailbox not found or owned by another user (`mailbox_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_mailbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(mailbox_id): Path<String>,
) -> Result<Json<Data<MailboxResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mailbox = service::get(&ctx, &mailbox_id).await?;
    Ok(Json(Data::new(mailbox.into())))
}

#[utoipa::path(
    patch,
    path = "/mailboxes/{mailbox_id}",
    tag = TAG,
    operation_id = "mailboxes_update",
    security(("session_cookie" = [])),
    params(("mailbox_id" = String, Path, description = "Mailbox id")),
    request_body = UpdateMailboxRequest,
    responses(
        (status = 200, description = "Mailbox updated. New `settings` or `credentials` are validated with the provider again (the address must stay the same) and resume a paused synchronization; publishes `mailbox.updated` with audit (credentials redacted)", body = Data<MailboxResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Mailbox not found (`mailbox_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid input (`validation_error`), different address (`mailbox_address_mismatch`) or provider error (`mail_provider_*`)", body = ErrorEnvelope),
    )
)]
async fn update_mailbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(mailbox_id): Path<String>,
    JsonBody(request): JsonBody<UpdateMailboxRequest>,
) -> Result<Json<Data<MailboxResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mailbox = service::update(&ctx, &mailbox_id, request.into_command(client.0.ip)).await?;
    Ok(Json(Data::new(mailbox.into())))
}

#[utoipa::path(
    delete,
    path = "/mailboxes/{mailbox_id}",
    tag = TAG,
    operation_id = "mailboxes_delete",
    security(("session_cookie" = [])),
    params(("mailbox_id" = String, Path, description = "Mailbox id")),
    responses(
        (status = 204, description = "Mailbox removed with its conversations, files and credentials; publishes `mailbox.disconnected` with audit"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Mailbox not found (`mailbox_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_mailbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(mailbox_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::delete(&ctx, &mailbox_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/mailboxes/{mailbox_id}/test",
    tag = TAG,
    operation_id = "mailboxes_test",
    security(("session_cookie" = [])),
    params(("mailbox_id" = String, Path, description = "Mailbox id")),
    responses(
        (status = 200, description = "Connection tested with the stored credentials; the status becomes `active`", body = Data<MailboxResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Mailbox not found (`mailbox_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The provider refused the connection (`mail_provider_*`); the status becomes `error` with the reason", body = ErrorEnvelope),
    )
)]
async fn test_mailbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(mailbox_id): Path<String>,
) -> Result<Json<Data<MailboxResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mailbox = service::test(&ctx, &mailbox_id).await?;
    Ok(Json(Data::new(mailbox.into())))
}

#[utoipa::path(
    post,
    path = "/mailboxes/{mailbox_id}/sync",
    tag = TAG,
    operation_id = "mailboxes_sync",
    security(("session_cookie" = [])),
    params(("mailbox_id" = String, Path, description = "Mailbox id")),
    responses(
        (status = 202, description = "Synchronization queued (job `email.sync.mailbox`, one at a time per mailbox); also resumes a synchronization paused after errors"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Mailbox not found (`mailbox_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The mailbox is disconnected (`mailbox_disconnected`)", body = ErrorEnvelope),
    )
)]
async fn sync_mailbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(mailbox_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::request_sync(&ctx, &mailbox_id).await?;
    Ok(StatusCode::ACCEPTED)
}
