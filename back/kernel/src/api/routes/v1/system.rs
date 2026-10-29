use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::system::{MigrationsResponse, RecentFailuresResponse, SystemOverviewResponse};
use crate::api::error::ApiError;
use crate::api::extractors::AuthContext;
use crate::api::routes::health::readiness;
use crate::api::routes::v1::system_jobs::{TAG, actor, use_cases};
use crate::api::state::AppState;
use crate::application::system::APHEL_VERSION;
use crate::application::system::admin::FAILURE_WINDOW_DAYS;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(overview))
        .routes(routes!(migrations))
        .routes(routes!(failures))
}

#[utoipa::path(
    get,
    path = "/system/overview",
    tag = TAG,
    operation_id = "system_overview",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Version, readiness checks, workers and compiled modules", body = Data<SystemOverviewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn overview(
    State(state): State<AppState>,
    admin: AuthContext,
) -> Result<Json<Data<SystemOverviewResponse>>, ApiError> {
    let use_cases = use_cases(&state)?;
    let actor = actor(&admin, None);
    let heartbeats = use_cases.heartbeats(&actor).await?;
    let modules = use_cases.modules(&actor).await?;
    let readiness = readiness(&state).await;
    Ok(Json(Data::new(SystemOverviewResponse {
        version: APHEL_VERSION.to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        readiness: readiness.into(),
        heartbeats: heartbeats.into_iter().map(Into::into).collect(),
        modules: modules.into_iter().map(Into::into).collect(),
    })))
}

#[utoipa::path(
    get,
    path = "/system/migrations",
    tag = TAG,
    operation_id = "system_migrations",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Applied and pending migrations of the core and of the compiled modules", body = Data<MigrationsResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn migrations(
    State(state): State<AppState>,
    admin: AuthContext,
) -> Result<Json<Data<MigrationsResponse>>, ApiError> {
    let status = use_cases(&state)?.migrations(&actor(&admin, None)).await?;
    Ok(Json(Data::new(status.into())))
}

#[utoipa::path(
    get,
    path = "/system/failures",
    tag = TAG,
    operation_id = "system_recent_failures",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Recent automation, outgoing webhook and integration failures (metadata only)", body = Data<RecentFailuresResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing permission `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn failures(
    State(state): State<AppState>,
    admin: AuthContext,
) -> Result<Json<Data<RecentFailuresResponse>>, ApiError> {
    let failures = use_cases(&state)?
        .recent_failures(&actor(&admin, None))
        .await?;
    Ok(Json(Data::new(RecentFailuresResponse::new(
        failures,
        FAILURE_WINDOW_DAYS,
    ))))
}
