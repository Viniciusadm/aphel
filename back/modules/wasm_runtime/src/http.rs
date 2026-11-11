use std::sync::LazyLock;
use std::time::Duration;

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::multipart::MultipartRejection;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, RawQuery, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::compiled;
use crate::grants;
use crate::host::setting_secret_key;
use crate::manifest::{HostCatalog, PluginManifest, match_route};
use crate::package::{MAX_PACKAGE_BYTES, PUBLIC_APP_INDEX, read_package};
use crate::registry;
use crate::runtime::{self, wit};
use crate::schema;
use crate::store::{self, InstalledPlugin};
use aphel_kernel::domain::Secret;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo};
use aphel_sdk::platform::api::rate_limit::SharedRateLimiter;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::organization_profile::CORE_UI_PLUGIN;
use aphel_sdk::platform::application::secrets::MAX_SECRET_BYTES;
use aphel_sdk::platform::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::infrastructure::organization_profile::{
    clear_ui_plugin, find_ui_plugin_default,
};
use aphel_sdk::platform::infrastructure::repositories::runtime_permissions::{
    RuntimeGrant, remove_runtime_permissions, replace_runtime_permissions,
    set_runtime_permissions_active,
};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub const TAG: &str = "plugins";
const UI_PLUGIN_HEADER: &str = "x-aphel-ui-plugin";
const PLUGIN_NOT_FOUND: &str = "plugin_not_found";
const PLUGIN_ROUTE_NOT_FOUND: &str = "plugin_route_not_found";
const INVALID_SETTINGS: &str = "plugin_settings_invalid";
const WEB_PREFIX: &str = "web/";
const PUBLIC_APP_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'";
const PERMISSIONS_OWNER_PREFIX: &str = "plugin:";
const REDACTED: &str = "[redacted]";
const PUBLIC_PER_ADDRESS_LIMIT: u32 = 60;
const PUBLIC_PER_PLUGIN_LIMIT: u32 = 600;
const MINUTE: Duration = Duration::from_secs(60);
const WITHHELD_HEADERS: [HeaderName; 3] = [
    header::AUTHORIZATION,
    header::COOKIE,
    header::PROXY_AUTHORIZATION,
];

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(
            routes!(list_plugins, install_plugin)
                .layer(DefaultBodyLimit::max(MAX_PACKAGE_BYTES + 1024 * 1024)),
        )
        .routes(
            routes!(list_packs, install_pack).layer(DefaultBodyLimit::max(
                crate::pack::MAX_PACK_BYTES + 1024 * 1024,
            )),
        )
        .routes(routes!(uninstall_plugin))
        .routes(routes!(enable_plugin))
        .routes(routes!(disable_plugin))
        .routes(routes!(update_plugin_settings, list_plugin_settings))
        .routes(routes!(reapply_plugin_grants))
        .routes(routes!(plugin_changes))
        .routes(routes!(plugin_change_stream))
        .routes(routes!(host_catalog))
        .routes(routes!(catalog_readiness))
        .route("/p/{key}/{*path}", any(dispatch))
}

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(web_plugins))
        .route("/public/wasm-runtime/{key}/{*path}", get(web_asset))
        .route("/public/p/{key}/{*path}", any(public_dispatch))
}

pub(crate) fn app_mount_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .route("/p/{key}", get(public_app_redirect))
        .route("/p/{key}/", get(public_app_index))
        .route("/p/{key}/{*path}", get(public_app_file))
}

fn not_found(key: &str) -> AppError {
    AppError::not_found_code(
        PLUGIN_NOT_FOUND,
        format!("the plugin {key} is not installed"),
    )
}

async fn installed(state: &AppState, key: &str) -> Result<InstalledPlugin, AppError> {
    store::find(&state.db()?, key)
        .await?
        .ok_or_else(|| not_found(key))
}

async fn record(
    state: &AppState,
    scope: &AccessScope,
    action: &'static str,
    plugin: &str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let pool = state.db()?;
    let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
        pool.begin().await.map_err(AppError::from)?,
    );
    record_in(&mut tx, state, scope, action, plugin, changes, ip).await?;
    tx.commit().await?;
    registry::invalidate().await;
    Ok(())
}

async fn record_in(
    tx: &mut MySqlTx,
    state: &AppState,
    scope: &AccessScope,
    action: &'static str,
    plugin: &str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    let actor = Actor::from_scope(scope);
    store::bump(tx).await?;
    state
        .events
        .publish(
            tx,
            DomainEvent::new(actor.clone(), action, now).with_payload(json!({ "plugin": plugin })),
        )
        .await?;
    state
        .audit
        .record(
            tx,
            AuditEntry::new(actor, action, now)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await?;
    Ok(())
}

fn grant_changes(report: &grants::GrantReport) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    if !report.created_roles.is_empty() {
        changes.push(FieldChange::new(
            "created_roles",
            Value::Null,
            json!(report.created_roles),
        ));
    }
    if report.granted > 0 {
        changes.push(FieldChange::new(
            "granted_permissions",
            Value::Null,
            json!(report.granted),
        ));
    }
    changes
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginSettingResponse {
    key: String,
    label: String,
    #[serde(rename = "type")]
    kind: String,
    help: Option<String>,
    min: Option<i64>,
    max: Option<i64>,
    group: Option<String>,
    options: Vec<crate::manifest::SettingOption>,
    options_from: Option<crate::manifest::SettingOptionsSource>,
    mask: Option<String>,
    #[schema(value_type = Object)]
    value: Value,
    configured: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginPermissionResponse {
    key: String,
    label: String,
    description: Option<String>,
    grant_to: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginRolePresetResponse {
    key: String,
    label: String,
    description: Option<String>,
    permissions: Vec<String>,
    scope: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginEntityScopeResponse {
    record_type: String,
    permission_scope: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginDeclarations {
    entities: Vec<String>,
    extended_types: Vec<String>,
    hooks: Vec<String>,
    events: Vec<String>,
    routes: Vec<String>,
    public_routes: Vec<String>,
    requires: Vec<String>,
    optional_requires: Vec<String>,
    permissions: Vec<PluginPermissionResponse>,
    allowed_hosts: Vec<String>,
    schedules: Vec<String>,
    secrets: Vec<String>,
    actions: Vec<String>,
    emits: Vec<String>,
    notifications: Vec<String>,
    widgets: Vec<String>,
    operations: Vec<String>,
    optional_operations: Vec<String>,
    role_presets: Vec<PluginRolePresetResponse>,
    entity_scopes: Vec<PluginEntityScopeResponse>,
    unique_fields: Vec<String>,
    menu: crate::manifest::MenuSpec,
    replaces: Vec<String>,
    ui: Option<String>,
    has_web: bool,
}

impl From<&PluginManifest> for PluginDeclarations {
    fn from(manifest: &PluginManifest) -> Self {
        let mut extended: Vec<String> = manifest
            .fields
            .iter()
            .map(|field| field.record_type.clone())
            .collect();
        extended.sort();
        extended.dedup();
        let routes = |audience: crate::manifest::RouteAudience| {
            manifest
                .routes
                .iter()
                .filter(|route| route.audience() == audience)
                .map(|route| format!("{} {}", route.method, route.path))
                .collect()
        };
        Self {
            entities: manifest.entity_record_types(),
            extended_types: extended,
            hooks: manifest
                .hooks
                .subscribe
                .iter()
                .map(|hook| format!("{} ({})", hook.hook, hook.record_types.join(", ")))
                .collect(),
            events: manifest.events.clone(),
            routes: routes(crate::manifest::RouteAudience::Private),
            public_routes: routes(crate::manifest::RouteAudience::Public)
                .into_iter()
                .chain(
                    manifest
                        .web
                        .as_ref()
                        .and_then(|web| web.public.as_ref())
                        .map(|_| format!("GET /p/{}/", manifest.key)),
                )
                .chain(crate::site::public_routes(manifest))
                .collect(),
            requires: manifest.requires.clone(),
            optional_requires: manifest.optional_requires.clone(),
            permissions: manifest
                .permissions
                .iter()
                .map(|permission| PluginPermissionResponse {
                    key: permission.key.clone(),
                    label: permission.label.clone(),
                    description: permission.description.clone(),
                    grant_to: permission.grant_to.clone(),
                })
                .collect(),
            allowed_hosts: manifest.http.allowed_hosts.clone(),
            schedules: manifest
                .schedules
                .iter()
                .map(|schedule| format!("{} ({})", schedule.name, schedule.cron))
                .collect(),
            secrets: manifest
                .secret_settings()
                .map(|setting| setting.key.clone())
                .collect(),
            actions: manifest
                .actions
                .iter()
                .map(|action| action.label.clone())
                .collect(),
            emits: manifest
                .emits
                .iter()
                .map(|emit| emit.label.clone())
                .collect(),
            notifications: manifest
                .notifications
                .iter()
                .map(|notification| notification.label.clone())
                .collect(),
            widgets: manifest
                .widgets
                .iter()
                .map(|widget| widget.title.clone())
                .collect(),
            operations: manifest.operations.clone(),
            optional_operations: manifest.optional_operations.clone(),
            role_presets: manifest
                .role_presets
                .iter()
                .map(|preset| PluginRolePresetResponse {
                    key: preset.key.clone(),
                    label: preset.label.clone(),
                    description: preset.description.clone(),
                    permissions: preset.permissions.clone(),
                    scope: preset.scope.clone(),
                })
                .collect(),
            entity_scopes: manifest
                .entities
                .iter()
                .filter_map(|entity| {
                    manifest
                        .entity_scope(entity)
                        .map(|scope| PluginEntityScopeResponse {
                            record_type: manifest.entity_record_type(&entity.key),
                            permission_scope: scope.as_str().to_owned(),
                        })
                })
                .collect(),
            unique_fields: manifest
                .entities
                .iter()
                .flat_map(|entity| {
                    let record_type = manifest.entity_record_type(&entity.key);
                    entity
                        .fields
                        .iter()
                        .filter(|field| field.unique)
                        .map(move |field| format!("{record_type}.{}", field.key))
                })
                .chain(
                    manifest
                        .fields
                        .iter()
                        .filter(|extension| extension.field.unique)
                        .map(|extension| {
                            format!("{}.{}", extension.record_type, extension.field.key)
                        }),
                )
                .collect(),
            menu: manifest.menu.clone(),
            replaces: crate::modules::held_modules(manifest),
            ui: manifest.ui.as_ref().map(|ui| ui.label.clone()),
            has_web: manifest.web.is_some(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginResponse {
    key: String,
    name: String,
    version: String,
    description: Option<String>,
    enabled: bool,
    #[serde(flatten)]
    declares: PluginDeclarations,
    backend_error: Option<String>,
    settings: Vec<PluginSettingResponse>,
    grants: Vec<grants::RoleGrantStatus>,
    ungranted_roles: Vec<String>,
    installed_at: String,
    updated_at: String,
    pack: Option<PackReference>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct PackReference {
    key: String,
    name: String,
    version: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginConsentResponse {
    key: String,
    name: String,
    version: String,
    description: Option<String>,
    installed_version: Option<String>,
    #[serde(flatten)]
    declares: PluginDeclarations,
    grants: Vec<grants::RoleGrantStatus>,
    conflicts: Vec<crate::conflicts::PluginConflict>,
    module_conflicts: Vec<crate::modules::ModuleConflictPlan>,
}

async fn setting_responses(
    state: &AppState,
    plugin: &InstalledPlugin,
) -> Result<Vec<PluginSettingResponse>, AppError> {
    let values = plugin.effective_settings();
    let mut settings = Vec::with_capacity(plugin.manifest.settings.len());
    for setting in &plugin.manifest.settings {
        let configured = if crate::manifest::is_sensitive_setting(&setting.kind) {
            Some(
                state
                    .secrets
                    .reveal(
                        &AccessScope::system(),
                        &setting_secret_key(&plugin.key, setting),
                    )
                    .await?
                    .is_some(),
            )
        } else {
            None
        };
        settings.push(PluginSettingResponse {
            key: setting.key.clone(),
            label: setting.label.clone(),
            kind: setting.kind.clone(),
            help: setting.help.clone(),
            min: setting.min,
            max: setting.max,
            group: setting.group.clone(),
            options: setting.options.clone(),
            options_from: setting.options_from.clone(),
            mask: setting.mask.clone(),
            value: if crate::manifest::is_sensitive_setting(&setting.kind) {
                Value::Null
            } else {
                values.get(&setting.key).cloned().unwrap_or(Value::Null)
            },
            configured,
        });
    }
    Ok(settings)
}

async fn respond(state: &AppState, plugin: InstalledPlugin) -> Result<PluginResponse, AppError> {
    let snapshot = registry::current(&state.db()?).await?;
    let pack = match &plugin.pack_key {
        Some(key) => store::find_pack(&state.db()?, key)
            .await?
            .map(|pack| PackReference {
                key: pack.key,
                name: pack.name,
                version: pack.version,
            }),
        None => None,
    };
    let settings = setting_responses(state, &plugin).await?;
    let grants = grants::status(&state.db()?, &plugin.manifest).await?;
    let ungranted_roles = grants
        .iter()
        .filter(|status| !status.role_exists || status.granted < status.expected)
        .map(|status| status.role.clone())
        .collect();
    Ok(PluginResponse {
        grants,
        ungranted_roles,
        declares: PluginDeclarations::from(&plugin.manifest),
        backend_error: snapshot.backend_error(&plugin.key),
        settings,
        installed_at: plugin.installed_at.to_rfc3339(),
        updated_at: plugin.updated_at.to_rfc3339(),
        pack,
        key: plugin.key,
        name: plugin.name,
        version: plugin.version,
        description: plugin.description,
        enabled: plugin.enabled,
    })
}

async fn prune_compiled(state: &AppState) -> Result<(), AppError> {
    let pool = state.db()?;
    let mut keep = Vec::new();
    for plugin in store::list(&pool).await? {
        if let Some(bytes) = store::backend(&pool, &plugin.key).await? {
            keep.push(runtime::backend_hash(&bytes));
        }
    }
    compiled::prune(&pool, &keep).await
}

fn permissions_owner(key: &str) -> String {
    format!("{PERMISSIONS_OWNER_PREFIX}{key}")
}

async fn sync_permissions(
    tx: &mut MySqlTx,
    manifest: &PluginManifest,
    active: bool,
) -> Result<(), AppError> {
    let permissions: Vec<(Permission, Vec<String>)> = manifest
        .permissions
        .iter()
        .map(|declared| {
            Permission::parse(declared.key.clone())
                .map(|permission| (permission, declared.grant_to.clone()))
                .map_err(|_| AppError::Validation(format!("invalid permission {}", declared.key)))
        })
        .collect::<Result<_, _>>()?;
    let grants: Vec<RuntimeGrant<'_>> = permissions
        .iter()
        .zip(&manifest.permissions)
        .map(|((permission, roles), declared)| RuntimeGrant {
            permission,
            roles,
            label: &declared.label,
            description: declared.description.as_deref(),
        })
        .collect();
    replace_runtime_permissions(tx, &permissions_owner(&manifest.key), &grants, active).await
}

#[utoipa::path(
    get,
    path = "/wasm-runtime",
    tag = TAG,
    operation_id = "plugins_list",
    security(("session_cookie" = [])),
    description = "Lists the plugins installed at runtime (permission `system.manage`), with what each one declares: entities, extended record types, hooks, consumed events, routes and settings. `backend_error` is set when the installed backend no longer loads: writes to the record types it hooks then fail with 503 `plugin_failed` until the plugin is reinstalled.",
    responses(
        (status = 200, description = "Installed plugins", body = Data<Vec<PluginResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_plugins(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<PluginResponse>>>, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let mut plugins = Vec::new();
    for plugin in store::list(&state.db()?).await? {
        plugins.push(respond(&state, plugin).await?);
    }
    Ok(Json(Data::new(plugins)))
}

#[utoipa::path(
    get,
    path = "/wasm-runtime/readiness",
    tag = TAG,
    operation_id = "plugins_readiness",
    security(("session_cookie" = [])),
    description = "Reports metadata catalog readiness independently of lazy backend compilation. Catalog failures return 503 rather than an empty catalog.",
    responses(
        (status = 200, description = "Catalog readiness", body = Data<registry::CatalogReadiness>),
        (status = 503, description = "Catalog unavailable", body = ErrorEnvelope),
    )
)]
async fn catalog_readiness(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<registry::CatalogReadiness>>, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    match tokio::time::timeout(Duration::from_millis(100), registry::current(&state.db()?)).await {
        Ok(result) => {
            result?;
        }
        Err(_) => {
            return Ok(Json(Data::new(registry::CatalogReadiness {
                state: "loading".to_owned(),
                version: None,
                backend_errors: vec![],
            })));
        }
    }
    Ok(Json(Data::new(registry::readiness())))
}

#[utoipa::path(
    get,
    path = "/wasm-runtime/host-catalog",
    tag = TAG,
    operation_id = "plugins_host_catalog",
    security(("session_cookie" = [])),
    description = "Lists what this Aphel offers to plugins (permission `system.manage`): the hooks a manifest can subscribe to in `hooks` (generic record hooks plus the domain hooks of compiled modules, with kind `veto`, `filter` or `action`) and the operations a plugin can call with `invoke` after declaring them in `operations`. `enabled` is false when the owning module is disabled: its hooks never fire and its operations answer 404 `plugin_operation_unknown`.",
    responses(
        (status = 200, description = "Hooks and operations offered to plugins", body = Data<crate::bridge::HostCatalogResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn host_catalog(
    tenant: AuthContext,
) -> Result<Json<Data<crate::bridge::HostCatalogResponse>>, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    Ok(Json(Data::new(crate::bridge::host_catalog_response())))
}

#[utoipa::path(
    post,
    path = "/wasm-runtime",
    tag = TAG,
    operation_id = "plugins_install",
    security(("session_cookie" = [])),
    description = "Installs or upgrades a plugin from an `.aphelplugin` package sent as the multipart field `package` (permission `system.manage`). The package is a zip with `plugin.toml`, an optional `backend.wasm` built with aphel-plugin-sdk and an optional `web/` bundle. Declared entities become custom objects managed by the plugin, declared fields become managed custom fields, declared permissions enter the role catalog, and hooks, events, schedules and routes start working without restarting Aphel. The plugin acts with what it `requires` intersected with the permissions of whoever installs it, so the installer gives consent: call first with `dry_run=true` to get what the package asks for (status 200, nothing is installed). Integrated frontend code runs with the browser session, independently of backend capabilities; packages with web code require acknowledge_web_version=<manifest version> on each install or upgrade, including upgrades kept disabled. Dry runs need no admission. Publishes `plugin.installed`.",
    params(InstallParams),
    request_body(content_type = "multipart/form-data", description = "Field `package` with the .aphelplugin file"),
    responses(
        (status = 200, description = "Dry run: what the package declares and asks for", body = Data<PluginConsentResponse>),
        (status = 201, description = "Plugin installed or upgraded", body = Data<PluginResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "A custom object, field or permission with the same key already exists (`plugin_object_conflict`, `plugin_field_conflict`, `permission_already_registered`), integrated frontend code was not admitted (`plugin_web_unacknowledged`), or the plugin overlaps with another installed plugin and `acknowledge_conflicts` was not sent (`plugin_conflicts_unacknowledged`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid package, manifest or backend (`plugin_package_invalid`, `plugin_manifest_invalid`, `plugin_backend_invalid`)", body = ErrorEnvelope),
    )
)]
async fn install_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Query(params): Query<InstallParams>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Response, ApiError> {
    let scope = &tenant.scope;
    scope.require(&Permission::SYSTEM_MANAGE)?;
    let (bytes, presets) = package_bytes(multipart).await?;
    let package = read_package(&bytes)?;
    let prepared = prepare_install(&state, scope, package, &[], &[], &[]).await?;
    let presets = grants::selected_presets(&prepared.package.manifest, &presets)?;
    if params.dry_run {
        return Ok(Json(Data::new(consent_of(&state, &prepared).await?)).into_response());
    }
    require_web_admission(
        &prepared.package.manifest,
        params.acknowledge_web_version.as_deref(),
    )?;
    crate::conflicts::require_acknowledged(&prepared.conflicts, params.acknowledge_conflicts)?;
    crate::modules::ensure_unblocked(&prepared.module_conflicts)?;
    let plugin = apply_install(&state, scope, client.0.ip, &prepared, &presets, None).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(respond(&state, plugin).await?)),
    )
        .into_response())
}

pub(crate) struct PreparedInstall {
    pub package: crate::package::PluginPackage,
    pub previous: Option<InstalledPlugin>,
    pub conflicts: Vec<crate::conflicts::PluginConflict>,
    pub module_conflicts: Vec<crate::modules::ModuleConflictPlan>,
}

pub(crate) async fn prepare_install(
    state: &AppState,
    scope: &AccessScope,
    package: crate::package::PluginPackage,
    extra_operations: &[String],
    pack: &[PluginManifest],
    pending_modules: &[String],
) -> Result<PreparedInstall, AppError> {
    let compiled: Vec<String> = state
        .app
        .modules()
        .iter()
        .map(|module| module.manifest.key.to_owned())
        .collect();
    let catalog = crate::bridge::host_catalog();
    let mut operations: Vec<&str> = catalog.operations.clone();
    operations.extend(extra_operations.iter().map(String::as_str));
    package.manifest.validate(
        &compiled,
        APHEL_VERSION,
        HostCatalog {
            hooks: &catalog.hooks,
            operations: &operations,
        },
    )?;
    if package.manifest.needs_backend() && package.backend().is_none() {
        return Err(AppError::validation_code(
            "plugin_backend_missing",
            "the plugin declares runtime callbacks but has no backend.wasm",
        ));
    }
    if let Some(backend) = package.backend() {
        runtime::validate(backend.content.clone()).await?;
    }
    let pool = state.db()?;
    let installed = crate::registry::current(&pool).await?;
    let others: Vec<&PluginManifest> = installed
        .plugins
        .iter()
        .map(|plugin| &plugin.manifest)
        .chain(pack.iter())
        .collect();
    crate::site::validate_package(
        &package.manifest,
        &package.files,
        &others,
        &aphel_sdk::module::public_app_paths(state),
    )?;
    let previous = store::find(&pool, &package.manifest.key).await?;
    if previous.as_ref().is_some_and(|plugin| plugin.enabled) {
        crate::dependents::ensure_none_broken(
            state,
            &package.manifest.key,
            Some(&package.manifest),
        )
        .await?;
    }
    schema::check_with(state, &package.manifest, pack, pending_modules).await?;
    let conflicts = crate::conflicts::detect_for(&pool, &package.manifest).await?;
    let module_conflicts = crate::modules::plan(state, scope, &package.manifest).await?;
    Ok(PreparedInstall {
        package,
        previous,
        conflicts,
        module_conflicts,
    })
}

pub(crate) async fn consent_of(
    state: &AppState,
    prepared: &PreparedInstall,
) -> Result<PluginConsentResponse, AppError> {
    let manifest = &prepared.package.manifest;
    Ok(PluginConsentResponse {
        key: manifest.key.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        installed_version: prepared
            .previous
            .as_ref()
            .map(|plugin| plugin.version.clone()),
        declares: PluginDeclarations::from(manifest),
        grants: grants::status(&state.db()?, manifest).await?,
        conflicts: prepared.conflicts.clone(),
        module_conflicts: prepared.module_conflicts.clone(),
    })
}

pub(crate) async fn apply_install(
    state: &AppState,
    scope: &AccessScope,
    ip: Option<String>,
    prepared: &PreparedInstall,
    presets: &[String],
    pack_key: Option<&str>,
) -> Result<InstalledPlugin, AppError> {
    let package = &prepared.package;
    let pool = state.db()?;
    let enabled = prepared
        .previous
        .as_ref()
        .is_none_or(|plugin| plugin.enabled);
    let mut held: Vec<String> = Vec::new();
    let outcome: Result<InstalledPlugin, AppError> = async {
        schema::apply(state, scope, &package.manifest).await?;
        let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
            pool.begin().await.map_err(AppError::from)?,
        );
        store::save(&mut tx, package, Some(scope.user_id()), pack_key).await?;
        sync_permissions(&mut tx, &package.manifest, enabled).await?;
        let report = grants::reconcile(&mut tx, &package.manifest, presets).await?;
        let mut changes = vec![FieldChange::new(
            "version",
            json!(
                prepared
                    .previous
                    .as_ref()
                    .map(|plugin| plugin.version.clone())
            ),
            json!(package.manifest.version),
        )];
        changes.extend(grant_changes(&report));
        record_in(
            &mut tx,
            state,
            scope,
            "plugin.installed",
            &package.manifest.key,
            changes,
            ip,
        )
        .await?;
        tx.commit().await.map_err(AppError::from)?;
        if enabled {
            held = crate::modules::hold(state, scope, &package.manifest).await?;
        }
        registry::invalidate().await;
        if let Err(error) = prune_compiled(state).await {
            tracing::warn!(error = %error, "could not prune precompiled plugin backends");
        }
        let plugin = installed(state, &package.manifest.key).await?;
        if !plugin.enabled {
            schema::set_active(state, &plugin.manifest, false).await?;
        }
        Ok(plugin)
    }
    .await;
    match outcome {
        Ok(plugin) => Ok(plugin),
        Err(error) => {
            crate::modules::release_modules(state, scope, &package.manifest.key, &held).await;
            Err(error)
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PackPluginConsentResponse {
    key: String,
    name: String,
    version: String,
    action: &'static str,
    settings: Vec<PackSettingResponse>,
    consent: Option<PluginConsentResponse>,
    error: Option<crate::pack::PackPluginError>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PackSettingResponse {
    key: String,
    label: String,
    value: Value,
}

fn labeled_settings(
    manifest: &PluginManifest,
    values: Map<String, Value>,
) -> Vec<PackSettingResponse> {
    values
        .into_iter()
        .map(|(key, value)| PackSettingResponse {
            label: manifest
                .settings
                .iter()
                .find(|setting| setting.key == key)
                .map(|setting| setting.label.clone())
                .unwrap_or_else(|| key.clone()),
            key,
            value,
        })
        .collect()
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginPackConsentResponse {
    key: String,
    name: String,
    version: String,
    description: Option<String>,
    installed_version: Option<String>,
    role_presets: Vec<String>,
    web: bool,
    plugins: Vec<PackPluginConsentResponse>,
    modules_to_enable: Vec<String>,
    provisions: Vec<crate::provision::ProvisionPlanResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PackPluginResultResponse {
    key: String,
    version: String,
    action: &'static str,
    status: &'static str,
    error: Option<crate::pack::PackPluginError>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginPackResponse {
    key: String,
    name: String,
    version: String,
    description: Option<String>,
    complete: bool,
    plugins: Vec<PackPluginResultResponse>,
    modules_enabled: Vec<String>,
    provisions: Vec<crate::provision::ProvisionResultResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct InstalledPackResponse {
    key: String,
    name: String,
    version: String,
    description: Option<String>,
    plugins: Vec<String>,
    installed_at: String,
    updated_at: String,
}

const ACTION_INSTALL: &str = "install";
const ACTION_UPGRADE: &str = "upgrade";
const ACTION_UNCHANGED: &str = "unchanged";

fn pack_action(prepared: &PreparedInstall) -> &'static str {
    match &prepared.previous {
        None => ACTION_INSTALL,
        Some(previous) if previous.checksum == prepared.package.checksum => ACTION_UNCHANGED,
        Some(_) => ACTION_UPGRADE,
    }
}

fn pending_settings(
    member: &crate::pack::PackMember,
    previous: Option<&InstalledPlugin>,
) -> Map<String, Value> {
    member
        .settings
        .iter()
        .filter(|(key, _)| previous.is_none_or(|plugin| !plugin.settings.contains_key(*key)))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[utoipa::path(
    get,
    path = "/wasm-runtime/packs",
    tag = TAG,
    operation_id = "plugin_packs_list",
    security(("session_cookie" = [])),
    description = "Lists the plugin packs installed in this Aphel with the plugins that still belong to each (permission `system.manage`).",
    responses(
        (status = 200, description = "Installed packs", body = Data<Vec<InstalledPackResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_packs(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<InstalledPackResponse>>>, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let pool = state.db()?;
    let plugins = store::list(&pool).await?;
    let packs = store::list_packs(&pool)
        .await?
        .into_iter()
        .filter_map(|pack| {
            let members: Vec<String> = plugins
                .iter()
                .filter(|plugin| plugin.pack_key.as_deref() == Some(pack.key.as_str()))
                .map(|plugin| plugin.key.clone())
                .collect();
            (!members.is_empty()).then(|| InstalledPackResponse {
                key: pack.key,
                name: pack.name,
                version: pack.version,
                description: pack.description,
                plugins: members,
                installed_at: pack.installed_at.to_rfc3339(),
                updated_at: pack.updated_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(Data::new(packs)))
}

#[utoipa::path(
    post,
    path = "/wasm-runtime/packs",
    tag = TAG,
    operation_id = "plugin_packs_install",
    security(("session_cookie" = [])),
    description = "Installs or upgrades a plugin pack: an `.aphelpack` zip with `pack.toml` and the `.aphelplugin` packages it lists, sent as the multipart field `package` (permission `system.manage`). The plugins are installed in dependency order (a plugin that calls an operation or listens to an event of another plugin of the pack comes after it). Every plugin is validated before anything is installed; call first with `dry_run=true` to get each plugin's consent, action (`install`, `upgrade`, `unchanged`), error and the pack settings that will apply. Plugins whose package did not change are skipped. Pack settings fill only the settings that have no value yet. `acknowledge_web_version=<pack version>` admits the integrated frontend code of every plugin of the pack. If a plugin fails while installing, the following ones are skipped and the response is 207; sending the pack again continues from there. `requires_aphel` and `requires_modules` are checked first (`plugin_pack_aphel_incompatible`, `plugin_pack_module_unavailable`); required modules that are disabled are enabled before the plugins. After the plugins, the `[[provision]]` resources are created in dependency order and reported in `provisions` (`created`, `updated`, `kept`, `user_changed`, `removed`, `failed`, `skipped`); a failed resource makes the response 207 and is retried on the next install. Publishes `plugin.pack_installed`.",
    params(InstallParams),
    request_body(content_type = "multipart/form-data", description = "Field `package` with the .aphelpack file and optional `role_presets` (comma separated)"),
    responses(
        (status = 200, description = "Dry run: the pack, its plugins in install order and what each asks for", body = Data<PluginPackConsentResponse>),
        (status = 201, description = "Pack installed or upgraded", body = Data<PluginPackResponse>),
        (status = 207, description = "A plugin failed while installing; the response says which ones were installed", body = Data<PluginPackResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "Integrated frontend code was not admitted (`plugin_web_unacknowledged`) or overlaps were not acknowledged (`plugin_conflicts_unacknowledged`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pack (`plugin_pack_invalid`, `plugin_pack_setting_invalid`, `plugin_pack_dependency_cycle`, `plugin_pack_missing_dependency`) or a plugin of the pack is invalid (`plugin_pack_plugins_invalid`)", body = ErrorEnvelope),
    )
)]
async fn install_pack(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Query(params): Query<InstallParams>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Response, ApiError> {
    let scope = &tenant.scope;
    scope.require(&Permission::SYSTEM_MANAGE)?;
    let (bytes, presets) = package_bytes(multipart).await?;
    let pack = crate::pack::read_pack(&bytes)?;
    let requirements = crate::provision::requirements(&state, scope, &pack.manifest).await?;
    let catalog = crate::bridge::host_catalog();
    crate::pack::ensure_dependencies(&pack, &catalog.operations)?;
    let provided = pack.provided_operations();
    let keys = pack.keys();
    let manifests: Vec<PluginManifest> = pack
        .members
        .iter()
        .map(|member| member.package.manifest.clone())
        .collect();
    let selected: Vec<String> = if presets.is_empty() && params.dry_run {
        pack.manifest.role_presets.clone()
    } else {
        presets
    };
    let mut prepared = Vec::with_capacity(pack.members.len());
    for member in &pack.members {
        let outcome = prepare_install(
            &state,
            scope,
            member.package.clone(),
            &provided,
            &manifests,
            &requirements.modules_to_enable,
        )
        .await
        .and_then(|mut ready| {
            ready
                .conflicts
                .retain(|conflict| !keys.contains(&conflict.other_plugin));
            let own: Vec<String> = selected
                .iter()
                .filter(|preset| {
                    ready
                        .package
                        .manifest
                        .role_presets
                        .iter()
                        .any(|candidate| &&candidate.key == preset)
                })
                .cloned()
                .collect();
            grants::selected_presets(&ready.package.manifest, &own).map(|own| (ready, own))
        });
        prepared.push(outcome);
    }
    let previous_pack = store::find_pack(&state.db()?, &pack.manifest.key).await?;
    if params.dry_run {
        let mut plugins = Vec::with_capacity(pack.members.len());
        for (member, outcome) in pack.members.iter().zip(&prepared) {
            let manifest = &member.package.manifest;
            let (action, consent, error, settings) = match outcome {
                Ok((ready, _)) => (
                    pack_action(ready),
                    Some(consent_of(&state, ready).await?),
                    None,
                    pending_settings(member, ready.previous.as_ref()),
                ),
                Err(error) => (
                    ACTION_INSTALL,
                    None,
                    Some(crate::pack::PackPluginError::from_error(error)),
                    member.settings.clone(),
                ),
            };
            plugins.push(PackPluginConsentResponse {
                key: manifest.key.clone(),
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                action,
                settings: labeled_settings(manifest, settings),
                consent,
                error,
            });
        }
        let consent = PluginPackConsentResponse {
            key: pack.manifest.key.clone(),
            name: pack.manifest.name.clone(),
            version: pack.manifest.version.clone(),
            description: pack.manifest.description.clone(),
            installed_version: previous_pack.map(|existing| existing.version),
            role_presets: pack.manifest.role_presets.clone(),
            web: pack
                .members
                .iter()
                .any(|member| member.package.manifest.web.is_some()),
            plugins,
            modules_to_enable: requirements.modules_to_enable.clone(),
            provisions: crate::provision::plan(
                &state,
                scope,
                &pack.manifest,
                &requirements.modules_to_enable,
            )
            .await?,
        };
        return Ok(Json(Data::new(consent)).into_response());
    }
    let failures: Vec<Value> = pack
        .members
        .iter()
        .zip(&prepared)
        .filter_map(|(member, outcome)| {
            outcome.as_ref().err().map(|error| {
                json!({
                    "plugin": member.package.manifest.key,
                    "error": crate::pack::PackPluginError::from_error(error),
                })
            })
        })
        .collect();
    if !failures.is_empty() {
        return Err(AppError::with_details(
            aphel_kernel::errors::CodedKind::Validation,
            crate::pack::PACK_PLUGINS_INVALID,
            "some plugins of the pack are invalid; nothing was installed",
            json!({ "plugins": failures }),
        )
        .into());
    }
    let provisions = crate::provision::plan(
        &state,
        scope,
        &pack.manifest,
        &requirements.modules_to_enable,
    )
    .await?;
    if provisions.iter().any(|provision| provision.error.is_some()) {
        return Err(AppError::with_details(
            aphel_kernel::errors::CodedKind::Validation,
            crate::pack::PACK_INVALID,
            "the pack needs unavailable provisioners; nothing was installed",
            json!({ "provisions": provisions }),
        )
        .into());
    }
    let ready: Vec<(PreparedInstall, Vec<String>)> = prepared
        .into_iter()
        .map(|outcome| outcome.expect("checked"))
        .collect();
    let admitted =
        params.acknowledge_web_version.as_deref() == Some(pack.manifest.version.as_str());
    for (plan, _) in &ready {
        if pack_action(plan) == ACTION_UNCHANGED {
            continue;
        }
        if plan.package.manifest.web.is_some() && !admitted {
            return Err(AppError::conflict_code(
                "plugin_web_unacknowledged",
                "the pack has integrated frontend code; send acknowledge_web_version=<pack version> after reviewing its session-level access",
            )
            .into());
        }
        crate::conflicts::require_acknowledged(&plan.conflicts, params.acknowledge_conflicts)?;
        crate::modules::ensure_unblocked(&plan.module_conflicts)?;
    }
    crate::provision::enable_modules(&state, scope, &requirements.modules_to_enable).await?;
    let pool = state.db()?;
    let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
        pool.begin().await.map_err(AppError::from)?,
    );
    store::save_pack(&mut tx, &pack.manifest, Some(scope.user_id())).await?;
    tx.commit().await.map_err(AppError::from)?;
    let mut results = Vec::with_capacity(ready.len());
    let mut failed = false;
    for (member, (plan, own)) in pack.members.iter().zip(&ready) {
        let manifest = &member.package.manifest;
        let action = pack_action(plan);
        if failed {
            results.push(PackPluginResultResponse {
                key: manifest.key.clone(),
                version: manifest.version.clone(),
                action,
                status: "skipped",
                error: None,
            });
            continue;
        }
        let outcome: Result<&'static str, AppError> = async {
            if action == ACTION_UNCHANGED {
                store::set_pack(&pool, &manifest.key, &pack.manifest.key).await?;
            } else {
                apply_install(
                    &state,
                    scope,
                    client.0.ip.clone(),
                    plan,
                    own,
                    Some(&pack.manifest.key),
                )
                .await?;
            }
            apply_pack_settings(&state, scope, client.0.ip.clone(), member).await?;
            Ok(match action {
                ACTION_INSTALL => "installed",
                ACTION_UPGRADE => "upgraded",
                _ => "unchanged",
            })
        }
        .await;
        match outcome {
            Ok(status) => results.push(PackPluginResultResponse {
                key: manifest.key.clone(),
                version: manifest.version.clone(),
                action,
                status,
                error: None,
            }),
            Err(error) => {
                failed = true;
                results.push(PackPluginResultResponse {
                    key: manifest.key.clone(),
                    version: manifest.version.clone(),
                    action,
                    status: "failed",
                    error: Some(crate::pack::PackPluginError::from_error(&error)),
                });
            }
        }
    }
    let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
        pool.begin().await.map_err(AppError::from)?,
    );
    record_in(
        &mut tx,
        &state,
        scope,
        "plugin.pack_installed",
        &pack.manifest.key,
        vec![
            FieldChange::new(
                "version",
                json!(previous_pack.map(|existing| existing.version)),
                json!(pack.manifest.version),
            ),
            FieldChange::new(
                "plugins",
                Value::Null,
                json!(
                    results
                        .iter()
                        .map(|result| json!({ "plugin": result.key, "status": result.status }))
                        .collect::<Vec<_>>()
                ),
            ),
        ],
        client.0.ip,
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    registry::invalidate().await;
    let provisions = if failed {
        Vec::new()
    } else {
        let (provisions, provision_failed) =
            crate::provision::run(&state, scope, &pack.manifest).await?;
        failed = provision_failed;
        provisions
    };
    let status = if failed {
        StatusCode::MULTI_STATUS
    } else {
        StatusCode::CREATED
    };
    Ok((
        status,
        Json(Data::new(PluginPackResponse {
            key: pack.manifest.key.clone(),
            name: pack.manifest.name.clone(),
            version: pack.manifest.version.clone(),
            description: pack.manifest.description.clone(),
            complete: !failed,
            plugins: results,
            modules_enabled: requirements.modules_to_enable,
            provisions,
        })),
    )
        .into_response())
}

async fn apply_pack_settings(
    state: &AppState,
    scope: &AccessScope,
    ip: Option<String>,
    member: &crate::pack::PackMember,
) -> Result<(), AppError> {
    if member.settings.is_empty() {
        return Ok(());
    }
    let plugin = installed(state, &member.package.manifest.key).await?;
    let pending = pending_settings(member, Some(&plugin));
    if pending.is_empty() {
        return Ok(());
    }
    let mut settings = plugin.settings.clone();
    settings.extend(pending);
    let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
        state.db()?.begin().await.map_err(AppError::from)?,
    );
    store::set_settings(&mut tx, &plugin.key, &settings).await?;
    record_in(
        &mut tx,
        state,
        scope,
        "plugin.settings_updated",
        &plugin.key,
        vec![FieldChange::new(
            "settings",
            Value::Object(plugin.settings),
            Value::Object(settings),
        )],
        ip,
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(())
}

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct InstallParams {
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    acknowledge_conflicts: bool,
    #[serde(default)]
    acknowledge_web_version: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
pub(crate) struct ToggleParams {
    #[serde(default)]
    acknowledge_conflicts: bool,
    #[serde(default)]
    acknowledge_web_version: Option<String>,
}

async fn package_bytes(
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
    match package {
        Some(package) => Ok((package, presets)),
        None => Err(AppError::invalid_fields(
            "invalid form",
            vec![FieldViolation::new("package", "is required")],
        )),
    }
}

#[utoipa::path(
    post,
    path = "/wasm-runtime/{key}/grants",
    tag = TAG,
    operation_id = "plugins_reapply_grants",
    security(("session_cookie" = [])),
    description = "Reapplies the plugin's declared grants (permission `system.manage`): permissions with `grant_to`, role presets and entity `permission_scope` defaults are granted again to the roles that exist now. Roles are never created here; only grants missing from existing preset roles are added. Publishes `plugin.grants_reapplied` with an audit entry in the same transaction as the permission writes. The response reports, per role, how many of the expected permissions are granted.",
    params(("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Grants reapplied", body = Data<PluginResponse>),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn reapply_plugin_grants(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<Json<Data<PluginResponse>>, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let plugin = installed(&state, &key).await?;
    let mut tx = MySqlTx::from(state.db()?.begin().await.map_err(AppError::from)?);
    let report = grants::reconcile(&mut tx, &plugin.manifest, &[]).await?;
    record_in(
        &mut tx,
        &state,
        &tenant.scope,
        "plugin.grants_reapplied",
        &key,
        grant_changes(&report),
        client.0.ip,
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    registry::invalidate().await;
    Ok(Json(Data::new(
        respond(&state, installed(&state, &key).await?).await?,
    )))
}

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct ChangesParams {
    #[serde(default)]
    since: u64,
    #[serde(default)]
    limit: Option<u32>,
}

async fn enabled_plugin(state: &AppState, key: &str) -> Result<(), AppError> {
    let plugin = installed(state, key).await?;
    if !plugin.enabled {
        return Err(not_found(key));
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/wasm-runtime/{key}/changes",
    tag = TAG,
    operation_id = "plugins_changes",
    security(("session_cookie" = [])),
    description = "Change feed of a plugin: every create, update, delete, archive and restore of its entities and every `plugin.<key>.*` event, in commit order, with a monotonic `seq`. Pass the last `next` as `since`. Entries of record types the caller cannot read are skipped (the cursor still advances). `fields` lists the changed field names, never their values; read the record to get them. The feed keeps 24 hours; `reset = true` means entries after `since` were pruned and the client must reload its state.",
    params(("key" = String, Path, description = "Plugin key"), ChangesParams),
    responses(
        (status = 200, description = "Changes after `since`", body = crate::changes::ChangesPage),
        (status = 404, description = "Plugin not installed or disabled (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn plugin_changes(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
    Query(params): Query<ChangesParams>,
) -> Result<Json<crate::changes::ChangesPage>, ApiError> {
    enabled_plugin(&state, &key).await?;
    Ok(Json(
        crate::changes::page(
            &tenant.scope,
            &key,
            params.since,
            params.limit.unwrap_or(crate::changes::MAX_PAGE),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/wasm-runtime/{key}/changes/stream",
    tag = TAG,
    operation_id = "plugins_change_stream",
    security(("session_cookie" = [])),
    description = "The change feed as Server-Sent Events: each entry is an event `change` with the JSON of `GET /wasm-runtime/{key}/changes` and `id` = `seq`; `reset` asks the client to reload. Resumes from `since` or the `Last-Event-ID` header. The server closes the stream after 5 minutes; `EventSource` reconnects with `Last-Event-ID`.",
    params(("key" = String, Path, description = "Plugin key"), ChangesParams),
    responses(
        (status = 200, description = "text/event-stream of changes"),
        (status = 404, description = "Plugin not installed or disabled (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn plugin_change_stream(
    State(state): State<AppState>,
    tenant: AuthContext,
    headers: HeaderMap,
    Path(key): Path<String>,
    Query(params): Query<ChangesParams>,
) -> Result<Response, ApiError> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use futures::StreamExt;
    enabled_plugin(&state, &key).await?;
    let since = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(params.since);
    let scope = tenant.scope.clone();
    let started = std::time::Instant::now();
    let stream = futures::stream::unfold((since, true), move |(cursor, first)| {
        let scope = scope.clone();
        let key = key.clone();
        async move {
            if started.elapsed() > crate::changes::STREAM_LIFETIME {
                return None;
            }
            if !first {
                tokio::time::sleep(crate::changes::STREAM_POLL).await;
            }
            let (next, events) = match crate::changes::page(&scope, &key, cursor, 200).await {
                Ok(page) => {
                    let mut events = Vec::with_capacity(page.data.len() + 1);
                    if page.reset {
                        events.push(Event::default().event("reset").data("{}"));
                    }
                    for change in &page.data {
                        events.push(
                            Event::default()
                                .event("change")
                                .id(change.seq.to_string())
                                .data(crate::changes::as_event(change).to_string()),
                        );
                    }
                    (page.next, events)
                }
                Err(error) => (
                    cursor,
                    vec![Event::default().event("error").data(error.client_message())],
                ),
            };
            Some((
                futures::stream::iter(events.into_iter().map(Ok::<_, std::convert::Infallible>)),
                (next, false),
            ))
        }
    })
    .flatten();
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response())
}

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct UninstallParams {
    #[serde(default)]
    purge: bool,
}

#[utoipa::path(
    delete,
    path = "/wasm-runtime/{key}",
    tag = TAG,
    operation_id = "plugins_uninstall",
    security(("session_cookie" = [])),
    description = "Uninstalls a plugin (permission `system.manage`): its code, web bundle and key-value data are removed. With `purge=true` its entities (and their records) and its fields on other record types are deleted too; otherwise they are only deactivated and come back if the plugin is installed again. Publishes `plugin.uninstalled`.",
    params(("key" = String, Path, description = "Plugin key"), UninstallParams),
    responses(
        (status = 204, description = "Plugin uninstalled"),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn uninstall_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
    Query(params): Query<UninstallParams>,
) -> Result<StatusCode, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let plugin = installed(&state, &key).await?;
    crate::dependents::ensure_none_broken(&state, &key, None).await?;
    crate::modules::release(&state, &tenant.scope, &key).await;
    if params.purge {
        schema::purge(&state, &plugin.manifest).await?;
    } else {
        schema::set_active(&state, &plugin.manifest, false).await?;
    }
    let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
        state.db()?.begin().await.map_err(AppError::from)?,
    );
    store::delete(&mut tx, &key).await?;
    clear_ui_plugin(&mut tx, &key).await?;
    remove_runtime_permissions(&mut tx, &permissions_owner(&key)).await?;
    // Remove all credentials, including declarations removed by a plugin update.
    for owner in [
        crate::host::SECRET_OWNER_TYPE,
        "plugin_tls",
        "plugin_http",
        "plugin_signing",
    ] {
        state
            .secrets
            .remove_owner(&mut tx, &tenant.scope, owner, &key)
            .await?;
    }
    tx.commit().await.map_err(AppError::from)?;
    record(
        &state,
        &tenant.scope,
        "plugin.uninstalled",
        &key,
        vec![FieldChange::new(
            "version",
            json!(plugin.version),
            Value::Null,
        )],
        client.0.ip,
    )
    .await?;
    if let Err(error) = prune_compiled(&state).await {
        tracing::warn!(error = %error, "could not prune precompiled plugin backends");
    }
    Ok(StatusCode::NO_CONTENT)
}

fn require_web_admission(
    manifest: &PluginManifest,
    acknowledged: Option<&str>,
) -> Result<(), AppError> {
    if manifest.web.is_some() && acknowledged != Some(manifest.version.as_str()) {
        return Err(AppError::conflict_code(
            "plugin_web_unacknowledged",
            "integrated frontend code requires explicit admission for this version; send acknowledge_web_version=<manifest version> after reviewing its session-level access",
        ));
    }
    Ok(())
}

async fn toggle(
    state: &AppState,
    tenant: &AuthContext,
    client: ClientInfo,
    key: &str,
    enabled: bool,
    params: ToggleParams,
) -> Result<PluginResponse, AppError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let plugin = installed(state, key).await?;
    if plugin.enabled != enabled {
        if !enabled {
            crate::dependents::ensure_none_broken(state, key, None).await?;
        }
        if enabled {
            require_web_admission(&plugin.manifest, params.acknowledge_web_version.as_deref())?;
            schema::validate_dependencies(state, &plugin.manifest).await?;
            let conflicts = crate::conflicts::detect_for(&state.db()?, &plugin.manifest).await?;
            crate::conflicts::require_acknowledged(&conflicts, params.acknowledge_conflicts)?;
            let plans = crate::modules::plan(state, &tenant.scope, &plugin.manifest).await?;
            crate::modules::ensure_unblocked(&plans)?;
            crate::modules::hold(state, &tenant.scope, &plugin.manifest).await?;
        }
        let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
            state.db()?.begin().await.map_err(AppError::from)?,
        );
        store::set_enabled(&mut tx, key, enabled).await?;
        set_runtime_permissions_active(&mut tx, &permissions_owner(key), enabled).await?;
        tx.commit().await?;
        schema::set_active(state, &plugin.manifest, enabled).await?;
        if !enabled {
            crate::modules::release(state, &tenant.scope, key).await;
        }
        record(
            state,
            &tenant.scope,
            if enabled {
                "plugin.enabled"
            } else {
                "plugin.disabled"
            },
            key,
            vec![FieldChange::new("enabled", json!(!enabled), json!(enabled))],
            client.0.ip,
        )
        .await?;
    }
    respond(state, installed(state, key).await?).await
}

#[utoipa::path(
    post,
    path = "/wasm-runtime/{key}/enable",
    tag = TAG,
    operation_id = "plugins_enable",
    security(("session_cookie" = [])),
    description = "Enables an installed plugin (permission `system.manage`): its entities, fields, hooks, events, routes and web bundle come back. Integrated frontend code requires acknowledge_web_version=<manifest version> before reactivation. Publishes `plugin.enabled`.",
    params(("key" = String, Path, description = "Plugin key"), ToggleParams),
    responses(
        (status = 200, description = "Plugin enabled", body = Data<PluginResponse>),
        (status = 409, description = "Integrated frontend code was not admitted (`plugin_web_unacknowledged`), or the plugin overlaps with another enabled plugin and the overlap was not acknowledged (`plugin_conflicts_unacknowledged`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn enable_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
    Query(params): Query<ToggleParams>,
) -> Result<Json<Data<PluginResponse>>, ApiError> {
    Ok(Json(Data::new(
        toggle(&state, &tenant, client, &key, true, params).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/wasm-runtime/{key}/disable",
    tag = TAG,
    operation_id = "plugins_disable",
    security(("session_cookie" = [])),
    description = "Disables a plugin without removing anything (permission `system.manage`): its entities and fields are hidden and its hooks, events, routes and web bundle stop. Publishes `plugin.disabled`.",
    params(("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Plugin disabled", body = Data<PluginResponse>),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn disable_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<Json<Data<PluginResponse>>, ApiError> {
    Ok(Json(Data::new(
        toggle(
            &state,
            &tenant,
            client,
            &key,
            false,
            ToggleParams::default(),
        )
        .await?,
    )))
}

fn validate_settings(
    manifest: &PluginManifest,
    values: &Map<String, Value>,
) -> Result<Map<String, Value>, AppError> {
    let mut violations = Vec::new();
    let mut accepted = Map::new();
    for (key, value) in values {
        let Some(setting) = manifest.settings.iter().find(|setting| &setting.key == key) else {
            violations.push(FieldViolation::new(key, "is not a setting of this plugin"));
            continue;
        };
        if crate::manifest::is_sensitive_setting(&setting.kind) {
            continue;
        }
        let accepted_value = match setting.kind.as_str() {
            "number" => value
                .as_i64()
                .is_some_and(|number| {
                    setting.min.is_none_or(|min| number >= min)
                        && setting.max.is_none_or(|max| number <= max)
                })
                .then(|| value.clone()),
            "boolean" => value.is_boolean().then(|| value.clone()),
            crate::manifest::SETTING_SELECT => value
                .as_str()
                .is_some_and(|text| setting.options.iter().any(|option| option.value == text))
                .then(|| value.clone()),
            _ => value
                .as_str()
                .and_then(|text| setting.normalize_text(text))
                .map(Value::String),
        };
        if value.is_null() {
            accepted.insert(key.clone(), Value::Null);
        } else if let Some(accepted_value) = accepted_value {
            accepted.insert(key.clone(), accepted_value);
        } else {
            violations.push(FieldViolation::new(
                key,
                format!(
                    "must be a valid {}",
                    setting.mask.as_deref().unwrap_or(&setting.kind)
                ),
            ));
        }
    }
    if violations.is_empty() {
        Ok(accepted)
    } else {
        Err(AppError::invalid_fields_code(
            INVALID_SETTINGS,
            "invalid plugin settings",
            violations,
        ))
    }
}

fn require_settings_access(tenant: &AuthContext, plugin: &InstalledPlugin) -> Result<(), AppError> {
    if tenant.scope.require(&Permission::SYSTEM_MANAGE).is_ok() {
        return Ok(());
    }
    match &plugin.manifest.settings_permission {
        Some(permission) => {
            tenant
                .scope
                .require(&Permission::parse(permission.clone())?)?;
            Ok(())
        }
        None => tenant.scope.require(&Permission::SYSTEM_MANAGE).map(|_| ()),
    }
}

#[utoipa::path(
    get,
    path = "/wasm-runtime/{key}/settings",
    tag = TAG,
    operation_id = "plugins_settings",
    security(("session_cookie" = [])),
    description = "Lists the settings of a plugin with their current values (permission `system.manage` or the plugin's `settings_permission`). Secret settings only report `configured`.",
    params(("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Settings", body = Data<Vec<PluginSettingResponse>>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_plugin_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<Vec<PluginSettingResponse>>>, ApiError> {
    let plugin = installed(&state, &key).await?;
    require_settings_access(&tenant, &plugin)?;
    Ok(Json(Data::new(setting_responses(&state, &plugin).await?)))
}

#[utoipa::path(
    put,
    path = "/wasm-runtime/{key}/settings",
    tag = TAG,
    operation_id = "plugins_update_settings",
    security(("session_cookie" = [])),
    description = "Replaces the settings of a plugin (permission `system.manage` or the plugin's `settings_permission`), validated against the settings it declares. `null` restores the default. Secret settings are stored encrypted and never returned (only `configured`): a string sets the secret, `null` removes it and leaving the key out keeps it. Publishes `plugin.settings_updated`.",
    params(("key" = String, Path, description = "Plugin key")),
    request_body(content = Object, description = "Map of setting key to value"),
    responses(
        (status = 200, description = "Settings saved", body = Data<PluginResponse>),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid settings (`plugin_settings_invalid`)", body = ErrorEnvelope),
    )
)]
async fn update_plugin_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
    Json(values): Json<Map<String, Value>>,
) -> Result<Json<Data<PluginResponse>>, ApiError> {
    let plugin = installed(&state, &key).await?;
    require_settings_access(&tenant, &plugin)?;
    let mut settings = validate_settings(&plugin.manifest, &values)?;
    settings.retain(|_, value| !value.is_null());
    let mut secrets = Vec::new();
    let mut violations = Vec::new();
    for setting in plugin.manifest.secret_settings() {
        match values.get(&setting.key) {
            None => {}
            Some(Value::Null) => secrets.push((setting.key.clone(), None)),
            Some(Value::String(text)) if !text.is_empty() && text.len() <= MAX_SECRET_BYTES => {
                if setting.kind == crate::manifest::SETTING_TLS_IDENTITY {
                    crate::net::parse_identity(text)?;
                }
                if matches!(
                    setting.kind.as_str(),
                    crate::manifest::SETTING_TLS_IDENTITY
                        | crate::manifest::SETTING_SIGNING_IDENTITY
                ) && !setting.signing_algorithms.is_empty()
                {
                    crate::signing::validate_identity(text, &setting.signing_algorithms)?;
                }
                if setting.kind == crate::manifest::SETTING_HTTP_CREDENTIAL {
                    crate::net::credential_header(text)?;
                }
                secrets.push((setting.key.clone(), Some(Secret::new(text.clone()))))
            }
            Some(_) => violations.push(FieldViolation::new(
                &setting.key,
                "must be a non-empty text or null",
            )),
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            INVALID_SETTINGS,
            "invalid plugin settings",
            violations,
        )
        .into());
    }
    let mut changes = vec![FieldChange::new(
        "settings",
        Value::Object(plugin.settings),
        Value::Object(settings.clone()),
    )];
    let mut tx = aphel_sdk::platform::database::transaction::MySqlTx::from(
        state.db()?.begin().await.map_err(AppError::from)?,
    );
    store::set_settings(&mut tx, &key, &settings).await?;
    for (name, secret) in &secrets {
        let setting = plugin
            .manifest
            .settings
            .iter()
            .find(|setting| &setting.key == name)
            .expect("validated setting");
        let secret_key = setting_secret_key(&key, setting);
        match secret {
            Some(secret) => {
                state
                    .secrets
                    .store(&mut tx, &tenant.scope, &secret_key, secret)
                    .await?
            }
            None => {
                state
                    .secrets
                    .remove(&mut tx, &tenant.scope, &secret_key)
                    .await?
            }
        }
        changes.push(FieldChange::new(
            format!("secrets.{name}"),
            json!(REDACTED),
            json!(REDACTED),
        ));
    }
    tx.commit().await.map_err(AppError::from)?;
    record(
        &state,
        &tenant.scope,
        "plugin.settings_updated",
        &key,
        changes,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        respond(&state, installed(&state, &key).await?).await?,
    )))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WebPluginResponse {
    key: String,
    name: String,
    version: String,
    menu: crate::manifest::MenuSpec,
    role_homes: std::collections::BTreeMap<String, String>,
    ui: bool,
    entry: Option<String>,
    style: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
struct WebPluginsQuery {
    ui: Option<String>,
}

#[utoipa::path(
    get,
    path = "/public/wasm-runtime",
    tag = TAG,
    operation_id = "plugins_web",
    description = "Lists the web bundles of the enabled plugins, loaded by the Aphel web app before it builds its routes. Plugins without a bundle are listed too when they declare `[menu]` or a role preset `home`, with `entry` null, so the web app applies their menu and role homes. Plugins that declare an interface (`[ui]`) are listed only when they are the one requested in `ui` (an enabled interface plugin) or, without a valid `ui`, the organization default. Public: plugin bundles are code, not data. The `x-aphel-ui-plugin` response header names the interface plugin that applies, when there is one; `ui=core` asks for none.",
    params(WebPluginsQuery),
    responses(
        (status = 200, description = "Web bundles to load", body = Data<Vec<WebPluginResponse>>),
    )
)]
async fn web_plugins(
    State(state): State<AppState>,
    Query(query): Query<WebPluginsQuery>,
) -> Result<Response, ApiError> {
    let pool = state.db()?;
    let plugins = store::list(&pool).await?;
    let chosen_ui = match query.ui.as_deref() {
        Some(CORE_UI_PLUGIN) => None,
        requested => {
            let valid = requested.filter(|key| {
                plugins.iter().any(|plugin| {
                    plugin.enabled && plugin.key == *key && plugin.manifest.ui.is_some()
                })
            });
            match valid {
                Some(key) => Some(key.to_owned()),
                None => find_ui_plugin_default(&pool).await?,
            }
        }
    };
    let chosen_ui = chosen_ui.filter(|key| {
        plugins
            .iter()
            .any(|plugin| plugin.enabled && plugin.key == *key && plugin.manifest.ui.is_some())
    });
    let asset = |plugin: &InstalledPlugin, path: &str| {
        format!(
            "/api/v1/public/wasm-runtime/{}/{path}?v={}",
            plugin.key,
            &plugin.checksum[..12]
        )
    };
    let listed: Vec<WebPluginResponse> = plugins
        .iter()
        .filter(|plugin| plugin.enabled)
        .filter(|plugin| {
            plugin.manifest.ui.is_none() || chosen_ui.as_deref() == Some(plugin.key.as_str())
        })
        .filter_map(|plugin| {
            let web = plugin.manifest.web.as_ref();
            let role_homes: std::collections::BTreeMap<String, String> = plugin
                .manifest
                .role_presets
                .iter()
                .filter_map(|preset| Some((preset.key.clone(), preset.home.clone()?)))
                .collect();
            if web.is_none() && plugin.manifest.menu.is_empty() && role_homes.is_empty() {
                return None;
            }
            Some(WebPluginResponse {
                key: plugin.key.clone(),
                name: plugin.manifest.name.clone(),
                version: plugin.version.clone(),
                menu: plugin.manifest.menu.clone(),
                role_homes,
                ui: plugin.manifest.ui.is_some(),
                entry: web.map(|web| asset(plugin, &web.entry)),
                style: web
                    .and_then(|web| web.style.as_deref())
                    .map(|style| asset(plugin, style)),
            })
        })
        .collect();
    let mut response = Json(Data::new(listed)).into_response();
    if let Some(key) = chosen_ui.as_deref()
        && let Ok(value) = HeaderValue::from_str(key)
    {
        response
            .headers_mut()
            .insert(HeaderName::from_static(UI_PLUGIN_HEADER), value);
    }
    Ok(response)
}

async fn web_asset(
    State(state): State<AppState>,
    Path((key, path)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    if !path.starts_with(WEB_PREFIX) {
        return Err(not_found(&key).into());
    }
    let Some((content_type, content)) = store::file(&state.db()?, &key, &path).await? else {
        return Err(not_found(&key).into());
    };
    let mut response = Body::from(content).into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(PUBLIC_APP_CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn public_app_redirect(Path(key): Path<String>) -> Result<Response, ApiError> {
    if !key
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(not_found(&key).into());
    }
    Ok(axum::response::Redirect::permanent(&format!("{key}/")).into_response())
}

async fn public_app_index(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Response, ApiError> {
    Ok(serve_public_app(&state, &key, "").await?)
}

async fn public_app_file(
    State(state): State<AppState>,
    Path((key, path)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    Ok(serve_public_app(&state, &key, &path).await?)
}

async fn serve_public_app(state: &AppState, key: &str, path: &str) -> Result<Response, AppError> {
    let plugin = installed(state, key).await?;
    let root = plugin
        .manifest
        .web
        .as_ref()
        .and_then(|web| web.public.as_deref())
        .filter(|_| plugin.enabled)
        .ok_or_else(|| not_found(key))?
        .trim_end_matches('/')
        .to_owned();
    let pool = state.db()?;
    let index = format!("{root}/{PUBLIC_APP_INDEX}");
    let requested = (!path.is_empty()).then(|| format!("{root}/{path}"));
    let found = match &requested {
        Some(requested) => store::file(&pool, key, requested).await?,
        None => None,
    };
    let is_route = path
        .rsplit('/')
        .next()
        .is_none_or(|name| !name.contains('.'));
    let (is_index, file) = match found {
        Some(file) => (false, file),
        None if is_route => (
            true,
            store::file(&pool, key, &index)
                .await?
                .ok_or_else(|| not_found(key))?,
        ),
        None => return Err(not_found(key)),
    };
    let (content_type, content) = file;
    let mut response = Body::from(content).into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if is_index {
            "no-cache"
        } else {
            "public, max-age=3600"
        }),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(PUBLIC_APP_CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    Ok(response)
}

pub(crate) struct RouteCall {
    pub(crate) method: Method,
    pub(crate) key: String,
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    pub(crate) customer: Option<aphel_contract_site::SiteCustomer>,
}

const PLUGIN_RESPONSE_HEADERS: [&str; 6] = [
    "cache-control",
    "etag",
    "last-modified",
    "location",
    "vary",
    "content-language",
];
const PRIVATE_NO_STORE: &str = "private, no-store";

fn apply_plugin_headers(response: &mut Response, declared: &[(String, String)], personal: bool) {
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(PRIVATE_NO_STORE),
    );
    for (name, value) in declared {
        let name = name.to_ascii_lowercase();
        if !PLUGIN_RESPONSE_HEADERS.contains(&name.as_str())
            || (name == "cache-control" && personal)
            || (name == "location" && (!value.starts_with('/') || value.starts_with("//")))
        {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            header::HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            headers.insert(name, value);
        }
    }
}

pub(crate) async fn serve_route(
    state: &AppState,
    call: RouteCall,
    caller: Option<&AccessScope>,
    audience: crate::manifest::RouteAudience,
) -> Result<Response, AppError> {
    let RouteCall {
        method,
        key,
        path,
        query,
        headers,
        body,
        customer,
    } = call;
    let personal = customer.is_some();
    let snapshot = registry::current(&state.db()?).await?;
    let plugin = snapshot.plugin(&key).cloned().ok_or_else(|| {
        AppError::not_found_code(
            PLUGIN_NOT_FOUND,
            format!("the plugin {key} is not installed or is disabled"),
        )
    })?;
    let route_missing = || {
        AppError::not_found_code(
            PLUGIN_ROUTE_NOT_FOUND,
            format!("the plugin {key} has no route {method} /{path}"),
        )
    };
    let (route, params) = match_route(&plugin.manifest.routes, audience, method.as_str(), &path)
        .ok_or_else(route_missing)?;
    if let (Some(permission), Some(scope)) = (&route.permission, caller) {
        let permission = Permission::parse(permission.clone()).map_err(|_| {
            AppError::Configuration(format!("the plugin {key} declares an invalid permission"))
        })?;
        scope.require(&permission)?;
    }
    let body_json = if body.is_empty() {
        Value::Null.to_string()
    } else {
        serde_json::from_slice::<Value>(&body)
            .map_err(|_| AppError::Validation("the body must be JSON".to_owned()))?
            .to_string()
    };
    let (actor, scope) = match caller {
        Some(scope) => (
            runtime::scope_actor(scope),
            runtime::capability_scope(&plugin, scope),
        ),
        None => (
            runtime::anonymous_actor(),
            runtime::plugin_scope(&plugin).await?,
        ),
    };
    let request = wit::Request {
        method: method.as_str().to_owned(),
        path: format!("/{path}"),
        route: route.path.clone(),
        params: params
            .into_iter()
            .map(|(name, value)| (name, value.as_str().unwrap_or_default().to_owned()))
            .collect(),
        query: query.as_deref().map(form_pairs).unwrap_or_default(),
        headers: forwarded_headers(
            &headers,
            if caller.is_none() {
                &route.forward_headers
            } else {
                &[]
            },
        ),
        body_json,
        body_raw: body.to_vec(),
        actor,
        customer: customer.map(|customer| wit::Customer {
            id: customer.id,
            email: customer.email,
            name: customer.name,
            email_verified: customer.email_verified,
        }),
    };
    let result = runtime::on_route(&plugin, scope, &request).await?;
    let status = StatusCode::from_u16(result.response.status).unwrap_or(StatusCode::OK);
    if let Some(raw) = result.raw_body {
        let mut response = (status, raw.bytes).into_response();
        let headers = response.headers_mut();
        if let Ok(value) = HeaderValue::from_str(&raw.content_type) {
            headers.insert(header::CONTENT_TYPE, value);
        }
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        if let Some(filename) = raw.filename
            && let Ok(value) =
                HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }
        apply_plugin_headers(&mut response, &result.response.headers, personal);
        return Ok(response);
    }
    let body: Value = serde_json::from_str(&result.response.body_json).unwrap_or(Value::Null);
    let mut response = (status, Json(body)).into_response();
    apply_plugin_headers(&mut response, &result.response.headers, personal);
    Ok(response)
}

async fn dispatch(
    State(state): State<AppState>,
    tenant: AuthContext,
    method: Method,
    Path((key, path)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let call = RouteCall {
        method,
        key,
        path,
        query,
        headers,
        body,
        customer: None,
    };
    Ok(serve_route(
        &state,
        call,
        Some(&tenant.scope),
        crate::manifest::RouteAudience::Private,
    )
    .await?)
}

static PUBLIC_LIMITS: LazyLock<(SharedRateLimiter, SharedRateLimiter)> = LazyLock::new(|| {
    (
        SharedRateLimiter::new(
            "wasm_runtime.public_per_address",
            PUBLIC_PER_ADDRESS_LIMIT,
            MINUTE,
        ),
        SharedRateLimiter::new(
            "wasm_runtime.public_per_plugin",
            PUBLIC_PER_PLUGIN_LIMIT,
            MINUTE,
        ),
    )
});

pub(crate) async fn check_public_limits(key: &str, address: &str) -> Result<(), AppError> {
    let (per_address, per_plugin) = &*PUBLIC_LIMITS;
    let message = "muitas requisições em sequência; aguarde um pouco e tente novamente";
    per_address
        .check(&format!("{key}:{address}"), message)
        .await?;
    per_plugin.check(key, message).await
}

async fn public_dispatch(
    State(state): State<AppState>,
    client: ClientInfo,
    method: Method,
    Path((key, path)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    check_public_limits(&key, client.ip_or_unknown()).await?;
    let call = RouteCall {
        method,
        key,
        path,
        query,
        headers,
        body,
        customer: None,
    };
    Ok(serve_route(&state, call, None, crate::manifest::RouteAudience::Public).await?)
}

fn forwarded_headers(headers: &HeaderMap, forwarded: &[String]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| {
            !WITHHELD_HEADERS.contains(name)
                || forwarded
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(name.as_str()))
        })
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect()
}

fn form_pairs(query: &str) -> Vec<(String, String)> {
    form_urlencoded::parse(query.as_bytes())
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}
