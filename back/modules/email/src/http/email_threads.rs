use axum::Json;
use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_sync::{
    EmailThreadDetailResponse, EmailThreadResponse, EmailThreadsQueryParams,
    LinkEmailThreadRequest, MyEmailThreadsQueryParams, UpdateEmailThreadRequest,
};
use crate::service::{linking, reading};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::files::content_disposition;
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "email threads";
const ATTACHMENT_CONTENT_SECURITY_POLICY: &str = "default-src 'none'; sandbox";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_email_threads))
        .routes(routes!(list_my_email_threads))
        .routes(routes!(get_email_thread, update_email_thread))
        .routes(routes!(link_email_thread))
        .routes(routes!(unlink_email_thread))
        .routes(routes!(download_email_attachment))
}

#[utoipa::path(
    get,
    path = "/email-threads",
    tag = TAG,
    operation_id = "email_threads_list",
    security(("session_cookie" = [])),
    params(EmailThreadsQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Conversations linked to the record, newest first. `private` conversations of other owners are left out; with `access = metadata` the snippet is `null`", body = Page<EmailThreadResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.read` or read access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_email_threads(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<EmailThreadsQueryParams>, QueryRejection>,
) -> Result<Json<Page<EmailThreadResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = reading::list_for_record(
        &ctx,
        &params.record_type,
        &params.record_id,
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/email-threads/mine",
    tag = TAG,
    operation_id = "email_threads_mine",
    security(("session_cookie" = [])),
    params(MyEmailThreadsQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Conversations of the user's own mailboxes (optionally one mailbox), newest first", body = Page<EmailThreadResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.read` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Mailbox not found (`mailbox_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_my_email_threads(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<MyEmailThreadsQueryParams>, QueryRejection>,
) -> Result<Json<Page<EmailThreadResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing =
        reading::list_mine(&ctx, params.mailbox_id.as_deref(), window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/email-threads/{thread_id}",
    tag = TAG,
    operation_id = "email_threads_get",
    security(("session_cookie" = [])),
    params(("thread_id" = String, Path, description = "Conversation id")),
    responses(
        (status = 200, description = "Conversation with its messages. Other users need a readable linked record; with `access = metadata` preview, body and attachments are empty; tracking only for the owner; HTML is sanitized", body = Data<EmailThreadDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.read` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Conversation not found, private or without a readable record (`email_thread_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_email_thread(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(thread_id): Path<String>,
) -> Result<Json<Data<EmailThreadDetailResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let detail = reading::get(&ctx, &thread_id).await?;
    Ok(Json(Data::new(detail.into())))
}

#[utoipa::path(
    patch,
    path = "/email-threads/{thread_id}",
    tag = TAG,
    operation_id = "email_threads_update",
    security(("session_cookie" = [])),
    params(("thread_id" = String, Path, description = "Conversation id")),
    request_body = UpdateEmailThreadRequest,
    responses(
        (status = 200, description = "Visibility of the conversation changed by the mailbox owner (`null` goes back to the mailbox default); publishes `email_thread.visibility_changed` with audit", body = Data<EmailThreadDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only the owner changes the visibility (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Conversation not found (`email_thread_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid visibility (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_email_thread(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(thread_id): Path<String>,
    JsonBody(request): JsonBody<UpdateEmailThreadRequest>,
) -> Result<Json<Data<EmailThreadDetailResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let detail =
        reading::set_visibility(&ctx, &thread_id, request.visibility.as_deref(), client.0.ip)
            .await?;
    Ok(Json(Data::new(detail.into())))
}

#[utoipa::path(
    post,
    path = "/email-threads/{thread_id}/links",
    tag = TAG,
    operation_id = "email_threads_link",
    security(("session_cookie" = [])),
    params(("thread_id" = String, Path, description = "Conversation id")),
    request_body = LinkEmailThreadRequest,
    responses(
        (status = 201, description = "Conversation linked to the record by hand (an automatic link becomes manual); publishes `email_thread.linked` with audit", body = Data<EmailThreadDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Needs full access to the conversation and read access to the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Conversation not found (`email_thread_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn link_email_thread(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(thread_id): Path<String>,
    JsonBody(request): JsonBody<LinkEmailThreadRequest>,
) -> Result<(StatusCode, Json<Data<EmailThreadDetailResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let detail = linking::link(
        &ctx,
        &thread_id,
        &request.record_type,
        &request.record_id,
        client.0.ip,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(detail.into()))))
}

#[utoipa::path(
    delete,
    path = "/email-threads/{thread_id}/links/{record_type}/{record_id}",
    tag = TAG,
    operation_id = "email_threads_unlink",
    security(("session_cookie" = [])),
    params(
        ("thread_id" = String, Path, description = "Conversation id"),
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 204, description = "Link removed; publishes `email_thread.unlinked` with audit"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Needs full access to the conversation and read access to the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Conversation or link not found (`email_thread_not_found`, `email_link_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unlink_email_thread(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((thread_id, record_type, record_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    linking::unlink(&ctx, &thread_id, &record_type, &record_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/email-messages/{message_id}/attachments/{attachment_id}",
    tag = TAG,
    operation_id = "email_messages_attachment",
    security(("session_cookie" = [])),
    params(
        ("message_id" = String, Path, description = "Message id"),
        ("attachment_id" = String, Path, description = "Attachment id"),
    ),
    responses(
        (status = 200, description = "Attachment content (only with full access to the conversation)", content_type = "application/octet-stream"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Attachment not found or not readable (`email_attachment_not_found`)", body = ErrorEnvelope),
    )
)]
async fn download_email_attachment(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((message_id, attachment_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let download = reading::attachment(&ctx, &message_id, &attachment_id).await?;
    let content_type = HeaderValue::from_str(&download.attachment.mime_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let disposition = HeaderValue::from_str(&content_disposition(&download.attachment.name))
        .map_err(|_| AppError::Internal("invalid content disposition".to_owned()))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, download.attachment.size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(
            header::CONTENT_SECURITY_POLICY,
            ATTACHMENT_CONTENT_SECURITY_POLICY,
        )
        .body(Body::from_stream(download.content))
        .map_err(|error| {
            ApiError::from(AppError::Internal(format!(
                "cannot build download response: {error}"
            )))
        })
}
