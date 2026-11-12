use std::sync::Arc;

use axum::Json;
use axum::extract::multipart::MultipartRejection;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use utoipa::IntoParams;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::modules::{
    ModuleDataExportResponse, ModuleResponse, PurgeModuleDataRequest, UpdateModuleSettingsRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody};
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};
use crate::modules::runtime::module_use_cases;
use crate::modules::runtime_catalog::{
    PluginInstallRequest, PluginToggleRequest, plugin_administration,
};
use crate::state::AppState;
use crate::tenancy::console::dto::PluginPayload;
use crate::tenancy::console::session::actor;
use crate::tenancy::console::{OperatorSession, Shared};
use crate::tenancy::{Tenant, TenantRuntime, record_operator_action, scope};

const TAG: &str = "console";
const MAX_PACKAGE_UPLOAD_BYTES: usize = 51 * 1024 * 1024;

pub(super) fn router() -> OpenApiRouter<Shared> {
    OpenApiRouter::new()
        .routes(routes!(list_plugins))
        .routes(routes!(install_plugin).layer(DefaultBodyLimit::max(MAX_PACKAGE_UPLOAD_BYTES)))
        .routes(routes!(plugin_readiness))
        .routes(routes!(install_pack).layer(DefaultBodyLimit::max(MAX_PACKAGE_UPLOAD_BYTES)))
        .routes(routes!(reapply_grants))
        .routes(routes!(update_module_settings))
        .routes(routes!(export_module_data))
        .routes(routes!(purge_module_data))
        .routes(routes!(enable_plugin))
        .routes(routes!(disable_plugin))
        .routes(routes!(uninstall_plugin))
        .routes(routes!(plugin_settings, update_plugin_settings))
        .routes(routes!(list_modules))
        .routes(routes!(enable_module))
        .routes(routes!(disable_module))
}

#[derive(Debug, Default, Deserialize, IntoParams)]
struct InstallQuery {
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    acknowledge_conflicts: bool,
    #[serde(default)]
    acknowledge_web_version: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
struct ToggleQuery {
    #[serde(default)]
    acknowledge_conflicts: bool,
    #[serde(default)]
    acknowledge_web_version: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
struct UninstallQuery {
    #[serde(default)]
    purge: bool,
}

async fn open(state: &Shared, slug: &str) -> Result<(Tenant, Arc<TenantRuntime>), AppError> {
    let tenant = state.control.require_tenant(slug).await?;
    let runtime = state.runtimes.get(&Arc::new(tenant.clone())).await?;
    Ok((tenant, runtime))
}

fn tenant_state(runtime: &TenantRuntime) -> Result<AppState, AppError> {
    runtime
        .state()
        .cloned()
        .ok_or_else(|| AppError::Internal("the tenant state is not ready".to_owned()))
}

async fn audit(
    state: &Shared,
    session: &OperatorSession,
    client: &ClientInfo,
    tenant: &Tenant,
    action: &str,
    details: Value,
) {
    record_operator_action(
        state.control.pool(),
        &actor(session, client),
        action,
        Some(tenant.id),
        details,
    )
    .await;
}

#[utoipa::path(
    get,
    path = "/tenants/{slug}/plugins",
    tag = TAG,
    operation_id = "console_list_tenant_plugins",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Runtime plugins installed in the organization, in the shape of the organization API (`PluginResponse`)", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown tenant (`tenant_not_found`) or no plugin module in this build (`module_not_installed`)", body = ErrorEnvelope),
        (status = 503, description = "The organization is not available (`tenant_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn list_plugins(
    State(shared): State<Shared>,
    _session: OperatorSession,
    Path(slug): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let (_, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let body = scope(runtime, async {
        plugin_administration()?.list(&state).await
    })
    .await?;
    Ok(Json(Data::new(body)))
}

async fn read_upload(
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(Vec<u8>, Vec<String>), AppError> {
    let mut multipart = multipart
        .map_err(|_| AppError::Validation("send the package as multipart/form-data".to_owned()))?;
    let mut package = None;
    let mut presets = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("the form could not be read".to_owned()))?
    {
        match field.name() {
            Some("package") => {
                let bytes = field.bytes().await.map_err(|_| {
                    AppError::Validation("the package could not be read".to_owned())
                })?;
                package = Some(bytes.to_vec());
            }
            Some("role_presets") => {
                let text = field.text().await.map_err(|_| {
                    AppError::Validation("the role presets could not be read".to_owned())
                })?;
                presets.extend(
                    text.split(',')
                        .map(str::trim)
                        .filter(|key| !key.is_empty())
                        .map(str::to_owned),
                );
            }
            _ => {}
        }
    }
    package.map(|package| (package, presets)).ok_or_else(|| {
        AppError::invalid_fields(
            "invalid form",
            vec![FieldViolation::new("package", "is required")],
        )
    })
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/plugins",
    tag = TAG,
    operation_id = "console_install_tenant_plugin",
    description = "Installs or upgrades a plugin in the organization from an `.aphelplugin` sent as the multipart field `package`, with the same checks and acknowledgements as the organization API. With `dry_run=true` nothing is installed and the response describes what the package asks for. Works even when the organization's plugin management is switched off.",
    params(("slug" = String, Path, description = "Tenant slug"), InstallQuery),
    request_body(content_type = "multipart/form-data", description = "Field `package` with the .aphelplugin file"),
    responses(
        (status = 200, description = "Dry run: what the package declares and asks for (`PluginConsentResponse`)", body = Data<PluginPayload>),
        (status = 201, description = "Plugin installed or upgraded (`PluginResponse`)", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "Conflicts or web code not acknowledged", body = ErrorEnvelope),
        (status = 422, description = "Invalid package", body = ErrorEnvelope),
    )
)]
async fn install_plugin(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
    Query(query): Query<InstallQuery>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Response, ApiError> {
    let (package, role_presets) = read_upload(multipart).await?;
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let ip = client.0.ip.clone();
    let result = scope(runtime, async {
        plugin_administration()?
            .install(
                &state,
                &AccessScope::system(),
                ip,
                PluginInstallRequest {
                    package,
                    role_presets,
                    dry_run: query.dry_run,
                    acknowledge_conflicts: query.acknowledge_conflicts,
                    acknowledge_web_version: query.acknowledge_web_version,
                },
            )
            .await
    })
    .await?;
    if !result.installed {
        return Ok(Json(Data::new(result.body)).into_response());
    }
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.plugin_installed",
        json!({
            "slug": tenant.slug,
            "plugin": result.body.get("key"),
            "version": result.body.get("version"),
        }),
    )
    .await;
    Ok((StatusCode::CREATED, Json(Data::new(result.body))).into_response())
}

async fn toggle(
    shared: Shared,
    session: OperatorSession,
    client: ClientInfo,
    slug: String,
    key: String,
    enabled: bool,
    query: ToggleQuery,
) -> Result<Json<Data<Value>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let ip = client.0.ip.clone();
    let plugin = key.clone();
    let body = scope(runtime, async move {
        plugin_administration()?
            .set_enabled(
                &state,
                &AccessScope::system(),
                ip,
                &plugin,
                PluginToggleRequest {
                    enabled,
                    acknowledge_conflicts: query.acknowledge_conflicts,
                    acknowledge_web_version: query.acknowledge_web_version,
                },
            )
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        if enabled {
            "tenant.plugin_enabled"
        } else {
            "tenant.plugin_disabled"
        },
        json!({ "slug": tenant.slug, "plugin": key }),
    )
    .await;
    Ok(Json(Data::new(body)))
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/plugins/{key}/enable",
    tag = TAG,
    operation_id = "console_enable_tenant_plugin",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Plugin key"), ToggleQuery),
    responses(
        (status = 200, description = "Plugin enabled (`PluginResponse`)", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Web code or overlaps not acknowledged", body = ErrorEnvelope),
    )
)]
async fn enable_plugin(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
    Query(query): Query<ToggleQuery>,
) -> Result<Json<Data<Value>>, ApiError> {
    toggle(shared, session, client, slug, key, true, query).await
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/plugins/{key}/disable",
    tag = TAG,
    operation_id = "console_disable_tenant_plugin",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Plugin disabled (`PluginResponse`)", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn disable_plugin(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
) -> Result<Json<Data<Value>>, ApiError> {
    toggle(
        shared,
        session,
        client,
        slug,
        key,
        false,
        ToggleQuery::default(),
    )
    .await
}

#[utoipa::path(
    delete,
    path = "/tenants/{slug}/plugins/{key}",
    tag = TAG,
    operation_id = "console_uninstall_tenant_plugin",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Plugin key"), UninstallQuery),
    responses(
        (status = 204, description = "Plugin removed; with `purge=true` its records and fields are deleted too"),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn uninstall_plugin(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
    Query(query): Query<UninstallQuery>,
) -> Result<StatusCode, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let ip = client.0.ip.clone();
    let plugin = key.clone();
    scope(runtime, async move {
        plugin_administration()?
            .uninstall(&state, &AccessScope::system(), ip, &plugin, query.purge)
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.plugin_uninstalled",
        json!({ "slug": tenant.slug, "plugin": key, "purge": query.purge }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/tenants/{slug}/plugins/{key}/settings",
    tag = TAG,
    operation_id = "console_tenant_plugin_settings",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Settings with their current values (`PluginSettingResponse[]`); secrets only report `configured`", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn plugin_settings(
    State(shared): State<Shared>,
    _session: OperatorSession,
    Path((slug, key)): Path<(String, String)>,
) -> Result<Json<Data<Value>>, ApiError> {
    let (_, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let body = scope(runtime, async move {
        plugin_administration()?.settings(&state, &key).await
    })
    .await?;
    Ok(Json(Data::new(body)))
}

#[utoipa::path(
    put,
    path = "/tenants/{slug}/plugins/{key}/settings",
    tag = TAG,
    operation_id = "console_update_tenant_plugin_settings",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Plugin key")),
    request_body(content = Object, description = "Map of setting key to value"),
    responses(
        (status = 200, description = "Settings saved (`PluginResponse`)", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid settings (`plugin_settings_invalid`)", body = ErrorEnvelope),
    )
)]
async fn update_plugin_settings(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
    JsonBody(values): JsonBody<Map<String, Value>>,
) -> Result<Json<Data<Value>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let ip = client.0.ip.clone();
    let plugin = key.clone();
    let body = scope(runtime, async move {
        plugin_administration()?
            .update_settings(&state, &AccessScope::system(), ip, &plugin, values)
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.plugin_settings_updated",
        json!({ "slug": tenant.slug, "plugin": key }),
    )
    .await;
    Ok(Json(Data::new(body)))
}

#[utoipa::path(
    get,
    path = "/tenants/{slug}/modules",
    tag = TAG,
    operation_id = "console_list_tenant_modules",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Modules compiled into this build and their state in the organization", body = Data<Vec<ModuleResponse>>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown tenant (`tenant_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_modules(
    State(shared): State<Shared>,
    _session: OperatorSession,
    Path(slug): Path<String>,
) -> Result<Json<Data<Vec<ModuleResponse>>>, ApiError> {
    let (_, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let views = scope(runtime, async {
        module_use_cases(&state)?.list(&AccessScope::system()).await
    })
    .await?;
    Ok(Json(Data::new(views.into_iter().map(Into::into).collect())))
}

async fn set_module_enabled(
    shared: Shared,
    session: OperatorSession,
    client: ClientInfo,
    slug: String,
    key: String,
    enabled: bool,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let module = key.clone();
    let view = scope(runtime, async move {
        let use_cases = module_use_cases(&state)?;
        if enabled {
            use_cases.enable(&AccessScope::system(), &module).await
        } else {
            use_cases.disable(&AccessScope::system(), &module).await
        }
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        if enabled {
            "tenant.module_enabled"
        } else {
            "tenant.module_disabled"
        },
        json!({ "slug": tenant.slug, "module": key }),
    )
    .await;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/modules/{key}/enable",
    tag = TAG,
    operation_id = "console_enable_tenant_module",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "Module enabled", body = Data<ModuleResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Incompatible module or a dependency is disabled", body = ErrorEnvelope),
        (status = 422, description = "Required settings are missing (`module_settings_required`)", body = ErrorEnvelope),
    )
)]
async fn enable_module(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    set_module_enabled(shared, session, client, slug, key, true).await
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/modules/{key}/disable",
    tag = TAG,
    operation_id = "console_disable_tenant_module",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "Module disabled", body = Data<ModuleResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "An enabled module depends on it (`module_required_by`) or the build requires it (`module_required`)", body = ErrorEnvelope),
    )
)]
async fn disable_module(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    set_module_enabled(shared, session, client, slug, key, false).await
}

#[utoipa::path(
    get,
    path = "/tenants/{slug}/plugins/readiness",
    tag = TAG,
    operation_id = "console_tenant_plugins_readiness",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Plugin catalog readiness (`loading`, `ready` or `degraded`) and backend errors", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
    )
)]
async fn plugin_readiness(
    State(shared): State<Shared>,
    _session: OperatorSession,
    Path(slug): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let (_, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let body = scope(runtime, async move {
        plugin_administration()?.readiness(&state).await
    })
    .await?;
    Ok(Json(Data::new(body)))
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/plugins/packs",
    tag = TAG,
    operation_id = "console_install_tenant_plugin_pack",
    description = "Installs or upgrades a plugin pack (`.aphelpack`, multipart field `package`) in the organization. With `dry_run=true` nothing is installed and the response describes the pack. Status 207 means some plugins failed.",
    params(("slug" = String, Path, description = "Tenant slug"), InstallQuery),
    request_body(content_type = "multipart/form-data", description = "Field `package` with the .aphelpack file"),
    responses(
        (status = 200, description = "Dry run: what the pack declares and asks for", body = Data<PluginPayload>),
        (status = 201, description = "Pack installed", body = Data<PluginPayload>),
        (status = 207, description = "Some plugins of the pack failed", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "Conflicts or web code not acknowledged", body = ErrorEnvelope),
        (status = 422, description = "Invalid pack", body = ErrorEnvelope),
    )
)]
async fn install_pack(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
    Query(query): Query<InstallQuery>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Response, ApiError> {
    let (package, role_presets) = read_upload(multipart).await?;
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let ip = client.0.ip.clone();
    let dry_run = query.dry_run;
    let result = scope(runtime, async {
        plugin_administration()?
            .install_pack(
                &state,
                &AccessScope::system(),
                ip,
                PluginInstallRequest {
                    package,
                    role_presets,
                    dry_run,
                    acknowledge_conflicts: query.acknowledge_conflicts,
                    acknowledge_web_version: query.acknowledge_web_version,
                },
            )
            .await
    })
    .await?;
    if !dry_run {
        audit(
            &shared,
            &session,
            &client,
            &tenant,
            "tenant.plugin_pack_installed",
            json!({
                "slug": tenant.slug,
                "pack": result.body.get("key"),
                "version": result.body.get("version"),
            }),
        )
        .await;
    }
    let status = StatusCode::from_u16(result.status).unwrap_or(StatusCode::OK);
    Ok((status, Json(Data::new(result.body))).into_response())
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/plugins/{key}/grants",
    tag = TAG,
    operation_id = "console_reapply_tenant_plugin_grants",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Grants reapplied to the roles that exist now (`PluginResponse`)", body = Data<PluginPayload>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn reapply_grants(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
) -> Result<Json<Data<Value>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let ip = client.0.ip.clone();
    let plugin = key.clone();
    let body = scope(runtime, async move {
        plugin_administration()?
            .reapply_grants(&state, &AccessScope::system(), ip, &plugin)
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.plugin_grants_reapplied",
        json!({ "slug": tenant.slug, "plugin": key }),
    )
    .await;
    Ok(Json(Data::new(body)))
}

#[utoipa::path(
    put,
    path = "/tenants/{slug}/modules/{key}/settings",
    tag = TAG,
    operation_id = "console_update_tenant_module_settings",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Module key")),
    request_body = UpdateModuleSettingsRequest,
    responses(
        (status = 200, description = "Saved", body = Data<ModuleResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid settings", body = ErrorEnvelope),
    )
)]
async fn update_module_settings(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
    JsonBody(request): JsonBody<UpdateModuleSettingsRequest>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let module = key.clone();
    let view = scope(runtime, async move {
        module_use_cases(&state)?
            .update_settings(&AccessScope::system(), &module, request.settings)
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.module_settings_updated",
        json!({ "slug": tenant.slug, "module": key }),
    )
    .await;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/modules/{key}/data-export",
    tag = TAG,
    operation_id = "console_export_tenant_module_data",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Module key")),
    responses(
        (status = 200, description = "The exported data", body = Data<ModuleDataExportResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
    )
)]
async fn export_module_data(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
) -> Result<Json<Data<ModuleDataExportResponse>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let module = key.clone();
    let export = scope(runtime, async move {
        module_use_cases(&state)?
            .export_data(&AccessScope::system(), &module)
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.module_data_exported",
        json!({ "slug": tenant.slug, "module": key }),
    )
    .await;
    Ok(Json(Data::new(export.into())))
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/modules/{key}/data-purge",
    tag = TAG,
    operation_id = "console_purge_tenant_module_data",
    params(("slug" = String, Path, description = "Tenant slug"), ("key" = String, Path, description = "Module key")),
    request_body = PurgeModuleDataRequest,
    responses(
        (status = 200, description = "Data removed", body = Data<ModuleResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown module (`module_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The module is enabled or was not exported after being disabled", body = ErrorEnvelope),
        (status = 422, description = "Missing confirmation (`module_confirmation_required`)", body = ErrorEnvelope),
    )
)]
async fn purge_module_data(
    State(shared): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, key)): Path<(String, String)>,
    JsonBody(request): JsonBody<PurgeModuleDataRequest>,
) -> Result<Json<Data<ModuleResponse>>, ApiError> {
    let (tenant, runtime) = open(&shared, &slug).await?;
    let state = tenant_state(&runtime)?;
    let module = key.clone();
    let view = scope(runtime, async move {
        module_use_cases(&state)?
            .purge_data(&AccessScope::system(), &module, &request.confirm)
            .await
    })
    .await?;
    audit(
        &shared,
        &session,
        &client,
        &tenant,
        "tenant.module_data_purged",
        json!({ "slug": tenant.slug, "module": key }),
    )
    .await;
    Ok(Json(Data::new(view.into())))
}
