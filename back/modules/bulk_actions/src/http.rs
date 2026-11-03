use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    BulkExportRequest, BulkItemResponse, BulkItemsParams, BulkOperationResponse, StartBulkRequest,
};
use crate::{BulkStatus, service};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use exports::dto::ExportJobResponse;

pub(crate) const TAG: &str = "bulk";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(start_bulk_operation, list_bulk_operations))
        .routes(routes!(export_bulk_selection))
        .routes(routes!(get_bulk_operation))
        .routes(routes!(list_bulk_items))
}

#[utoipa::path(
    post,
    path = "/bulk-operations",
    tag = TAG,
    operation_id = "bulk_operations_start",
    security(("session_cookie" = [])),
    request_body = StartBulkRequest,
    responses(
        (status = 200, description = "Up to 200 records: applied right away and returned `completed`. Each record goes through the same use case of the single action (validation, permission, event and audit); records the user cannot change are `skipped` and listed in `/bulk-operations/{id}/items`", body = Data<BulkOperationResponse>),
        (status = 202, description = "More than 200 records: queued as `bulk.run` (`background: true`). Poll `GET /bulk-operations/{id}` for progress; it finishes with `bulk_operation.completed` or `bulk_operation.failed` (`Actor::System`, `requested_by` in the payload)", body = Data<BulkOperationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.read` or the permission the action needs (`<type>.update`, `<type>.archive`, `<type>.delete` or, for `enroll_in_sequence` on people, `sequences.enroll`) (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Unsupported record type (`bulk_not_supported`), invalid action, empty selection (`bulk_empty_selection`), invalid filter or more than 10 000 records (`bulk_too_large`); other field errors use `validation_error`", body = ErrorEnvelope),
    )
)]
async fn start_bulk_operation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<StartBulkRequest>,
) -> Result<(StatusCode, Json<Data<BulkOperationResponse>>), ApiError> {
    let command = request.into_command(client.0.ip)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let operation = service::start(&ctx, command).await?;
    let status = if operation.status == BulkStatus::Queued {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(Data::new(operation.into()))))
}

#[utoipa::path(
    get,
    path = "/bulk-operations",
    tag = TAG,
    operation_id = "bulk_operations_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Bulk operations requested by the user, most recent first", body = Page<BulkOperationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_bulk_operations(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<BulkOperationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list(&ctx, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/bulk-operations/export",
    tag = TAG,
    operation_id = "bulk_operations_export",
    security(("session_cookie" = [])),
    request_body = BulkExportRequest,
    responses(
        (status = 202, description = "Export of the selection queued through the exports module (`export.run`); selected ids are limited to 100, larger selections use the filter. Follow the job in `GET /exports/{id}`", body = Data<ExportJobResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.export` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid selection or columns (`validation_error`, `bulk_empty_selection`, `bulk_not_supported`)", body = ErrorEnvelope),
    )
)]
async fn export_bulk_selection(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<BulkExportRequest>,
) -> Result<(StatusCode, Json<Data<ExportJobResponse>>), ApiError> {
    let command = request.into_command(client.0.ip)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let job = service::export(&ctx, command).await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(job.into()))))
}

#[utoipa::path(
    get,
    path = "/bulk-operations/{operation_id}",
    tag = TAG,
    operation_id = "bulk_operations_get",
    security(("session_cookie" = [])),
    params(("operation_id" = String, Path, description = "Bulk operation id")),
    responses(
        (status = 200, description = "Progress and totals of the operation", body = Data<BulkOperationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Operation not found or requested by someone else (`bulk_operation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_bulk_operation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(operation_id): Path<String>,
) -> Result<Json<Data<BulkOperationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let operation = service::get(&ctx, &operation_id).await?;
    Ok(Json(Data::new(operation.into())))
}

#[utoipa::path(
    get,
    path = "/bulk-operations/{operation_id}/items",
    tag = TAG,
    operation_id = "bulk_operations_items",
    security(("session_cookie" = [])),
    params(
        ("operation_id" = String, Path, description = "Bulk operation id"),
        BulkItemsParams,
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Report of the records that were skipped (validation, permission, not found, stage rules) or failed, with the error code and message", body = Page<BulkItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Operation not found (`bulk_operation_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid outcome or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_bulk_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(operation_id): Path<String>,
    Query(params): Query<BulkItemsParams>,
    pagination: Pagination,
) -> Result<Json<Page<BulkItemResponse>>, ApiError> {
    let outcome = params.outcome()?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::items(&ctx, &operation_id, outcome, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}
