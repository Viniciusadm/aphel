use axum::Json;
use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::http::custom_dto::{
    CreateReportRequest, CreateScheduleRequest, ExportReportParams, ListReportsParams,
    ReportDeliveryResponse, ReportResponse, ReportScheduleResponse, RunReportRequest,
    UpdateReportRequest, UpdateScheduleRequest,
};
use crate::service::{reports, schedules};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::aggregates::AggregateResultResponse;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::files::content_disposition;
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "reports";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_reports, create_report))
        .routes(routes!(get_report, update_report, delete_report))
        .routes(routes!(run_report))
        .routes(routes!(export_report))
        .routes(routes!(list_schedules, create_schedule))
        .routes(routes!(update_schedule, delete_schedule))
        .routes(routes!(list_deliveries))
}

fn paginate<T>(items: Vec<T>, pagination: Pagination) -> (Vec<T>, u64) {
    let window = window(pagination);
    let total = items.len() as u64;
    let items = items
        .into_iter()
        .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
        .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
        .collect();
    (items, total)
}

#[utoipa::path(
    get,
    path = "/reports",
    tag = TAG,
    operation_id = "reports_list",
    security(("session_cookie" = [])),
    params(ListReportsParams, PaginationQuery),
    responses(
        (status = 200, description = "Saved reports visible to the user (their own, the organization ones and those of their teams) over record types the user can read, ordered by name", body = Page<ReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the requested record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid query or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_reports(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<ListReportsParams>, QueryRejection>,
) -> Result<Json<Page<ReportResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let reports = reports::list_reports(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.record_type.as_deref(),
    )
    .await?;
    let (items, total) = paginate(reports, pagination);
    Ok(Json(Page::new(
        items.into_iter().map(Into::into).collect(),
        total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/reports",
    tag = TAG,
    operation_id = "reports_create",
    security(("session_cookie" = [])),
    description = "Saves a custom report. `definition` is the same body of `POST /aggregates` (record type, filter, groups, metrics, period, comparison, sort and limit) and is validated by running it under the permissions of the caller, so unknown or hidden fields answer 422 with paths under `definition.`. Visibility follows the saved views: `organization` needs `settings.manage` and `team` needs membership of the team (or `settings.manage`).",
    request_body = CreateReportRequest,
    responses(
        (status = 201, description = "Report created", body = Data<ReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type, or sharing with an audience the user cannot reach (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, visibility, team, visualization, settings or definition (`validation_error`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn create_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateReportRequest>,
) -> Result<(StatusCode, Json<Data<ReportResponse>>), ApiError> {
    let report = reports::create_report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(report.into()))))
}

#[utoipa::path(
    get,
    path = "/reports/{report_id}",
    tag = TAG,
    operation_id = "reports_get",
    security(("session_cookie" = [])),
    params(("report_id" = String, Path, description = "Report id")),
    responses(
        (status = 200, description = "The saved report", body = Data<ReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(report_id): Path<String>,
) -> Result<Json<Data<ReportResponse>>, ApiError> {
    let report =
        reports::get_report(&Ctx::from_state(&state, &tenant.scope).await?, &report_id).await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    patch,
    path = "/reports/{report_id}",
    tag = TAG,
    operation_id = "reports_update",
    security(("session_cookie" = [])),
    params(
        ("report_id" = String, Path, description = "Report id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the report as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateReportRequest,
    responses(
        (status = 200, description = "Report updated; omitted properties stay unchanged and a new `definition` is validated like on creation", body = Data<ReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only the owner or someone with `settings.manage` can change the report (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The report changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties or definition (`validation_error`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn update_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(report_id): Path<String>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateReportRequest>,
) -> Result<Json<Data<ReportResponse>>, ApiError> {
    let report = reports::update_report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        request.into_command(expected_updated_at, client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    delete,
    path = "/reports/{report_id}",
    tag = TAG,
    operation_id = "reports_delete",
    security(("session_cookie" = [])),
    params(("report_id" = String, Path, description = "Report id")),
    responses(
        (status = 204, description = "Report deleted together with its schedules"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only the owner or someone with `settings.manage` can delete the report (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(report_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    reports::delete_report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/reports/{report_id}/run",
    tag = TAG,
    operation_id = "reports_run",
    security(("session_cookie" = [])),
    description = "Runs the saved report under the read scope and field permissions of the caller, exactly like `POST /aggregates`. `preset` (or `from`/`to`, which imply `custom`) replaces the period of the report on its own period field; `compare_to_previous` replaces the comparison flag.",
    params(("report_id" = String, Path, description = "Report id")),
    request_body(content = RunReportRequest, description = "Period and comparison overrides; send `{}` to run the report as saved"),
    responses(
        (status = 200, description = "Aggregated rows, totals and the resolved periods", body = Data<AggregateResultResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid override, a period override on a report without period (`report_period_required`), or a field hidden for the caller (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn run_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(report_id): Path<String>,
    JsonBody(request): JsonBody<RunReportRequest>,
) -> Result<Json<Data<AggregateResultResponse>>, ApiError> {
    let (_, result) = reports::run_report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        request.into(),
    )
    .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    get,
    path = "/reports/{report_id}/export",
    tag = TAG,
    operation_id = "reports_export",
    security(("session_cookie" = [])),
    description = "Runs the saved report like `POST /reports/{report_id}/run` and returns the result as a CSV (`;` separated, UTF-8 with BOM) or XLSX file: one column per group, then each metric (with previous value and variation when comparing) and a total row.",
    params(("report_id" = String, Path, description = "Report id"), ExportReportParams),
    responses(
        (status = 200, description = "The report result as an attachment", content_type = "application/octet-stream"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown format or invalid override (`validation_error`, `report_period_required`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn export_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(report_id): Path<String>,
    query: Result<Query<ExportReportParams>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let export = reports::export_report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        &params.format,
        params.overrides(),
    )
    .await?;
    let disposition = HeaderValue::from_str(&content_disposition(&export.file_name))
        .map_err(|_| AppError::Internal("invalid content disposition".to_owned()))?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, export.mime_type)
        .header(header::CONTENT_LENGTH, export.content.len())
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(export.content))
        .map_err(|error| AppError::Internal(format!("cannot build export response: {error}")))?;
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/reports/{report_id}/schedules",
    tag = TAG,
    operation_id = "reports_schedules_list",
    security(("session_cookie" = [])),
    params(("report_id" = String, Path, description = "Report id"), PaginationQuery),
    responses(
        (status = 200, description = "E-mail schedules of the report", body = Page<ReportScheduleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only those who can edit the report see its schedules (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_schedules(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(report_id): Path<String>,
    pagination: Pagination,
) -> Result<Json<Page<ReportScheduleResponse>>, ApiError> {
    let schedules =
        schedules::list_schedules(&Ctx::from_state(&state, &tenant.scope).await?, &report_id)
            .await?;
    let (items, total) = paginate(schedules, pagination);
    Ok(Json(Page::new(
        items.into_iter().map(Into::into).collect(),
        total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/reports/{report_id}/schedules",
    tag = TAG,
    operation_id = "reports_schedules_create",
    security(("session_cookie" = [])),
    description = "Schedules the report to be e-mailed `daily`, `weekly` (`weekday` 1 = Monday … 7 = Sunday) or `monthly` (`day_of_month`, the last day when the month is shorter) at `hour`:`minute` in the time zone of the organization. Every recipient must be an active member who can see the report; each one receives the result computed with their own permissions. `period_preset` replaces the period of the report on each delivery.",
    params(("report_id" = String, Path, description = "Report id")),
    request_body = CreateScheduleRequest,
    responses(
        (status = 201, description = "Schedule created with its next run", body = Data<ReportScheduleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only the owner of the report or someone with `settings.manage` can schedule it (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The report does not exist or is not visible to the user (`report_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid frequency, time, period or recipients (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_schedule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(report_id): Path<String>,
    JsonBody(request): JsonBody<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<Data<ReportScheduleResponse>>), ApiError> {
    let schedule = schedules::create_schedule(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(schedule.into()))))
}

#[utoipa::path(
    patch,
    path = "/reports/{report_id}/schedules/{schedule_id}",
    tag = TAG,
    operation_id = "reports_schedules_update",
    security(("session_cookie" = [])),
    params(
        ("report_id" = String, Path, description = "Report id"),
        ("schedule_id" = String, Path, description = "Schedule id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the schedule as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateScheduleRequest,
    responses(
        (status = 200, description = "Schedule updated; changing the timing or reactivating it recomputes the next run", body = Data<ReportScheduleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only the owner of the report or someone with `settings.manage` can change it (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown report or schedule (`report_not_found`, `report_schedule_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The schedule changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid frequency, time, period or recipients (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_schedule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((report_id, schedule_id)): Path<(String, String)>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateScheduleRequest>,
) -> Result<Json<Data<ReportScheduleResponse>>, ApiError> {
    let schedule = schedules::update_schedule(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        &schedule_id,
        request.into_command(expected_updated_at, client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(schedule.into())))
}

#[utoipa::path(
    delete,
    path = "/reports/{report_id}/schedules/{schedule_id}",
    tag = TAG,
    operation_id = "reports_schedules_delete",
    security(("session_cookie" = [])),
    params(
        ("report_id" = String, Path, description = "Report id"),
        ("schedule_id" = String, Path, description = "Schedule id"),
    ),
    responses(
        (status = 204, description = "Schedule deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only the owner of the report or someone with `settings.manage` can delete it (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown report or schedule (`report_not_found`, `report_schedule_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_schedule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((report_id, schedule_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    schedules::delete_schedule(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        &schedule_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/reports/{report_id}/schedules/{schedule_id}/deliveries",
    tag = TAG,
    operation_id = "reports_schedules_deliveries",
    security(("session_cookie" = [])),
    params(
        ("report_id" = String, Path, description = "Report id"),
        ("schedule_id" = String, Path, description = "Schedule id"),
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Deliveries of the schedule, most recent first; `failed` ones carry the reason (recipient left the organization, lost access to the report or the record type, e-mail error)", body = Page<ReportDeliveryResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Only those who can edit the report see its deliveries (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown report or schedule (`report_not_found`, `report_schedule_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_deliveries(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((report_id, schedule_id)): Path<(String, String)>,
    pagination: Pagination,
) -> Result<Json<Page<ReportDeliveryResponse>>, ApiError> {
    let listing = schedules::list_deliveries(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &report_id,
        &schedule_id,
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}
