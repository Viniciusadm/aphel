use axum::Json;
use axum::extract::{Path, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::modules::{
    ModuleDataExportResponse, ModuleResponse, ModuleSetupHintResponse, PublicModuleResponse,
    PublicPlatformResponse, PurgeModuleDataRequest, TestModuleSettingsRequest,
    TestModuleSettingsResponse, UpdateModuleSettingsRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, JsonBody};
use crate::api::state::AppState;
use crate::application::access::Authorizer;
use crate::modules::runtime::module_use_cases;

pub const TAG: &str = "modules";

fn ensure_management_allowed() -> Result<(), crate::errors::AppError> {
    if crate::tenancy::plugin_management_enabled() {
        return Ok(());
    }
    Err(crate::errors::AppError::forbidden_code(
        "plugin_management_disabled",
        "module and plugin management is turned off for this organization; ask the platform operator to turn it on",
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_modules))
        .routes(routes!(get_module))
        .routes(routes!(enable_module))
        .routes(routes!(disable_module))
        .routes(routes!(update_module_settings))
        .routes(routes!(test_module_settings))
        .routes(routes!(module_setup_hints))
        .routes(routes!(export_module_data))
        .routes(routes!(purge_module_data))
}

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_public_modules))
        .routes(routes!(public_platform))
}

#[utoipa::path(
    get,
    path = "/public/modules",
    tag = TAG,
    operation_id = "modules_list_public",
    description = "Public. The key of every module compiled into this build and whether it is enabled, so the web app can leave out the screens and calls of disabled modules before anyone signs in. Nothing else about the modules is returned.",
    responses(
        (status = 200, description = "Compiled modules and their state", body = Data<Vec<PublicModuleResponse>>),
    )
)]
async fn list_public_modules(
    State(state): State<AppState>,
) -> Json<Data<Vec<PublicModuleResponse>>> {
    Json(Data::new(
        state
            .app
            .modules()
            .iter()
            .map(|module| PublicModuleResponse {
                key: module.key().to_owned(),
                enabled: state.app.is_enabled(module.key()),
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/public/platform",
    tag = TAG,
    operation_id = "platform_public",
    description = "Public. Capabilities the platform operator allows this organization, so the web app can leave out the screens that are switched off. `plugin_management` is false when the operator console disabled the plugins screen; always true outside multi-tenancy.",
    responses(
        (status = 200, description = "Allowed capabilities", body = Data<PublicPlatformResponse>),
    )
)]
async fn public_platform() -> Json<Data<PublicPlatformResponse>> {
    Json(Data::new(PublicPlatformResponse {
        plugin_management: crate::tenancy::plugin_management_enabled(),
    }))
}

#[utoipa::path(
    get,
    path = "/modules",
    tag = TAG,
    operation_id = "modules_list",
    security(("session_cookie" = [])),
    description = "Modules compiled into this Aphel build (the kernel, the business modules and the former modules) with their manifest (version, minimum Aphel version, dependencies, permissions, events and settings schema), whether they are required, their compatibility and state in this installation. Every member can list them (the web app loads the enabled ones); `settings` and `configured_secrets` are only filled for `settings.manage`. Secret settings never come back, only whether they are configured.",
    responses(
        (status = 200, description = "Compiled modules", body = Data<Vec<ModuleResponse>>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
    )
)]
async fn list_modules(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<ModuleResponse>>>, ApiError> {
    let views = module_use_cases(&state)?.list(&tenant.scope).await?;
    Ok(Json(Data::new(views.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    get,
    path = "/modules/{key}",
    tag = TAG,
    operation_id = "modules_get",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Module key, such as `proposals`")),
    responses(
        (status = 200, description = "The module and its state in this installation", body = Data<ModuleResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_module(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    let view = module_use_cases(&state)?.get(&tenant.scope, &key).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/modules/{key}/enable",
    tag = TAG,
    operation_id = "modules_enable",
    security(("session_cookie" = [])),
    description = "Enables the module: its routes, widgets, automation steps, consumers and jobs start to act and its permissions enter the role catalog. Requires every dependency enabled and the required settings configured. Applies the default permission grants of the module, records `installed_version` and publishes `module.enabled`.",
    params(("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "Enabled", body = Data<ModuleResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Incompatible module (`module_incompatible`) or a dependency is disabled (`module_dependency_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Required settings are missing (`module_settings_required` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn enable_module(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    ensure_management_allowed()?;
    let view = module_use_cases(&state)?
        .enable(&tenant.scope, &key)
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/modules/{key}/disable",
    tag = TAG,
    operation_id = "modules_disable",
    security(("session_cookie" = [])),
    description = "Disables the module: hides its screens, stops its routes (404 `module_disabled`), consumers, jobs, widgets and automation steps. Nothing is deleted; enabling again restores everything.",
    params(("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "Disabled", body = Data<ModuleResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "An enabled module depends on it (`module_required_by`) or the module is required by this build (`module_required`)", body = ErrorEnvelope),
    )
)]
async fn disable_module(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    ensure_management_allowed()?;
    let view = module_use_cases(&state)?
        .disable(&tenant.scope, &key)
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/modules/{key}/settings",
    tag = TAG,
    operation_id = "modules_update_settings",
    security(("session_cookie" = [])),
    description = "Merges settings into the configuration of the module, validated against its `settings_schema`. `null` or an empty string clears a value (for a secret, `null` removes it and an empty string keeps it). Secrets are stored encrypted and never returned.",
    params(("key" = String, Path, description = "Module key")),
    request_body = UpdateModuleSettingsRequest,
    responses(
        (status = 200, description = "Saved", body = Data<ModuleResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid settings (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_module_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<UpdateModuleSettingsRequest>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    ensure_management_allowed()?;
    let view = module_use_cases(&state)?
        .update_settings(&tenant.scope, &key, request.settings)
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/modules/{key}/test-settings",
    tag = TAG,
    operation_id = "modules_test_settings",
    security(("session_cookie" = [])),
    description = "Tries the given settings against the real service (for example one AI call) without saving them and without enabling the module. A secret left empty uses the one already stored. Only modules that declare a test (`can_test`) answer; the result is `ok` plus a message for the person.",
    params(("key" = String, Path, description = "Module key")),
    request_body = TestModuleSettingsRequest,
    responses(
        (status = 200, description = "The test ran; `ok` tells whether the service answered", body = Data<TestModuleSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module or the module has no test (`module_test_unsupported`)", body = ErrorEnvelope),
    )
)]
async fn test_module_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<TestModuleSettingsRequest>,
) -> Result<Json<Data<TestModuleSettingsResponse>>, ApiError> {
    ensure_management_allowed()?;
    tenant
        .scope
        .require(&crate::domain::access::Permission::SETTINGS_MANAGE)?;
    let tester = crate::modules::app()
        .all_contributions::<dyn crate::modules::ModuleSettingsTester>()
        .into_iter()
        .find(|(module, tester)| *module == key && tester.can_test())
        .map(|(_, tester)| tester)
        .ok_or_else(|| {
            crate::errors::AppError::not_found_code(
                "module_test_unsupported",
                "this module has no settings test",
            )
        })?;
    let ctx = crate::Ctx::from_state(&state, &tenant.scope).await?;
    let outcome = tester.test(&ctx, &request.settings).await?;
    Ok(Json(Data::new(TestModuleSettingsResponse {
        ok: outcome.ok,
        code: outcome.code,
        message: outcome.message,
    })))
}

#[utoipa::path(
    get,
    path = "/modules/{key}/setup-hints",
    tag = TAG,
    operation_id = "modules_setup_hints",
    security(("session_cookie" = [])),
    description = "Values the person needs while setting a module up, even before enabling it (for example the callback address to register at an identity provider). Only modules that declare them (`has_setup_hints`) answer.",
    params(("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "The hints of the module", body = Data<Vec<ModuleSetupHintResponse>>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module or the module has no hints (`module_setup_unsupported`)", body = ErrorEnvelope),
    )
)]
async fn module_setup_hints(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<Vec<ModuleSetupHintResponse>>>, ApiError> {
    ensure_management_allowed()?;
    tenant
        .scope
        .require(&crate::domain::access::Permission::SETTINGS_MANAGE)?;
    let source = crate::modules::app()
        .all_contributions::<dyn crate::modules::ModuleSettingsTester>()
        .into_iter()
        .find(|(module, tester)| *module == key && tester.has_hints())
        .map(|(_, tester)| tester)
        .ok_or_else(|| {
            crate::errors::AppError::not_found_code(
                "module_setup_unsupported",
                "this module has no setup hints",
            )
        })?;
    let ctx = crate::Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data::new(
        source
            .hints(&ctx)
            .await?
            .into_iter()
            .map(|hint| ModuleSetupHintResponse {
                key: hint.key.to_owned(),
                value: hint.value,
            })
            .collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/modules/{key}/data-export",
    tag = TAG,
    operation_id = "modules_export_data",
    security(("session_cookie" = [])),
    description = "Exports every row the module keeps (its `<key>_*` tables and the ones declared in `owned_tables`) plus what the module adds, as JSON. Required before removing the data.",
    params(("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "The exported data", body = Data<ModuleDataExportResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
    )
)]
async fn export_module_data(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<ModuleDataExportResponse>>, ApiError> {
    ensure_management_allowed()?;
    let export = module_use_cases(&state)?
        .export_data(&tenant.scope, &key)
        .await?;
    Ok(Json(Data::new(export.into())))
}

#[utoipa::path(
    post,
    path = "/modules/{key}/data-purge",
    tag = TAG,
    operation_id = "modules_purge_data",
    security(("session_cookie" = [])),
    description = "Permanently removes the data of a disabled module (rows of its tables, its secrets and settings). Requires `confirm` equal to the module key and an export made after the module was disabled. Publishes `module.data_purged`.",
    params(("key" = String, Path, description = "Module key")),
    request_body = PurgeModuleDataRequest,
    responses(
        (status = 200, description = "Data removed", body = Data<ModuleResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The module is enabled (`module_enabled`) or was not exported after being disabled (`module_export_required`)", body = ErrorEnvelope),
        (status = 422, description = "Missing confirmation (`module_confirmation_required`)", body = ErrorEnvelope),
    )
)]
async fn purge_module_data(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<PurgeModuleDataRequest>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    ensure_management_allowed()?;
    let view = module_use_cases(&state)?
        .purge_data(&tenant.scope, &key, &request.confirm)
        .await?;
    Ok(Json(Data::new(view.into())))
}
