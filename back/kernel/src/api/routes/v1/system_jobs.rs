use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::system_jobs::{FailedJobResponse, FailedJobsQuery, JobStatsResponse};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, Pagination, PaginationQuery};
use crate::api::state::AppState;
use crate::application::system::{FailedJobFilter, PageWindow, SystemActor, SystemAdminUseCases};
use crate::errors::AppError;
use crate::infrastructure::system::MySqlSystemAdminPorts;

pub const TAG: &str = "system";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(job_stats))
        .routes(routes!(failed_jobs))
        .routes(routes!(retry_job))
        .routes(routes!(discard_job))
}

pub fn use_cases(state: &AppState) -> Result<SystemAdminUseCases<MySqlSystemAdminPorts>, AppError> {
    Ok(SystemAdminUseCases::new(MySqlSystemAdminPorts::new(
        state.db()?,
    )))
}

pub fn actor(admin: &AuthContext, client: Option<&ClientInfo>) -> SystemActor {
    SystemActor {
        scope: admin.scope.clone(),
        user_id: admin.user().id.clone(),
        ip: client.and_then(|info| info.0.ip.clone()),
    }
}

#[utoipa::path(
    get,
    path = "/system/jobs/stats",
    tag = TAG,
    operation_id = "system_job_stats",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Background job queue statistics for the whole installation", body = Data<JobStatsResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn job_stats(
    State(state): State<AppState>,
    admin: AuthContext,
) -> Result<Json<Data<JobStatsResponse>>, ApiError> {
    let overview = use_cases(&state)?
        .queue_overview(&actor(&admin, None))
        .await?;
    Ok(Json(Data::new(overview.into())))
}

#[utoipa::path(
    get,
    path = "/system/jobs/failed",
    tag = TAG,
    operation_id = "system_failed_jobs",
    security(("session_cookie" = [])),
    params(PaginationQuery, FailedJobsQuery),
    responses(
        (status = 200, description = "Jobs that exhausted their attempts (metadata only, never the payload)", body = Page<FailedJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination", body = ErrorEnvelope),
    )
)]
async fn failed_jobs(
    State(state): State<AppState>,
    admin: AuthContext,
    pagination: Pagination,
    Query(query): Query<FailedJobsQuery>,
) -> Result<Json<Page<FailedJobResponse>>, ApiError> {
    let filter = FailedJobFilter {
        kind: query.kind.filter(|kind| !kind.trim().is_empty()),
    };
    let window = PageWindow {
        offset: pagination.offset(),
        limit: pagination.limit(),
    };
    let (jobs, total) = use_cases(&state)?
        .failed_jobs(&actor(&admin, None), &filter, window)
        .await?;
    Ok(Json(Page::new(
        jobs.into_iter().map(Into::into).collect(),
        total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/system/jobs/{id}/retry",
    tag = TAG,
    operation_id = "system_retry_job",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Failed job id")),
    responses(
        (status = 204, description = "Job put back in the queue with its attempts reset"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "No failed job with this id (`job_not_found`)", body = ErrorEnvelope),
    )
)]
async fn retry_job(
    State(state): State<AppState>,
    admin: AuthContext,
    client: ClientInfo,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?
        .retry_job(&actor(&admin, Some(&client)), &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/system/jobs/{id}",
    tag = TAG,
    operation_id = "system_discard_job",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Failed job id")),
    responses(
        (status = 204, description = "Failed job discarded"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "No failed job with this id (`job_not_found`)", body = ErrorEnvelope),
    )
)]
async fn discard_job(
    State(state): State<AppState>,
    admin: AuthContext,
    client: ClientInfo,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?
        .discard_job(&actor(&admin, Some(&client)), &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
