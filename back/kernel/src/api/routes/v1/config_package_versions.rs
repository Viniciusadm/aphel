use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::config_package_versions::{
    ApplyVersionRequest, CompareVersionRequest, InstalledVersionResponse, VersionApplyResponse,
    VersionComparisonResponse, resolutions,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody};
use crate::api::state::AppState;
use crate::application::config_packages::versioning::{ApplyVersionCommand, ConfigVersionUseCases};
use crate::errors::AppError;
use crate::infrastructure::config_packages::versioning::MySqlConfigVersionPorts;

pub const TAG: &str = "config-package-versions";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_versions))
        .routes(routes!(compare_version))
        .routes(routes!(apply_version))
}

pub async fn version_use_cases(
    state: &AppState,
) -> Result<ConfigVersionUseCases<MySqlConfigVersionPorts>, AppError> {
    Ok(ConfigVersionUseCases::new(MySqlConfigVersionPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/config-package-versions",
    tag = TAG,
    operation_id = "config_package_versions_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Versioned packages installed in the organization (every installed version, newest first, at most 500)", body = Data<Vec<InstalledVersionResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_versions(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<InstalledVersionResponse>>>, ApiError> {
    let versions = version_use_cases(&state)
        .await?
        .installed(&tenant.scope)
        .await?;
    Ok(Json(Data::new(
        versions.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/config-package-versions/compare",
    tag = TAG,
    operation_id = "config_package_versions_compare",
    security(("session_cookie" = [])),
    request_body = CompareVersionRequest,
    responses(
        (status = 200, description = "Compares a versioned package (`id` + `version`) with the version installed in the organization: every item gets its status in the package (`added`, `changed`, `removed`, `unchanged`), whether it was changed locally since the installation and the plan against the current configuration. `conflict` marks items the package changed that were also changed locally; `needs_resolution` marks items the admin must decide (`keep_local` or `accept_package`). Nothing is written", body = Data<VersionComparisonResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Malformed body", body = ErrorEnvelope),
    )
)]
async fn compare_version(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CompareVersionRequest>,
) -> Result<Json<Data<VersionComparisonResponse>>, ApiError> {
    let comparison = version_use_cases(&state)
        .await?
        .compare(&tenant.scope, &request.package)
        .await?;
    Ok(Json(Data::new(comparison.into())))
}

#[utoipa::path(
    post,
    path = "/config-package-versions/apply",
    tag = TAG,
    operation_id = "config_package_versions_apply",
    security(("session_cookie" = [])),
    request_body = ApplyVersionRequest,
    responses(
        (status = 200, description = "Installs or updates the package in one transaction. Conflicts follow `resolutions` (default `keep_local`: the local version of the item is kept). Nothing is ever deleted and no business record is touched. Records the version with a snapshot per item, adds the installation to the history and publishes `config_package.version_applied` with audit", body = Data<VersionApplyResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The organization or the installed version changed after the comparison (`plan_outdated`): compare again", body = ErrorEnvelope),
        (status = 422, description = "Invalid package (`invalid_package`) or resolutions (`invalid_resolution`)", body = ErrorEnvelope),
    )
)]
async fn apply_version(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ApplyVersionRequest>,
) -> Result<Json<Data<VersionApplyResponse>>, ApiError> {
    let result = version_use_cases(&state)
        .await?
        .apply(
            &tenant.scope,
            ApplyVersionCommand {
                package: request.package,
                origin: None,
                checksum: None,
                resolutions: resolutions(request.resolutions),
                fingerprint: request.fingerprint,
                ip: client.0.ip,
            },
        )
        .await?;
    Ok(Json(Data::new(result.into())))
}
