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
use crate::host::secret_key;
use crate::manifest::{HostCatalog, PluginManifest, SETTING_SECRET, match_route};
use crate::package::{MAX_PACKAGE_BYTES, PUBLIC_APP_INDEX, read_package};
use crate::registry;
use crate::runtime::{self, wit};
use crate::schema;
use crate::store::{self, InstalledPlugin};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo};
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::secrets::MAX_SECRET_BYTES;
use aphel_kernel::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::Secret;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::repositories::runtime_permissions::{
    RuntimeGrant, remove_runtime_permissions, replace_runtime_permissions,
    set_runtime_permissions_active,
};
use aphel_kernel::modules::builder::APHEL_VERSION;

pub const TAG: &str = "plugins";
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

pub(crate) fn root_router() -> OpenApiRouter<AppState> {
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
    let mut tx = aphel_kernel::database::transaction::MySqlTx::from(
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
    role_presets: Vec<PluginRolePresetResponse>,
    entity_scopes: Vec<PluginEntityScopeResponse>,
    unique_fields: Vec<String>,
    menu: crate::manifest::MenuSpec,
    conflicts_with: Vec<String>,
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
        let routes = |public: bool| {
            manifest
                .routes
                .iter()
                .filter(|route| route.public == public)
                .map(|route| format!("{} {}", route.method, route.path))
                .collect()
        };
        Self {
            entities: manifest.entity_record_types(),
            extended_types: extended,
            hooks: manifest
                .hooks
                .iter()
                .map(|hook| format!("{} ({})", hook.hook, hook.record_types.join(", ")))
                .collect(),
            events: manifest.events.clone(),
            routes: routes(false),
            public_routes: routes(true)
                .into_iter()
                .chain(
                    manifest
                        .web
                        .as_ref()
                        .and_then(|web| web.public.as_ref())
                        .map(|_| format!("GET /p/{}/", manifest.key)),
                )
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
            conflicts_with: manifest.conflicts_with.clone(),
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
        let configured = if setting.kind == SETTING_SECRET {
            Some(
                state
                    .secrets
                    .reveal(
                        &AccessScope::system(),
                        &secret_key(&plugin.key, &setting.key),
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
            value: values.get(&setting.key).cloned().unwrap_or(Value::Null),
            configured,
        });
    }
    Ok(settings)
}

async fn respond(state: &AppState, plugin: InstalledPlugin) -> Result<PluginResponse, AppError> {
    let snapshot = registry::current(&state.db()?).await?;
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
    description = "Installs or upgrades a plugin from an `.aphelplugin` package sent as the multipart field `package` (permission `system.manage`). The package is a zip with `plugin.toml`, an optional `backend.wasm` built with aphel-plugin-sdk and an optional `web/` bundle. Declared entities become custom objects managed by the plugin, declared fields become managed custom fields, declared permissions enter the role catalog, and hooks, events, schedules and routes start working without restarting Aphel. The plugin acts with what it `requires` intersected with the permissions of whoever installs it, so the installer gives consent: call first with `dry_run=true` to get what the package asks for (status 200, nothing is installed). Publishes `plugin.installed`.",
    params(InstallParams),
    request_body(content_type = "multipart/form-data", description = "Field `package` with the .aphelplugin file"),
    responses(
        (status = 200, description = "Dry run: what the package declares and asks for", body = Data<PluginConsentResponse>),
        (status = 201, description = "Plugin installed or upgraded", body = Data<PluginResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "A custom object, field or permission with the same key already exists (`plugin_object_conflict`, `plugin_field_conflict`, `permission_already_registered`), or the plugin overlaps with another installed plugin and `acknowledge_conflicts` was not sent (`plugin_conflicts_unacknowledged`)", body = ErrorEnvelope),
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
    let compiled: Vec<String> = state
        .app
        .modules()
        .iter()
        .map(|module| module.manifest.key.to_owned())
        .collect();
    let catalog = crate::bridge::host_catalog();
    package.manifest.validate(
        &compiled,
        APHEL_VERSION,
        HostCatalog {
            hooks: &catalog.hooks,
            operations: &catalog.operations,
        },
    )?;
    if package.manifest.needs_backend() && package.backend().is_none() {
        return Err(AppError::validation_code(
            "plugin_backend_missing",
            "the plugin declares runtime callbacks but has no backend.wasm",
        )
        .into());
    }
    if let Some(backend) = package.backend() {
        runtime::validate(backend.content.clone()).await?;
    }
    let pool = state.db()?;
    let previous = store::find(&pool, &package.manifest.key).await?;
    let presets = grants::selected_presets(&package.manifest, &presets)?;
    schema::check(&state, &package.manifest).await?;
    let conflicts = crate::conflicts::detect_for(&pool, &package.manifest).await?;
    let module_conflicts = crate::modules::plan(&state, scope, &package.manifest).await?;
    if params.dry_run {
        let manifest = &package.manifest;
        let consent = PluginConsentResponse {
            key: manifest.key.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            installed_version: previous.map(|plugin| plugin.version),
            declares: PluginDeclarations::from(manifest),
            grants: grants::status(&pool, manifest).await?,
            conflicts,
            module_conflicts,
        };
        return Ok(Json(Data::new(consent)).into_response());
    }
    crate::conflicts::require_acknowledged(&conflicts, params.acknowledge_conflicts)?;
    crate::modules::ensure_unblocked(&module_conflicts)?;
    let enabled = previous.as_ref().is_none_or(|plugin| plugin.enabled);
    let held = if enabled {
        crate::modules::hold(&state, scope, &package.manifest).await?
    } else {
        Vec::new()
    };
    let outcome: Result<InstalledPlugin, AppError> = async {
        schema::apply(&state, scope, &package.manifest).await?;
        let mut tx = aphel_kernel::database::transaction::MySqlTx::from(
            pool.begin().await.map_err(AppError::from)?,
        );
        store::save(&mut tx, &package, Some(scope.user_id())).await?;
        sync_permissions(&mut tx, &package.manifest, enabled).await?;
        let report = grants::reconcile(&mut tx, &package.manifest, &presets).await?;
        let mut changes = vec![FieldChange::new(
            "version",
            json!(previous.map(|plugin| plugin.version)),
            json!(package.manifest.version),
        )];
        changes.extend(grant_changes(&report));
        record_in(
            &mut tx,
            &state,
            scope,
            "plugin.installed",
            &package.manifest.key,
            changes,
            client.0.ip,
        )
        .await?;
        tx.commit().await.map_err(AppError::from)?;
        registry::invalidate().await;
        if let Err(error) = prune_compiled(&state).await {
            tracing::warn!(error = %error, "could not prune precompiled plugin backends");
        }
        let plugin = installed(&state, &package.manifest.key).await?;
        if !plugin.enabled {
            schema::set_active(&state, &plugin.manifest, false).await?;
        }
        Ok(plugin)
    }
    .await;
    let plugin = match outcome {
        Ok(plugin) => plugin,
        Err(error) => {
            crate::modules::release_modules(&state, scope, &package.manifest.key, &held).await;
            return Err(error.into());
        }
    };
    Ok((
        StatusCode::CREATED,
        Json(Data::new(respond(&state, plugin).await?)),
    )
        .into_response())
}

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct InstallParams {
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    acknowledge_conflicts: bool,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
pub(crate) struct ToggleParams {
    #[serde(default)]
    acknowledge_conflicts: bool,
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
    crate::modules::release(&state, &tenant.scope, &key).await;
    if params.purge {
        schema::purge(&state, &plugin.manifest).await?;
    } else {
        schema::set_active(&state, &plugin.manifest, false).await?;
    }
    let mut tx = aphel_kernel::database::transaction::MySqlTx::from(
        state.db()?.begin().await.map_err(AppError::from)?,
    );
    store::delete(&mut tx, &key).await?;
    remove_runtime_permissions(&mut tx, &permissions_owner(&key)).await?;
    for setting in plugin.manifest.secret_settings() {
        state
            .secrets
            .remove(&mut tx, &tenant.scope, &secret_key(&key, &setting.key))
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

async fn toggle(
    state: &AppState,
    tenant: &AuthContext,
    client: ClientInfo,
    key: &str,
    enabled: bool,
    acknowledge_conflicts: bool,
) -> Result<PluginResponse, AppError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let plugin = installed(state, key).await?;
    if plugin.enabled != enabled {
        if enabled {
            schema::validate_dependencies(state, &plugin.manifest).await?;
            let conflicts = crate::conflicts::detect_for(&state.db()?, &plugin.manifest).await?;
            crate::conflicts::require_acknowledged(&conflicts, acknowledge_conflicts)?;
            let plans = crate::modules::plan(state, &tenant.scope, &plugin.manifest).await?;
            crate::modules::ensure_unblocked(&plans)?;
            crate::modules::hold(state, &tenant.scope, &plugin.manifest).await?;
        }
        let mut tx = aphel_kernel::database::transaction::MySqlTx::from(
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
    description = "Enables an installed plugin (permission `system.manage`): its entities, fields, hooks, events, routes and web bundle come back. Publishes `plugin.enabled`.",
    params(("key" = String, Path, description = "Plugin key"), ToggleParams),
    responses(
        (status = 200, description = "Plugin enabled", body = Data<PluginResponse>),
        (status = 409, description = "The plugin overlaps with another enabled plugin and the overlap was not acknowledged (`plugin_conflicts_unacknowledged`)", body = ErrorEnvelope),
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
        toggle(
            &state,
            &tenant,
            client,
            &key,
            true,
            params.acknowledge_conflicts,
        )
        .await?,
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
        toggle(&state, &tenant, client, &key, false, false).await?,
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
        if setting.kind == SETTING_SECRET {
            continue;
        }
        let valid = match setting.kind.as_str() {
            "number" => value.as_i64().is_some_and(|number| {
                setting.min.is_none_or(|min| number >= min)
                    && setting.max.is_none_or(|max| number <= max)
            }),
            "boolean" => value.is_boolean(),
            crate::manifest::SETTING_SELECT => value
                .as_str()
                .is_some_and(|text| setting.options.iter().any(|option| option.value == text)),
            _ => value.as_str().is_some_and(|text| text.len() <= 1000),
        };
        if valid || value.is_null() {
            accepted.insert(key.clone(), value.clone());
        } else {
            violations.push(FieldViolation::new(
                key,
                format!("must be a valid {}", setting.kind),
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
    let mut tx = aphel_kernel::database::transaction::MySqlTx::from(
        state.db()?.begin().await.map_err(AppError::from)?,
    );
    store::set_settings(&mut tx, &key, &settings).await?;
    for (name, secret) in &secrets {
        let secret_key = secret_key(&key, name);
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
    entry: String,
    style: Option<String>,
}

#[utoipa::path(
    get,
    path = "/public/wasm-runtime",
    tag = TAG,
    operation_id = "plugins_web",
    description = "Lists the web bundles of the enabled plugins, loaded by the Aphel web app before it builds its routes. Public: plugin bundles are code, not data.",
    responses(
        (status = 200, description = "Web bundles to load", body = Data<Vec<WebPluginResponse>>),
    )
)]
async fn web_plugins(
    State(state): State<AppState>,
) -> Result<Json<Data<Vec<WebPluginResponse>>>, ApiError> {
    let plugins = store::list(&state.db()?).await?;
    let asset = |plugin: &InstalledPlugin, path: &str| {
        format!(
            "/api/v1/public/wasm-runtime/{}/{path}?v={}",
            plugin.key,
            &plugin.checksum[..12]
        )
    };
    Ok(Json(Data::new(
        plugins
            .iter()
            .filter(|plugin| plugin.enabled)
            .filter_map(|plugin| {
                let web = plugin.manifest.web.as_ref()?;
                Some(WebPluginResponse {
                    key: plugin.key.clone(),
                    name: plugin.manifest.name.clone(),
                    version: plugin.version.clone(),
                    menu: plugin.manifest.menu.clone(),
                    role_homes: plugin
                        .manifest
                        .role_presets
                        .iter()
                        .filter_map(|preset| Some((preset.key.clone(), preset.home.clone()?)))
                        .collect(),
                    entry: asset(plugin, &web.entry),
                    style: web.style.as_deref().map(|style| asset(plugin, style)),
                })
            })
            .collect(),
    )))
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
    Ok(axum::response::Redirect::permanent(&format!("/p/{key}/")).into_response())
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

struct RouteCall {
    method: Method,
    key: String,
    path: String,
    query: Option<String>,
    headers: HeaderMap,
    body: Bytes,
}

async fn serve_route(
    state: &AppState,
    call: RouteCall,
    caller: Option<&AccessScope>,
) -> Result<Response, AppError> {
    let RouteCall {
        method,
        key,
        path,
        query,
        headers,
        body,
    } = call;
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
    let (route, params) = match_route(
        &plugin.manifest.routes,
        caller.is_none(),
        method.as_str(),
        &path,
    )
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
        headers: forwarded_headers(&headers),
        body_json,
        actor,
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
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        );
        if let Some(filename) = raw.filename
            && let Ok(value) =
                HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }
        return Ok(response);
    }
    let body: Value = serde_json::from_str(&result.response.body_json).unwrap_or(Value::Null);
    Ok((status, Json(body)).into_response())
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
    };
    Ok(serve_route(&state, call, Some(&tenant.scope)).await?)
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

async fn public_dispatch(
    State(state): State<AppState>,
    client: ClientInfo,
    method: Method,
    Path((key, path)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let (per_address, per_plugin) = &*PUBLIC_LIMITS;
    let message = "muitas requisições em sequência; aguarde um pouco e tente novamente";
    per_address
        .check(&format!("{key}:{}", client.ip_or_unknown()), message)
        .await?;
    per_plugin.check(&key, message).await?;
    let call = RouteCall {
        method,
        key,
        path,
        query,
        headers,
        body,
    };
    Ok(serve_route(&state, call, None).await?)
}

fn forwarded_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| !WITHHELD_HEADERS.contains(name))
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
