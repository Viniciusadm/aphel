use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    DashboardLayoutResponse, DashboardSubjectsResponse, SaveDashboardLayoutRequest,
    WidgetDataParams, WidgetDataResponse, WidgetDescriptorResponse,
};
use crate::service;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody, Pagination, PaginationQuery};
use aphel_kernel::api::state::AppState;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "dashboard";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_widgets))
        .routes(routes!(widget_data))
        .routes(routes!(list_subjects))
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
        (status = 200, description = "Data of a widget, computed under the read scope of the user and the time zone of the organization. `period` defaults to `month` and is ignored by widgets with `uses_period = false`; `settings` is a JSON object validated by the widget. Shapes of `data`: `sales_indicators` → currency, open_count, pipeline_amount, forecast_amount (amount × probability of the deal or of its stage), won_count, won_amount, lost_count, conversion_rate (won / (won + lost) in the period, null without closed deals); `pending_center` (settings stale_days, only_mine, limit) → overdue, due_today and due_tomorrow (`{ count, items }` of open activities) and stale_deals (`{ count, stale_days, items }`, null without `deals.read`); `recent_activity` (settings limit) → items of the latest events of records the user can read; `report_totals` → contacts_total, open_deals, won_deals, lost_deals, pending_tasks, completed_activities (null when the permission is missing); `deals_by_stage` (settings pipeline_id) → pipeline, currency and stages with count and amount. Every monetary value (`*_amount`, `amount`) is a decimal string with two places next to its `currency`. `user_id` or `team_id` (one of them, from `GET /dashboard/subjects`) limits every widget to the records owned by that user or by the members of that team, on top of the read scope; with `compare_to_previous=true`, widgets that use the period also compute the period right before (`previous_period`) and return it in `data.previous` (`sales_indicators`: won_count, won_amount, lost_count, conversion_rate; `report_totals`: won_deals, lost_deals, completed_activities)", body = Data<WidgetDataResponse>),
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
    description = "Users and teams whose numbers the current user can follow on the dashboard, from the broadest read scope among `deals.read`, `activities.read` and `people.read`: `own` → only the user; `team` → the teams of the user and their members; `organization` → every member and team. Use the ids as `user_id`/`team_id` of the widget data.",
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
