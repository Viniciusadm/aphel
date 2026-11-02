use axum::Json;
use axum::extract::{Path, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::application::{CatalogEntryKind, CatalogSettings, CatalogUseCases};
use crate::dto::CatalogResponse;
use crate::infrastructure::MySqlCatalogPorts;
use ::config_packages::application::versioning::ApplyVersionCommand;
use ::config_packages::versions_http::version_use_cases;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;
use config_packages::versions_dto::{
    ApplyCatalogVersionRequest, VersionApplyResponse, VersionComparisonResponse, resolutions,
};

pub const TAG: &str = "catalog";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_catalog))
        .routes(routes!(compare_catalog_template))
        .routes(routes!(apply_catalog_template))
        .routes(routes!(download_catalog_theme))
}

fn settings(state: &AppState) -> CatalogSettings {
    crate::settings::catalog_settings(&state.configuration())
}

fn catalog_use_cases(state: &AppState) -> Result<CatalogUseCases<MySqlCatalogPorts>, AppError> {
    Ok(CatalogUseCases::new(MySqlCatalogPorts::new(
        state.db()?,
        settings(state),
    )))
}

#[utoipa::path(
    get,
    path = "/catalog",
    tag = TAG,
    operation_id = "catalog_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Entries of the community catalog read from the catalog source set in Settings → Modules (templates and modules), with compatibility with this Aphel and the installed version of each template. `enabled` is false when the catalog is turned off; invalid entries are listed in `rejected`", body = Data<CatalogResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "The index could not be read or is not a valid `aphel-catalog` (`catalog_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn get_catalog(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<CatalogResponse>>, ApiError> {
    let listing = catalog_use_cases(&state)?.list(&tenant.scope).await?;
    Ok(Json(Data::new(listing.into())))
}

#[utoipa::path(
    post,
    path = "/catalog/templates/{id}/compare",
    tag = TAG,
    operation_id = "catalog_template_compare",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Catalog entry id, such as `aphel.b2b`")),
    responses(
        (status = 200, description = "Downloads the template package, checks its SHA-256 and the minimum Aphel version, and compares it with the installed version (same body as `POST /config-package-versions/compare`). Nothing is written", body = Data<VersionComparisonResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Catalog turned off (`catalog_disabled`) or unknown entry (`catalog_entry_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Module entry (`catalog_entry_not_installable`), Aphel too old (`aphel_too_old`), checksum mismatch (`checksum_mismatch`), package that does not match the entry (`catalog_package_mismatch`) or download failure (`catalog_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn compare_catalog_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<Data<VersionComparisonResponse>>, ApiError> {
    let downloaded = catalog_use_cases(&state)?
        .download_template(&tenant.scope, &id)
        .await?;
    let comparison = version_use_cases(&state)
        .await?
        .compare(&tenant.scope, &downloaded.package)
        .await?;
    Ok(Json(Data::new(comparison.into())))
}

#[utoipa::path(
    post,
    path = "/catalog/templates/{id}/apply",
    tag = TAG,
    operation_id = "catalog_template_apply",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Catalog entry id, such as `aphel.b2b`")),
    request_body = ApplyCatalogVersionRequest,
    responses(
        (status = 200, description = "Downloads and verifies the package again and installs or updates it through the versioning flow (`POST /config-package-versions/apply`), recording the package URL as the origin", body = Data<VersionApplyResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Catalog turned off (`catalog_disabled`) or unknown entry (`catalog_entry_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The organization or the installed version changed after the comparison (`plan_outdated`)", body = ErrorEnvelope),
        (status = 422, description = "Verification failure (see compare), invalid package (`invalid_package`) or resolutions (`invalid_resolution`)", body = ErrorEnvelope),
    )
)]
async fn apply_catalog_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(id): Path<String>,
    JsonBody(request): JsonBody<ApplyCatalogVersionRequest>,
) -> Result<Json<Data<VersionApplyResponse>>, ApiError> {
    let downloaded = catalog_use_cases(&state)?
        .download_template(&tenant.scope, &id)
        .await?;
    let result = version_use_cases(&state)
        .await?
        .apply(
            &tenant.scope,
            ApplyVersionCommand {
                package: downloaded.package,
                origin: Some(downloaded.origin),
                checksum: Some(downloaded.checksum),
                resolutions: resolutions(request.resolutions),
                fingerprint: request.fingerprint,
                ip: client.0.ip,
            },
        )
        .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    post,
    path = "/catalog/themes/{id}/download",
    tag = TAG,
    operation_id = "catalog_theme_download",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Catalog entry id of a theme, such as `aphel.slate`")),
    responses(
        (status = 200, description = "Downloads the theme package, checks its SHA-256, the minimum Aphel version and that it matches the entry, and returns it unchanged. Nothing is installed: send it to `POST /themes/validate` and `POST /themes`", body = serde_json::Value),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Catalog turned off (`catalog_disabled`) or unknown entry (`catalog_entry_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Not a theme entry (`catalog_entry_not_installable`), Aphel too old (`aphel_too_old`), checksum mismatch (`checksum_mismatch`), package that does not match the entry (`catalog_package_mismatch`) or download failure (`catalog_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn download_catalog_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let downloaded = catalog_use_cases(&state)?
        .download_package(&tenant.scope, &id, CatalogEntryKind::Theme)
        .await?;
    Ok(Json(downloaded.package))
}
