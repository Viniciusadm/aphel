use axum::Json;
use axum::extract::{Path, State};
use serde_json::Value;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::application::{ApplyCommand, ConfigPackageUseCases};
use crate::dto::{
    ApplyPackageRequest, ApplyPackageResponse, ConfigPackageDocument,
    ConfigPackageInstallDetailResponse, ConfigPackageInstallResponse, ConfigSectionsResponse,
    ExportPackageRequest, PackagePlanResponse, PackageTemplateResponse, PlanPackageRequest,
    package_source,
};
use crate::infrastructure::MySqlConfigPackagePorts;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub const TAG: &str = "config-packages";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_sections))
        .routes(routes!(export_package))
        .routes(routes!(list_templates))
        .routes(routes!(get_template))
        .routes(routes!(plan_package))
        .routes(routes!(apply_package))
        .routes(routes!(list_installs))
        .routes(routes!(get_install))
}

pub async fn config_package_use_cases(
    state: &AppState,
) -> Result<ConfigPackageUseCases<MySqlConfigPackagePorts>, AppError> {
    Ok(ConfigPackageUseCases::new(MySqlConfigPackagePorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/config-packages/sections",
    tag = TAG,
    operation_id = "config_packages_sections",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Sections a configuration package can carry, in installation order", body = Data<ConfigSectionsResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_sections(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<ConfigSectionsResponse>>, ApiError> {
    let sections = config_package_use_cases(&state)
        .await?
        .section_keys(&tenant.scope)?;
    Ok(Json(Data::new(ConfigSectionsResponse { sections })))
}

#[utoipa::path(
    post,
    path = "/config-packages/export",
    tag = TAG,
    operation_id = "config_packages_export",
    security(("session_cookie" = [])),
    request_body = ExportPackageRequest,
    responses(
        (status = 200, description = "`aphel-config` package with the chosen sections (all when `sections` is omitted) of the organization; items are referenced by stable keys, never by id", body = Data<ConfigPackageDocument>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown section, invalid name or locale", body = ErrorEnvelope),
    )
)]
async fn export_package(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<ExportPackageRequest>,
) -> Result<Json<Data<ConfigPackageDocument>>, ApiError> {
    let package = config_package_use_cases(&state)
        .await?
        .export(&tenant.scope, request.into(), "pt-BR")
        .await?;
    Ok(Json(Data::new(package.into())))
}

#[utoipa::path(
    get,
    path = "/config-packages/templates",
    tag = TAG,
    operation_id = "config_packages_templates",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Business templates embedded in the binary", body = Data<Vec<PackageTemplateResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_templates(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<PackageTemplateResponse>>>, ApiError> {
    let templates = config_package_use_cases(&state)
        .await?
        .templates(&tenant.scope)
        .await?;
    Ok(Json(Data::new(
        templates.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/config-packages/templates/{key}",
    tag = TAG,
    operation_id = "config_packages_template",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Template key, such as `b2b`")),
    responses(
        (status = 200, description = "The template package", body = Data<ConfigPackageDocument>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template (`template_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    Ok(Json(Data::new(
        config_package_use_cases(&state)
            .await?
            .template(&tenant.scope, &key)
            .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/config-packages/plan",
    tag = TAG,
    operation_id = "config_packages_plan",
    security(("session_cookie" = [])),
    request_body = PlanPackageRequest,
    responses(
        (status = 200, description = "Validation of the package (`valid`, `issues` with paths such as `sections.fields.0.type`) and, when valid, the installation plan: one item per field, pipeline, stage, lost reason, activity type, tag, layout, view and dashboard with `create`, `update`, `unchanged` or `skip_conflict`, the changes and the `fingerprint` to send to apply. Sections that belong to a disabled module are not validated, planned or applied; they are listed in `skipped_sections`. References to fields of a disabled module (`<record_type>.<field>`, e.g. a layout field, a view column or sort, a stage required field) are removed before validating and listed in `skipped_fields`; a view whose filter uses one is left out. Nothing is written", body = Data<PackagePlanResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template (`template_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Neither or both of `package` and `template`", body = ErrorEnvelope),
    )
)]
async fn plan_package(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<PlanPackageRequest>,
) -> Result<Json<Data<PackagePlanResponse>>, ApiError> {
    let source = package_source(request.package, request.template)?;
    let plan = config_package_use_cases(&state)
        .await?
        .plan(&tenant.scope, source)
        .await?;
    Ok(Json(Data::new(plan.into())))
}

#[utoipa::path(
    post,
    path = "/config-packages/apply",
    tag = TAG,
    operation_id = "config_packages_apply",
    security(("session_cookie" = [])),
    request_body = ApplyPackageRequest,
    responses(
        (status = 200, description = "The plan was applied in one transaction: creates and updates run, conflicts are skipped unless `decisions` marks a resolvable one as `apply`. Nothing is ever deleted and no business record is touched. Publishes `config_package.applied` with audit and stores the installation in the history", body = Data<ApplyPackageResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template (`template_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The organization changed after the plan (`plan_outdated`): plan again", body = ErrorEnvelope),
        (status = 422, description = "Invalid package (`invalid_package` with the issues in `details.fields`) or decisions (`invalid_decision`)", body = ErrorEnvelope),
    )
)]
async fn apply_package(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ApplyPackageRequest>,
) -> Result<Json<Data<ApplyPackageResponse>>, ApiError> {
    let source = package_source(request.package, request.template)?;
    let result = config_package_use_cases(&state)
        .await?
        .apply(
            &tenant.scope,
            ApplyCommand {
                source,
                decisions: request
                    .decisions
                    .into_iter()
                    .map(|(item, decision)| (item, decision.into()))
                    .collect(),
                fingerprint: request.fingerprint,
                ip: client.0.ip,
            },
        )
        .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    get,
    path = "/config-packages/installs",
    tag = TAG,
    operation_id = "config_packages_installs",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Installation history of the organization, newest first", body = Page<ConfigPackageInstallResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination", body = ErrorEnvelope),
    )
)]
async fn list_installs(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<ConfigPackageInstallResponse>>, ApiError> {
    let listing = config_package_use_cases(&state)
        .await?
        .installs(&tenant.scope, window(pagination))
        .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/config-packages/installs/{id}",
    tag = TAG,
    operation_id = "config_packages_install",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Installation id")),
    responses(
        (status = 200, description = "One installation with the package that was applied", body = Data<ConfigPackageInstallDetailResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown installation (`config_package_install_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_install(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<Data<ConfigPackageInstallDetailResponse>>, ApiError> {
    let install = config_package_use_cases(&state)
        .await?
        .install(&tenant.scope, &id)
        .await?;
    Ok(Json(Data::new(install.into())))
}
