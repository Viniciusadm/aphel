use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{TrashItemResponse, TrashListParams, TrashSettingsBody, TrashTypeResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "trash";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_trash))
        .routes(routes!(list_trash_types))
        .routes(routes!(get_trash_settings, update_trash_settings))
        .routes(routes!(restore_record))
        .routes(routes!(purge_record))
}

#[utoipa::path(
    get,
    path = "/trash",
    tag = TAG,
    operation_id = "trash_list",
    security(("session_cookie" = [])),
    params(TrashListParams, PaginationQuery),
    responses(
        (status = 200, description = "Deleted records of every type with a trash (soft-deleted types with a title: people, organizations, deals, products, tasks, quotes, custom objects…), most recently deleted first, with the date they will be purged. Only records the user could read (read scope of each type: own, team or organization, plus shares) are listed; types the user cannot read are left out. Records merged into another one are not listed", body = Page<TrashItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_trash(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<TrashListParams>,
    pagination: Pagination,
) -> Result<Json<Page<TrashItemResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list(&ctx, params.into(), window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/trash/types",
    tag = TAG,
    operation_id = "trash_types",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Record types that have a trash and the user can read: every registered type with soft deletion and a title (custom objects included)", body = Data<Vec<TrashTypeResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_trash_types(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<TrashTypeResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let types = service::types(&ctx)?;
    Ok(Json(Data::new(types.iter().map(Into::into).collect())))
}

#[utoipa::path(
    get,
    path = "/trash/settings",
    tag = TAG,
    operation_id = "trash_get_settings",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Days a deleted record stays in the trash before the `trash.purge` job deletes it for good (default 30)", body = Data<TrashSettingsBody>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn get_trash_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<TrashSettingsBody>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let settings = service::settings(&ctx).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    put,
    path = "/trash/settings",
    tag = TAG,
    operation_id = "trash_update_settings",
    security(("session_cookie" = [])),
    request_body = TrashSettingsBody,
    responses(
        (status = 200, description = "Retention replaced (`trash.settings_updated`)", body = Data<TrashSettingsBody>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Retention outside 1–365 days (`validation_error` on `retention_days`)", body = ErrorEnvelope),
    )
)]
async fn update_trash_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<TrashSettingsBody>,
) -> Result<Json<Data<TrashSettingsBody>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let settings = service::update_settings(&ctx, request.retention_days, client.0.ip).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    post,
    path = "/trash/{record_type}/{record_id}/restore",
    tag = TAG,
    operation_id = "trash_restore",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type with a trash (`contact`, `company`, `deal`, `product`, `custom.<key>`…)"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Record restored with its links and custom fields through the record pipeline (validation hooks such as validation rules and approvals, `RecordRestored`, `RecordSaved`, `RecordCommitted`) and `<type>.restored`", body = Data<TrashItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage`, the read permission of the type or the delete permission on this record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "The record is not in the trash or not visible (`trash_item_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "An approval blocks the write (`approval_required`, `approval_pending`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type (`validation_error`) or a validation rule blocks the record (`validation_rule_failed`)", body = ErrorEnvelope),
    )
)]
async fn restore_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<TrashItemResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let item = service::restore(&ctx, &record_type, &record_id, client.0.ip).await?;
    Ok(Json(Data::new(item.into())))
}

#[utoipa::path(
    delete,
    path = "/trash/{record_type}/{record_id}",
    tag = TAG,
    operation_id = "trash_purge",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type with a trash (`contact`, `company`, `deal`, `product`, `custom.<key>`…)"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 204, description = "Record deleted for good (`<type>.purged`). The `RecordPurged` hook lets each module remove what it keeps about the record (notes, tags, activity links, duplicate candidates…); custom values, relationships, attachment links and shares are removed and record fields of other records that pointed to it are cleared"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage`, the read permission of the type or the delete permission on this record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "The record is not in the trash or not visible (`trash_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn purge_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::purge(&ctx, &record_type, &record_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}
