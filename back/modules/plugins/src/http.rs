use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::multipart::MultipartRejection;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, RawQuery, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::manifest::{PluginManifest, match_route};
use crate::package::{MAX_PACKAGE_BYTES, read_package};
use crate::registry;
use crate::runtime::{self, wit};
use crate::schema;
use crate::store::{self, InstalledPlugin};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange};
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::builder::APHEL_VERSION;

pub const TAG: &str = "plugins";
const PLUGIN_NOT_FOUND: &str = "plugin_not_found";
const PLUGIN_ROUTE_NOT_FOUND: &str = "plugin_route_not_found";
const INVALID_SETTINGS: &str = "plugin_settings_invalid";
const WEB_PREFIX: &str = "web/";

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
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PluginResponse {
    key: String,
    name: String,
    version: String,
    description: Option<String>,
    enabled: bool,
    entities: Vec<String>,
    extended_types: Vec<String>,
    hooks: Vec<String>,
    events: Vec<String>,
    routes: Vec<String>,
    has_web: bool,
    settings: Vec<PluginSettingResponse>,
    installed_at: String,
    updated_at: String,
}

impl From<InstalledPlugin> for PluginResponse {
    fn from(plugin: InstalledPlugin) -> Self {
        let values = plugin.effective_settings();
        let manifest = &plugin.manifest;
        let mut extended: Vec<String> = manifest
            .fields
            .iter()
            .map(|field| field.record_type.clone())
            .collect();
        extended.dedup();
        Self {
            entities: manifest.entity_record_types(),
            extended_types: extended,
            hooks: manifest
                .hooks
                .iter()
                .map(|hook| format!("{} ({})", hook.hook, hook.record_types.join(", ")))
                .collect(),
            events: manifest.events.clone(),
            routes: manifest
                .routes
                .iter()
                .map(|route| format!("{} {}", route.method, route.path))
                .collect(),
            has_web: manifest.web.is_some(),
            settings: manifest
                .settings
                .iter()
                .map(|setting| PluginSettingResponse {
                    key: setting.key.clone(),
                    label: setting.label.clone(),
                    kind: setting.kind.clone(),
                    help: setting.help.clone(),
                    min: setting.min,
                    max: setting.max,
                    value: values.get(&setting.key).cloned().unwrap_or(Value::Null),
                })
                .collect(),
            installed_at: plugin.installed_at.to_rfc3339(),
            updated_at: plugin.updated_at.to_rfc3339(),
            key: plugin.key,
            name: plugin.name,
            version: plugin.version,
            description: plugin.description,
            enabled: plugin.enabled,
        }
    }
}

#[utoipa::path(
    get,
    path = "/plugins",
    tag = TAG,
    operation_id = "plugins_list",
    security(("session_cookie" = [])),
    description = "Lists the plugins installed at runtime (permission `system.manage`), with what each one declares: entities, extended record types, hooks, consumed events, routes and settings.",
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
    let plugins = store::list(&state.db()?).await?;
    Ok(Json(Data::new(
        plugins.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/plugins",
    tag = TAG,
    operation_id = "plugins_install",
    security(("session_cookie" = [])),
    description = "Installs or upgrades a plugin from an `.aphelplugin` package sent as the multipart field `package` (permission `system.manage`). The package is a zip with `plugin.toml`, an optional `backend.wasm` built with aphel-plugin-sdk and an optional `web/` bundle. Declared entities become custom objects managed by the plugin, declared fields become managed custom fields, and hooks, events and routes start working without restarting Aphel. Publishes `plugin.installed`.",
    request_body(content_type = "multipart/form-data", description = "Field `package` with the .aphelplugin file"),
    responses(
        (status = 201, description = "Plugin installed or upgraded", body = Data<PluginResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "A custom object or field with the same key already exists (`plugin_object_conflict`, `plugin_field_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid package, manifest or backend (`plugin_package_invalid`, `plugin_manifest_invalid`, `plugin_backend_invalid`)", body = ErrorEnvelope),
    )
)]
async fn install_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<Data<PluginResponse>>), ApiError> {
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
    schema::apply(&state, scope, &package.manifest).await?;
    let mut tx = pool.begin().await.map_err(AppError::from)?;
    store::save(&mut tx, &package, Some(scope.user_id())).await?;
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
    Ok((StatusCode::CREATED, Json(Data::new(plugin.into()))))
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
    Ok(installed(state, key).await?.into())
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
    description = "Replaces the settings of a plugin (permission `system.manage`), validated against the settings it declares. `null` restores the default. Publishes `plugin.settings_updated`.",
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
    let mut tx = state.db()?.begin().await.map_err(AppError::from)?;
    store::set_settings(&mut tx, &key, &settings).await?;
    tx.commit().await.map_err(AppError::from)?;
    record(
        &state,
        &tenant.scope,
        "plugin.settings_updated",
        &key,
        vec![FieldChange::new(
            "settings",
            Value::Object(plugin.settings),
            Value::Object(settings),
        )],
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(installed(&state, &key).await?.into())))
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

async fn dispatch(
    State(state): State<AppState>,
    tenant: AuthContext,
    method: Method,
    Path((key, path)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Result<Response, ApiError> {
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
    let (route, params) =
        match_route(&plugin.manifest.routes, method.as_str(), &path).ok_or_else(route_missing)?;
    if let Some(permission) = &route.permission {
        let permission = Permission::parse(permission.clone()).map_err(|_| {
            AppError::Configuration(format!("the plugin {key} declares an invalid permission"))
        })?;
        tenant.scope.require(&permission)?;
    }
    let body_json = if body.is_empty() {
        Value::Null.to_string()
    } else {
        serde_json::from_slice::<Value>(&body)
            .map_err(|_| AppError::Validation("the body must be JSON".to_owned()))?
            .to_string()
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
        body_json,
        user_id: Some(tenant.scope.user_id().as_str().to_owned()),
    };
    let response = runtime::on_route(&plugin, &tenant.scope, &request).await?;
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK);
    let body: Value = serde_json::from_str(&response.body_json).unwrap_or(Value::Null);
    Ok((status, Json(body)).into_response())
}

fn form_pairs(query: &str) -> Vec<(String, String)> {
    form_urlencoded::parse(query.as_bytes())
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}
