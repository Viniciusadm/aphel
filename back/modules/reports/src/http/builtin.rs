use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::http::builtin_dto::{
    ConversionParams, ConversionReportResponse, PerformanceParams, PerformanceReportResponse,
};
use crate::service::builtin;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::AuthContext;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "reports-builtin";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(conversion_report))
        .routes(routes!(performance_report))
}

#[utoipa::path(
    get,
    path = "/reports/builtin/conversion",
    tag = TAG,
    operation_id = "reports_builtin_conversion",
    security(("session_cookie" = [])),
    description = "Conversion of a pipeline (default: the default active pipeline) for the deals created in the period, under the read scope of `deals.read` (`own` sees only their deals) and the field permissions of the role, in the time zone of the organization. `summary`: created, open, won and lost deals of the cohort, `conversion_rate` (won / created), `win_rate` (won / (won + lost)) and `average_cycle_seconds` (creation to closing of the won ones). `stages` (open stages in order): `reached` counts the deals that got to the stage or beyond (won deals reached every stage), `advanced` those that got to the next stage (the last one advances to won) and `passage_rate` = advanced / reached; `lost` counts the lost deals whose last open stage was this one (deals without stage history fall into `unattributed_lost`). Times come from the stage history (`deal_stage_history`, fed by `deal.created`/`deal.stage_changed` and backfilled from the event log): `average_seconds` over the visits that entered the stage in the period and already left it, `open_visits`/`open_average_seconds` for the deals still there. `lost_reasons` groups the lost deals of the cohort by reason. With `compare_to_previous=true`, `previous` is the same snapshot for the period right before. Amounts are decimal strings (null when the role cannot see `amount`).",
    params(ConversionParams),
    responses(
        (status = 200, description = "Conversion report; `pipeline`, `current` and `previous` are null when the organization has no active pipeline", body = Data<ConversionReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown pipeline (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pipeline id or period (`validation_error` with `details.fields`), or a field used by the report is hidden for the role (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn conversion_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<ConversionParams>, QueryRejection>,
) -> Result<Json<Data<ConversionReportResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let report = builtin::conversion(&ctx, params.into()).await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    get,
    path = "/reports/builtin/performance",
    tag = TAG,
    operation_id = "reports_builtin_performance",
    security(("session_cookie" = [])),
    description = "Sales performance grouped by `dimension` (`owner`, default; `team` of the owner, where an owner in two teams counts in both; `source`; `pipeline`), optionally limited to one pipeline, under the read scope of `deals.read` and `tasks.read` (`own` sees only their numbers) and the field permissions of the role. Per row and in `totals`: `created` (deals created in the period), `won`/`won_amount` and `lost` (closed in the period), `conversion_rate` (won / (won + lost)), `average_cycle_seconds` (creation to closing of the deals won in the period) and `activities_done` (tasks completed in the period, only for `owner`/`team` and with `tasks.read`; with `pipeline_id`, only tasks linked to deals of that pipeline). `key` is the user, team, source or pipeline id (null groups the deals without one) and `label` its name. Totals are computed apart, never by summing the rows. With `compare_to_previous=true`, every row and the totals carry `previous` for the period right before.",
    params(PerformanceParams),
    responses(
        (status = 200, description = "Performance report", body = Data<PerformanceReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown pipeline (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid dimension, pipeline id or period (`validation_error` with `details.fields`), or a field used by the report is hidden for the role (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn performance_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<PerformanceParams>, QueryRejection>,
) -> Result<Json<Data<PerformanceReportResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let report = builtin::performance(&ctx, params.into()).await?;
    Ok(Json(Data::new(report.into())))
}
