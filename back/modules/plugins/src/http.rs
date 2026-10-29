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

use crate::host::secret_key;
use crate::manifest::{PluginManifest, SETTING_SECRET, match_route};
use crate::package::{MAX_PACKAGE_BYTES, read_package};
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
        .routes(routes!(update_plugin_settings))
        .route("/p/{key}/{*path}", any(dispatch))
}

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(web_plugins))
        .route("/public/plugins/{key}/{*path}", get(web_asset))
        .route("/public/p/{key}/{*path}", any(public_dispatch))
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
    let now = chrono::Utc::now();
    let actor = Actor::from_scope(scope);
    let mut tx = pool.begin().await.map_err(AppError::from)?;
    store::bump(&mut tx).await?;
    state
        .events
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), action, now).with_payload(json!({ "plugin": plugin })),
        )
        .await?;
    state
        .audit
        .record(
            &mut tx,
            AuditEntry::new(actor, action, now)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    registry::invalidate().await;
    Ok(())
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
pub(crate) struct PluginDeclarations {
    entities: Vec<String>,
    extended_types: Vec<String>,
    hooks: Vec<String>,
    events: Vec<String>,
    routes: Vec<String>,
    public_routes: Vec<String>,
    requires: Vec<String>,
    permissions: Vec<PluginPermissionResponse>,
    allowed_hosts: Vec<String>,
    schedules: Vec<String>,
    secrets: Vec<String>,
    actions: Vec<String>,
    emits: Vec<String>,
    notifications: Vec<String>,
    widgets: Vec<String>,
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
            public_routes: routes(true),
            requires: manifest.requires.clone(),
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
}

async fn respond(state: &AppState, plugin: InstalledPlugin) -> Result<PluginResponse, AppError> {
    let snapshot = registry::current(&state.db()?).await?;
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
            value: values.get(&setting.key).cloned().unwrap_or(Value::Null),
            configured,
        });
    }
    Ok(PluginResponse {
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
        .map(|(permission, roles)| RuntimeGrant { permission, roles })
        .collect();
    replace_runtime_permissions(tx, &permissions_owner(&manifest.key), &grants, active).await
}

#[utoipa::path(
    get,
    path = "/plugins",
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
    post,
    path = "/plugins",
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
        (status = 409, description = "A custom object, field or permission with the same key already exists (`plugin_object_conflict`, `plugin_field_conflict`, `permission_already_registered`)", body = ErrorEnvelope),
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
    let bytes = package_bytes(multipart).await?;
    let package = read_package(&bytes)?;
    let compiled: Vec<String> = state
        .app
        .modules()
        .iter()
        .map(|module| module.manifest.key.to_owned())
        .collect();
    package.manifest.validate(&compiled, APHEL_VERSION)?;
    if let Some(backend) = package.backend() {
        runtime::validate(backend.content.clone()).await?;
    }
    let pool = state.db()?;
    let previous = store::find(&pool, &package.manifest.key).await?;
    if params.dry_run {
        let manifest = &package.manifest;
        let consent = PluginConsentResponse {
            key: manifest.key.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            installed_version: previous.map(|plugin| plugin.version),
            declares: PluginDeclarations::from(manifest),
        };
        return Ok(Json(Data::new(consent)).into_response());
    }
    schema::apply(&state, scope, &package.manifest).await?;
    let enabled = previous.as_ref().is_none_or(|plugin| plugin.enabled);
    let mut tx = pool.begin().await.map_err(AppError::from)?;
    store::save(&mut tx, &package, Some(scope.user_id())).await?;
    sync_permissions(&mut tx, &package.manifest, enabled).await?;
    tx.commit().await.map_err(AppError::from)?;
    record(
        &state,
        scope,
        "plugin.installed",
        &package.manifest.key,
        vec![FieldChange::new(
            "version",
            json!(previous.map(|plugin| plugin.version)),
            json!(package.manifest.version),
        )],
        client.0.ip,
    )
    .await?;
    let plugin = installed(&state, &package.manifest.key).await?;
    if !plugin.enabled {
        schema::set_active(&state, &plugin.manifest, false).await?;
    }
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
}

async fn package_bytes(
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Vec<u8>, AppError> {
    let mut multipart = multipart
        .map_err(|_| AppError::Validation("send the package as multipart/form-data".to_owned()))?;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("the form could not be read".to_owned()))?
    {
        if field.name() == Some("package") {
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::Validation("the package could not be read".to_owned()))?;
            return Ok(bytes.to_vec());
        }
    }
    Err(AppError::invalid_fields(
        "invalid form",
        vec![FieldViolation::new("package", "is required")],
    ))
}

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct UninstallParams {
    #[serde(default)]
    purge: bool,
}

#[utoipa::path(
    delete,
    path = "/plugins/{key}",
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
    if params.purge {
        schema::purge(&state, &plugin.manifest).await?;
    } else {
        schema::set_active(&state, &plugin.manifest, false).await?;
    }
    let mut tx = state.db()?.begin().await.map_err(AppError::from)?;
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
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle(
    state: &AppState,
    tenant: &AuthContext,
    client: ClientInfo,
    key: &str,
    enabled: bool,
) -> Result<PluginResponse, AppError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let plugin = installed(state, key).await?;
    if plugin.enabled != enabled {
        let mut tx = state.db()?.begin().await.map_err(AppError::from)?;
        store::set_enabled(&mut tx, key, enabled).await?;
        set_runtime_permissions_active(&mut tx, &permissions_owner(key), enabled).await?;
        tx.commit().await?;
        schema::set_active(state, &plugin.manifest, enabled).await?;
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
    path = "/plugins/{key}/enable",
    tag = TAG,
    operation_id = "plugins_enable",
    security(("session_cookie" = [])),
    description = "Enables an installed plugin (permission `system.manage`): its entities, fields, hooks, events, routes and web bundle come back. Publishes `plugin.enabled`.",
    params(("key" = String, Path, description = "Plugin key")),
    responses(
        (status = 200, description = "Plugin enabled", body = Data<PluginResponse>),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Plugin not installed (`plugin_not_found`)", body = ErrorEnvelope),
    )
)]
async fn enable_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<Json<Data<PluginResponse>>, ApiError> {
    Ok(Json(Data::new(
        toggle(&state, &tenant, client, &key, true).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/plugins/{key}/disable",
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
        toggle(&state, &tenant, client, &key, false).await?,
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

#[utoipa::path(
    put,
    path = "/plugins/{key}/settings",
    tag = TAG,
    operation_id = "plugins_update_settings",
    security(("session_cookie" = [])),
    description = "Replaces the settings of a plugin (permission `system.manage`), validated against the settings it declares. `null` restores the default. Secret settings are stored encrypted and never returned (only `configured`): a string sets the secret, `null` removes it and leaving the key out keeps it. Publishes `plugin.settings_updated`.",
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
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    let plugin = installed(&state, &key).await?;
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
    let mut tx = state.db()?.begin().await.map_err(AppError::from)?;
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
    version: String,
    entry: String,
    style: Option<String>,
}

#[utoipa::path(
    get,
    path = "/public/plugins",
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
            "/api/v1/public/plugins/{}/{path}?v={}",
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
                    version: plugin.version.clone(),
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
    let response = runtime::on_route(&plugin, scope, &request).await?;
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK);
    let body: Value = serde_json::from_str(&response.body_json).unwrap_or(Value::Null);
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
            "plugins.public_per_address",
            PUBLIC_PER_ADDRESS_LIMIT,
            MINUTE,
        ),
        SharedRateLimiter::new("plugins.public_per_plugin", PUBLIC_PER_PLUGIN_LIMIT, MINUTE),
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
