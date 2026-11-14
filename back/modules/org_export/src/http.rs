use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{OrgExportRequest, OrgExportResponse};
use crate::service::{self, RequestOrgExportCommand};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::files::content_disposition;
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "organization-exports";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(request_org_export, list_org_exports))
        .routes(routes!(get_org_export))
        .routes(routes!(download_org_export))
        .routes(routes!(delete_org_export))
}

#[utoipa::path(
    post,
    path = "/organization-exports",
    tag = TAG,
    operation_id = "organization_exports_request",
    security(("session_cookie" = [])),
    request_body = OrgExportRequest,
    responses(
        (status = 202, description = "Full export of the organization queued (`organization_export.run`). The worker writes a zip with `manifest.json` (format `aphel.organization_export` v1, Aphel version, source organization, counts and SHA-256 of every entry), the configuration package, one JSON Lines file per record type (including custom objects, with custom fields by stable key), relationships, notes, history and the attached files (`include_files`, default true). Finishes with `organization_export.completed` or `organization_export.failed` and notifies the requester. Requires `organization.export`", body = Data<OrgExportResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `organization.export` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "Another export of the organization is queued or running (`organization_export_in_progress`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn request_org_export(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<OrgExportRequest>,
) -> Result<(StatusCode, Json<Data<OrgExportResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let export = service::request(
        &ctx,
        RequestOrgExportCommand {
            include_files: request.include_files.unwrap_or(true),
            ip: client.0.ip,
        },
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(export.into()))))
}

#[utoipa::path(
    get,
    path = "/organization-exports",
    tag = TAG,
    operation_id = "organization_exports_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Full exports of the organization, most recent first. Requires `organization.export`", body = Page<OrgExportResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `organization.export` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_org_exports(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<OrgExportResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list(&ctx, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/organization-exports/{export_id}",
    tag = TAG,
    operation_id = "organization_exports_get",
    security(("session_cookie" = [])),
    params(("export_id" = String, Path, description = "Organization export id")),
    responses(
        (status = 200, description = "Status and counts of the export. Requires `organization.export`", body = Data<OrgExportResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `organization.export` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Export not found (`organization_export_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_org_export(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(export_id): Path<String>,
) -> Result<Json<Data<OrgExportResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let export = service::get(&ctx, &export_id).await?;
    Ok(Json(Data::new(export.into())))
}

#[utoipa::path(
    get,
    path = "/organization-exports/{export_id}/download",
    tag = TAG,
    operation_id = "organization_exports_download",
    security(("session_cookie" = [])),
    params(("export_id" = String, Path, description = "Organization export id")),
    responses(
        (status = 200, description = "The zip of a completed export, served as an attachment. Requires `organization.export`", content_type = "application/zip"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `organization.export` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Export not found (`organization_export_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The export is not completed or its file was removed (`organization_export_not_ready`)", body = ErrorEnvelope),
    )
)]
async fn download_org_export(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(export_id): Path<String>,
) -> Result<Response, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let download = service::download(&ctx, state.storage.as_ref(), &export_id).await?;
    let disposition = HeaderValue::from_str(&content_disposition(&download.file_name))
        .map_err(|_| AppError::Internal("invalid content disposition".to_owned()))?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(header::CONTENT_LENGTH, download.size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from_stream(download.content))
        .map_err(|error| AppError::Internal(format!("cannot build download response: {error}")))?;
    Ok(response)
}

#[utoipa::path(
    delete,
    path = "/organization-exports/{export_id}",
    tag = TAG,
    operation_id = "organization_exports_delete",
    security(("session_cookie" = [])),
    params(("export_id" = String, Path, description = "Organization export id")),
    responses(
        (status = 204, description = "Export and its zip file deleted; publishes `organization_export.deleted`. Requires `organization.export`"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `organization.export` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Export not found (`organization_export_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The export is still queued or running (`organization_export_in_progress`)", body = ErrorEnvelope),
    )
)]
async fn delete_org_export(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(export_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::delete(&ctx, state.storage.as_ref(), &export_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}
