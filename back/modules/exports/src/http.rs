use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{ExportJobResponse, ExportRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "exports";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(request_export, list_exports))
        .routes(routes!(get_export))
}

#[utoipa::path(
    post,
    path = "/exports",
    tag = TAG,
    operation_id = "exports_request",
    security(("session_cookie" = [])),
    request_body = ExportRequest,
    responses(
        (status = 202, description = "Export queued (`export.run`). The worker reads the records with the same filter, sort, search and own/team scope of the listing, writes a CSV (UTF-8 with BOM) or XLSX file with the chosen columns (all active fields when empty; `id` is also accepted), and finishes with `export.completed` (the file is downloaded with `GET /files/{file_id}`) or `export.failed`. Values that would run as spreadsheet formulas are prefixed with `'`", body = Data<ExportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.export` or `<type>.read` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Unsupported record type (`export_not_supported`), invalid filter, sort or columns, or more than 100 000 records (`export_too_large`)", body = ErrorEnvelope),
    )
)]
async fn request_export(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ExportRequest>,
) -> Result<(StatusCode, Json<Data<ExportJobResponse>>), ApiError> {
    let command = request.into_command(client.0.ip)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let job = service::request(&ctx, command).await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(job.into()))))
}

#[utoipa::path(
    get,
    path = "/exports",
    tag = TAG,
    operation_id = "exports_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Exports requested by the user, most recent first", body = Page<ExportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_exports(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<ExportJobResponse>>, ApiError> {
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
    path = "/exports/{export_id}",
    tag = TAG,
    operation_id = "exports_get",
    security(("session_cookie" = [])),
    params(("export_id" = String, Path, description = "Export id")),
    responses(
        (status = 200, description = "Export status; `file_id` is set when it is completed", body = Data<ExportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Export not found or requested by someone else (`export_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_export(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(export_id): Path<String>,
) -> Result<Json<Data<ExportJobResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let job = service::get(&ctx, &export_id).await?;
    Ok(Json(Data::new(job.into())))
}
