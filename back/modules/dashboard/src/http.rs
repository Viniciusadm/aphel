use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    ActivityPageResponse, ActivityParams, DashboardLayoutResponse, DashboardSubjectsResponse,
    SaveDashboardLayoutRequest, WidgetDataParams, WidgetDataResponse, WidgetDescriptorResponse,
};
use crate::service;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, JsonBody, Pagination, PaginationQuery};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "dashboard";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_widgets))
        .routes(routes!(widget_data))
        .routes(routes!(list_subjects))
        .routes(routes!(activity_feed))
        .routes(routes!(get_layout, save_layout, reset_layout))
}

#[utoipa::path(
    get,
    path = "/dashboard/widgets",
    tag = TAG,
    operation_id = "dashboard_widgets_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Widgets the current user can add to the dashboard (only those whose permissions the user holds)", body = Page<WidgetDescriptorResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_widgets(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<WidgetDescriptorResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let widgets = service::catalog(&ctx);
    let total = widgets.len() as u64;
    let items = widgets
        .into_iter()
        .skip(usize::try_from(pagination.offset()).unwrap_or(usize::MAX))
        .take(usize::try_from(pagination.limit()).unwrap_or(usize::MAX))
        .map(Into::into)
        .collect();
    Ok(Json(Page::new(items, total, pagination)))
}

#[utoipa::path(
    get,
    path = "/dashboard/widgets/{key}/data",
    tag = TAG,
    operation_id = "dashboard_widget_data",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Widget key"), WidgetDataParams),
    responses(
        (status = 200, description = "Widget data under the caller read scope, field policy, selected user/team and organization timezone. Providers own their payloads and validate their settings. Collection widgets return namespaced source sections; recent events merge registered entity scopes and owner-provided visibility rules. Period-aware providers can return previous-period values", body = Data<WidgetDataResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the permissions of the widget, or a user or team the caller cannot see (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown widget (`widget_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid period or settings (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn widget_data(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
    query: Result<Query<WidgetDataParams>, QueryRejection>,
) -> Result<Json<Data<WidgetDataResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let data = service::widget_data(&ctx, &key, params.into_query()?).await?;
    Ok(Json(Data::new(data.into())))
}

#[utoipa::path(
    get,
    path = "/dashboard/layout",
    tag = TAG,
    operation_id = "dashboard_layout_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Dashboard layout of the current user in the organization; without a saved layout, the default one (`is_default = true`). Widgets the user can no longer see are left out", body = Data<DashboardLayoutResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn get_layout(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<DashboardLayoutResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let layout = service::layout(&ctx).await?;
    Ok(Json(Data::new(layout.into())))
}

#[utoipa::path(
    put,
    path = "/dashboard/layout",
    tag = TAG,
    operation_id = "dashboard_layout_save",
    security(("session_cookie" = [])),
    request_body = SaveDashboardLayoutRequest,
    responses(
        (status = 200, description = "Saved layout (widgets in display order, with size and settings, the selected period, the optional `subject` user or team and `compare_to_previous`). A personal preference: no domain event or audit entry", body = Data<DashboardLayoutResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or unavailable widget, repeated id, invalid size, settings, period or a subject the user cannot see (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn save_layout(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<SaveDashboardLayoutRequest>,
) -> Result<Json<Data<DashboardLayoutResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let layout = service::save_layout(&ctx, request.into_layout()).await?;
    Ok(Json(Data::new(layout.into())))
}

#[utoipa::path(
    delete,
    path = "/dashboard/layout",
    tag = TAG,
    operation_id = "dashboard_layout_reset",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Removes the saved layout and returns the default one", body = Data<DashboardLayoutResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn reset_layout(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<DashboardLayoutResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let layout = service::reset_layout(&ctx).await?;
    Ok(Json(Data::new(layout.into())))
}

#[utoipa::path(
    get,
    path = "/dashboard/subjects",
    tag = TAG,
    operation_id = "dashboard_subjects_list",
    security(("session_cookie" = [])),
    description = "Users and teams whose numbers the current user can follow on the dashboard, from the broadest read scope among registered entities: `own` → only the user; `team` → the teams of the user and their members; `organization` → every member and team. Use the ids as `user_id`/`team_id` of the widget data.",
    responses(
        (status = 200, description = "Selectable users and teams", body = Data<DashboardSubjectsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_subjects(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<DashboardSubjectsResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let subjects = service::subjects(&ctx).await?;
    Ok(Json(Data::new(subjects.into())))
}

#[utoipa::path(
    get,
    path = "/dashboard/activity",
    tag = TAG,
    operation_id = "dashboard_activity",
    security(("session_cookie" = [])),
    params(ActivityParams),
    responses(
        (status = 200, description = "Recent actions on the records the caller follows or owns, newest first, under the caller read scope and field policy. `days` limits the period, `record_type` and `actor_id` filter. Answers `{ data, meta: { next_cursor } }`: pass `meta.next_cursor` back as `cursor` for the next page (null on the last one). Empty while no module provides the follow scope", body = ActivityPageResponse),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid cursor, period or page size (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn activity_feed(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<ActivityParams>, QueryRejection>,
) -> Result<Json<ActivityPageResponse>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let page = crate::activity::activity(&ctx, params.into()).await?;
    Ok(Json(page.into()))
}
