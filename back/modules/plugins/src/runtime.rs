use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use sqlx::MySqlPool;

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::compiled;
use crate::registry::LoadedPlugin;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::shared::{Actor, ExecutionContext};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::state::{runtime_pool, runtime_state};
use uuid::Uuid;

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    imports: { default: async },
    exports: { default: async },
});

pub use aphel::plugin::types as wit;

pub const HOOK_TIMEOUT: Duration = Duration::from_secs(2);
pub const ROUTE_TIMEOUT: Duration = Duration::from_secs(10);
pub const EVENT_TIMEOUT: Duration = Duration::from_secs(30);
pub const SCHEDULE_TIMEOUT: Duration = Duration::from_secs(60);
pub const ACTION_TIMEOUT: Duration = Duration::from_secs(30);
pub const WIDGET_TIMEOUT: Duration = Duration::from_secs(5);
const TICK: Duration = Duration::from_millis(10);
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const PLUGIN_FAILED: &str = "plugin_failed";
const INVALID_BACKEND: &str = "plugin_backend_invalid";
const PLUGIN_OWNER_INACTIVE: &str = "plugin_owner_inactive";
const COMPILED_CACHE_SIZE: usize = 16;
const MAX_STORED_ARTIFACT_BYTES: usize = 48 * 1024 * 1024;

pub type Backend = Arc<PluginPre<CallState>>;

pub struct CallState {
    pub plugin: Arc<LoadedPlugin>,
    pub scope: AccessScope,
    pub in_transaction: bool,
    pub read_only: bool,
    pub retryable: bool,
    pub hook_state: Option<serde_json::Value>,
    pub deadline: Instant,
    pub effects: Vec<crate::effects::PendingEffect>,
    pub audit_reason: Option<String>,
    pub route_call: bool,
    pub raw_body: Option<RawBody>,
    limits: StoreLimits,
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for CallState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn sandbox() -> WasiCtx {
    WasiCtxBuilder::new()
        .allow_tcp(false)
        .allow_udp(false)
        .allow_ip_name_lookup(false)
        .build()
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).expect("the plugin engine configuration is valid");
        let ticker = engine.weak();
        std::thread::Builder::new()
            .name("aphel-plugins-epoch".to_owned())
            .spawn(move || {
                while let Some(engine) = ticker.upgrade() {
                    engine.increment_epoch();
                    drop(engine);
                    std::thread::sleep(TICK);
                }
            })
            .expect("the plugin epoch thread starts");
        engine
    })
}

fn linker() -> &'static Linker<CallState> {
    static LINKER: OnceLock<Linker<CallState>> = OnceLock::new();
    LINKER.get_or_init(|| {
        let mut linker = Linker::new(engine());
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .expect("the WASI interfaces are defined once");
        Plugin::add_to_linker::<CallState, HasSelf<CallState>>(&mut linker, |state| state)
            .expect("the plugin host interface is defined once");
        linker
    })
}

fn invalid_backend(error: impl std::fmt::Display) -> AppError {
    AppError::validation_code(
        INVALID_BACKEND,
        format!(
            "backend.wasm is not a component of the aphel:plugin world; build it with aphel-plugin-sdk ({error})"
        ),
    )
}

pub fn backend_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn engine_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| {
        let mut hasher = DefaultHasher::new();
        engine().precompile_compatibility_hash().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    })
}

type CompiledCache = Mutex<Vec<(String, Weak<PluginPre<CallState>>)>>;

fn compiled_cache() -> &'static CompiledCache {
    static CACHE: OnceLock<CompiledCache> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn cached_backend(hash: &str) -> Option<Backend> {
    compiled_cache()
        .lock()
        .ok()?
        .iter()
        .find(|(cached, _)| cached == hash)
        .and_then(|(_, backend)| backend.upgrade())
}

fn remember_backend(hash: String, backend: Backend) {
    if let Ok(mut cache) = compiled_cache().lock() {
        cache.retain(|(cached, backend)| *cached != hash && backend.strong_count() > 0);
        cache.push((hash, Arc::downgrade(&backend)));
        if cache.len() > COMPILED_CACHE_SIZE {
            cache.remove(0);
        }
    }
}

fn release_compile_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        libc::malloc_trim(0);
    }
}

async fn stored_component(pool: &MySqlPool, hash: &str) -> Option<Component> {
    let artifact = match compiled::find(pool, hash, engine_key()).await {
        Ok(artifact) => artifact?,
        Err(error) => {
            tracing::warn!(error = %error, "could not read a precompiled plugin backend");
            return None;
        }
    };
    tokio::task::spawn_blocking(move || unsafe { Component::deserialize(engine(), &artifact) }.ok())
        .await
        .ok()
        .flatten()
}

async fn compile_component(
    pool: Option<&MySqlPool>,
    hash: &str,
    bytes: Vec<u8>,
) -> Result<Component, AppError> {
    let component = tokio::task::spawn_blocking(move || Component::new(engine(), &bytes))
        .await
        .map_err(|error| AppError::Internal(format!("plugin compilation panicked: {error}")))?
        .map_err(invalid_backend)?;
    if let Some(pool) = pool {
        match component.serialize() {
            Ok(artifact) if artifact.len() <= MAX_STORED_ARTIFACT_BYTES => {
                if let Err(error) = compiled::save(pool, hash, engine_key(), &artifact).await {
                    tracing::warn!(error = %error, "could not store a precompiled plugin backend");
                }
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(error = %error, "could not serialize a compiled plugin backend")
            }
        }
    }
    Ok(component)
}

pub async fn compile(bytes: Vec<u8>) -> Result<Backend, AppError> {
    let hash = backend_hash(&bytes);
    if let Some(backend) = cached_backend(&hash) {
        return Ok(backend);
    }
    static COMPILATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _compilation = COMPILATION.lock().await;
    if let Some(backend) = cached_backend(&hash) {
        return Ok(backend);
    }
    let pool = runtime_state().ok().and_then(|state| state.db().ok());
    let stored = match &pool {
        Some(pool) => stored_component(pool, &hash).await,
        None => None,
    };
    let component = match stored {
        Some(component) => component,
        None => compile_component(pool.as_ref(), &hash, bytes).await?,
    };
    let backend = linker()
        .instantiate_pre(&component)
        .map_err(invalid_backend)
        .and_then(|pre| PluginPre::new(pre).map_err(invalid_backend));
    drop(component);
    release_compile_memory();
    let backend = Arc::new(backend?);
    remember_backend(hash, backend.clone());
    Ok(backend)
}

pub async fn validate(bytes: Vec<u8>) -> Result<(), AppError> {
    compile(bytes).await.map(|_| ())
}

fn failure(plugin: &LoadedPlugin, error: wasmtime::Error) -> AppError {
    tracing::warn!(plugin = %plugin.key, error = %error, "plugin call failed");
    AppError::unavailable_code(
        PLUGIN_FAILED,
        format!("the plugin {} failed to run", plugin.key),
    )
}

pub fn capability_scope(plugin: &LoadedPlugin, scope: &AccessScope) -> AccessScope {
    scope
        .clone()
        .restricted(|permission| plugin.manifest.allows(permission))
}

pub async fn plugin_scope(plugin: &LoadedPlugin) -> Result<AccessScope, AppError> {
    let inactive = || {
        AppError::unavailable_code(
            PLUGIN_OWNER_INACTIVE,
            format!(
                "the plugin {} acts with the permissions of whoever installed it, who is no longer an active member; an administrator must reinstall it",
                plugin.key
            ),
        )
    };
    let installer = UserId::new(plugin.installed_by.clone().ok_or_else(inactive)?)?;
    let access = MySqlAccessRepository::new(runtime_pool()?)
        .load_access(&installer)
        .await?
        .ok_or_else(inactive)?;
    Ok(
        capability_scope(plugin, &build_access_scope(installer, access))
            .through_plugin(&plugin.key),
    )
}

async fn instantiate(
    plugin: &Arc<LoadedPlugin>,
    scope: AccessScope,
    timeout: Duration,
    in_transaction: bool,
) -> Result<(Store<CallState>, Plugin), AppError> {
    let Some(backend) = &plugin.backend else {
        if plugin.backend_error.is_some() {
            return Err(AppError::unavailable_code(
                PLUGIN_FAILED,
                format!(
                    "the plugin {} could not be loaded; an administrator must reinstall it",
                    plugin.key
                ),
            ));
        }
        return Err(AppError::Configuration(format!(
            "the plugin {} has no backend",
            plugin.key
        )));
    };
    let backend = backend.get().await?;
    let mut store = Store::new(
        engine(),
        CallState {
            plugin: plugin.clone(),
            scope,
            in_transaction,
            read_only: false,
            retryable: false,
            hook_state: None,
            deadline: Instant::now() + timeout,
            effects: Vec::new(),
            audit_reason: None,
            route_call: false,
            raw_body: None,
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_LIMIT)
                .instances(16)
                .build(),
            wasi: sandbox(),
            table: ResourceTable::new(),
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_epoch_deadline((timeout.as_millis() / TICK.as_millis()).max(1) as u64);
    let bindings = backend
        .instantiate_async(&mut store)
        .await
        .map_err(|error| failure(plugin, error))?;
    Ok((store, bindings))
}

fn execution_context(
    plugin: &LoadedPlugin,
    trigger: &str,
    scope: Option<&AccessScope>,
    parent: Option<&ExecutionContext>,
    event_id: Option<Uuid>,
) -> Result<ExecutionContext, AppError> {
    if trigger == "hook" && parent.is_some_and(|parent| parent.executor == plugin.key) {
        return Ok(parent.unwrap().clone());
    }
    let actor = scope
        .map(Actor::from_scope)
        .filter(|actor| !matches!(actor, Actor::Plugin(_) | Actor::System));
    ExecutionContext::plugin(
        &plugin.key,
        trigger,
        actor.as_ref(),
        parent,
        event_id,
        Uuid::now_v7(),
    )
}

pub async fn on_hook(
    plugin: &Arc<LoadedPlugin>,
    ctx: &mut aphel_kernel::modules::HookCtx<'_>,
    call: &wit::HookCall,
) -> Result<wit::HookOutcome, AppError> {
    let execution = execution_context(
        plugin,
        "hook",
        Some(ctx.scope),
        ExecutionContext::current().as_ref(),
        None,
    )?;
    let reentry =
        ExecutionContext::current().is_some_and(|parent| parent.chain.contains(&plugin.key));
    let scope = if ctx.scope.is_system() {
        plugin_scope(plugin).await?
    } else {
        capability_scope(plugin, ctx.scope)
    };
    let (mut store, bindings) = instantiate(plugin, scope, HOOK_TIMEOUT, true).await?;
    store.data_mut().read_only = reentry && call.hook != "record.committed";
    let record_id = Uuid::parse_str(&call.record_id).ok();
    let mut state = record_id.and_then(|id| ctx.tx.record_states.get(&(call.record_type.clone(), id)).cloned())
        .unwrap_or_else(|| serde_json::json!({"operation_id": ctx.tx.operation_id, "record_type": call.record_type,
            "record_id": call.record_id, "before": null, "before_available": false, "proposed": null, "persisted": null}));
    state["phase"] = serde_json::json!(call.hook);
    state["effects_allowed"] = serde_json::json!(!store.data().read_only);
    if call.hook == "record.saving" {
        let values = serde_json::from_str(&call.values_json).unwrap_or_default();
        let custom = serde_json::from_str(&call.custom_fields_json).unwrap_or_default();
        state["proposed"] = aphel_kernel::records::snapshot::proposed(
            state.get("before").filter(|value| !value.is_null()),
            &values,
            &custom,
        );
    }
    let runtime = runtime_state()?;
    let record_type = aphel_kernel::application::shared::RecordType::parse(&call.record_type)?;
    let redaction = aphel_kernel::api::routes::v1::field_permissions::field_redaction(
        &runtime,
        &store.data().scope,
        &record_type,
    )
    .await?;
    for key in ["before", "proposed", "persisted"] {
        if let Some(snapshot) = state.get_mut(key).filter(|value| value.is_object()) {
            let mut flat = snapshot
                .get("values")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            flat["custom_fields"] = snapshot
                .get("custom_fields")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            redaction.redact_value(&mut flat);
            snapshot["custom_fields"] = flat
                .as_object_mut()
                .and_then(|flat| flat.remove("custom_fields"))
                .unwrap_or_else(|| serde_json::json!({}));
            snapshot["values"] = flat;
        }
    }
    let mut call = call.clone();
    let mut values: serde_json::Value =
        serde_json::from_str(&call.values_json).unwrap_or_else(|_| serde_json::json!({}));
    values["custom_fields"] =
        serde_json::from_str(&call.custom_fields_json).unwrap_or_else(|_| serde_json::json!({}));
    redaction.redact_value(&mut values);
    call.custom_fields_json = values
        .as_object_mut()
        .and_then(|values| values.remove("custom_fields"))
        .unwrap_or_else(|| serde_json::json!({}))
        .to_string();
    call.values_json = values.to_string();
    call.changes
        .retain(|change| !redaction.hides_path(&change.field));
    store.data_mut().hook_state = Some(state);
    let outcome = execution
        .scope(async {
            bindings
                .call_on_hook(&mut store, &call)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await?;
    if call.hook == "record.saving" {
        let registry = runtime.registry().await?;
        let fields = aphel_kernel::application::fields::FieldSet::for_scope(
            &aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository::new(
                runtime.db()?,
            ),
            registry.require(&call.record_type)?,
            &store.data().scope,
        )
        .await?;
        for patch in [&outcome.values_json, &outcome.custom_fields_json] {
            let patch: serde_json::Value = serde_json::from_str(patch).map_err(|_| {
                AppError::validation_code(
                    "plugin_hook_patch_invalid",
                    "hook patches must be JSON objects",
                )
            })?;
            let object = patch.as_object().ok_or_else(|| {
                AppError::validation_code(
                    "plugin_hook_patch_invalid",
                    "hook patches must be JSON objects",
                )
            })?;
            for key in object.keys() {
                if !fields
                    .find(key)
                    .is_some_and(|field| field.definition.is_active && field.is_editable())
                {
                    return Err(AppError::forbidden_code(
                        "plugin_field_not_writable",
                        format!("the hook cannot change field {key}"),
                    ));
                }
            }
        }
    }
    let pending = std::mem::take(&mut store.data_mut().effects);
    crate::effects::persist(ctx.tx, ctx.pool, pending).await?;
    Ok(outcome)
}

pub async fn on_event(
    plugin: &Arc<LoadedPlugin>,
    event: &wit::Event,
    execution: ExecutionContext,
) -> Result<(), AppError> {
    let scope = plugin_scope(plugin).await?;
    let (mut store, bindings) = instantiate(plugin, scope, EVENT_TIMEOUT, false).await?;
    store.data_mut().retryable = true;
    execution
        .scope(async {
            bindings
                .call_on_event(&mut store, event)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await
}

pub async fn on_schedule(
    plugin: &Arc<LoadedPlugin>,
    name: &str,
    slot: &str,
    execution: ExecutionContext,
) -> Result<(), AppError> {
    let scope = plugin_scope(plugin).await?;
    let (mut store, bindings) = instantiate(plugin, scope, SCHEDULE_TIMEOUT, false).await?;
    store.data_mut().retryable = true;
    execution
        .scope(async {
            bindings
                .call_on_schedule(&mut store, name, slot)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await
}

pub async fn on_action(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    call: &wit::ActionCall,
    step_id: &str,
) -> Result<String, AppError> {
    let mut execution = execution_context(
        plugin,
        "automation",
        Some(scope),
        ExecutionContext::current().as_ref(),
        None,
    )?;
    execution.automation = Some(aphel_kernel::application::shared::AutomationExecution {
        automation_id: call.automation_id.clone(),
        run_id: call.run_id.clone(),
        step_id: step_id.to_owned(),
    });
    let pool = runtime_pool()?;
    let source = format!(
        "automation:{}",
        backend_hash(format!("{}:{}:{step_id}", call.automation_id, call.run_id).as_bytes())
    );
    sqlx::query("INSERT IGNORE INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, JSON_OBJECT(), ?)")
        .bind(execution.id.to_string()).bind(&plugin.key).bind(&source).bind(sqlx::types::Json(&execution)).execute(&pool).await?;
    let sqlx::types::Json(execution): sqlx::types::Json<ExecutionContext> = sqlx::query_scalar(
        "SELECT execution FROM plugins_executions WHERE plugin_key = ? AND source_key = ?",
    )
    .bind(&plugin.key)
    .bind(&source)
    .fetch_one(&pool)
    .await?;
    let scope = capability_scope(plugin, scope);
    let (mut store, bindings) = instantiate(plugin, scope, ACTION_TIMEOUT, false).await?;
    store.data_mut().retryable = true;
    execution
        .scope(async {
            bindings
                .call_on_action(&mut store, call)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await
}

pub async fn on_data_subject(
    plugin: &Arc<LoadedPlugin>,
    request: &aphel_kernel::application::data_subject::DataSubjectRequest<'_>,
    input: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let scope = plugin_scope(plugin).await?;
    let execution = execution_context(plugin, "privacy", None, None, None)?;
    let call = wit::ActionCall {
        name: format!("privacy.{}", request.action.as_str()),
        input_json: input.to_string(),
        record_type: crate::data_subject::PRIVACY_REQUEST_TYPE.to_owned(),
        record_id: request.id.to_string(),
        record_json: "{}".to_owned(),
        automation_id: String::new(),
        run_id: request.id.to_string(),
        actor: scope_actor(&scope),
    };
    let (mut store, bindings) = instantiate(plugin, scope, SCHEDULE_TIMEOUT, false).await?;
    store.data_mut().retryable = true;
    let output = execution
        .scope(async {
            bindings
                .call_on_action(&mut store, &call)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await?;
    serde_json::from_str(&output).map_err(|_| {
        AppError::Internal(format!(
            "the plugin {} answered a data subject call with invalid JSON",
            plugin.key
        ))
    })
}

pub async fn on_widget(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    name: &str,
    request: &wit::WidgetRequest,
) -> Result<String, AppError> {
    let execution = execution_context(plugin, "widget", Some(scope), None, None)?;
    let scope = capability_scope(plugin, scope);
    let (mut store, bindings) = instantiate(plugin, scope, WIDGET_TIMEOUT, false).await?;
    store.data_mut().read_only = true;
    execution
        .scope(async {
            bindings
                .call_on_widget(&mut store, name, request)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await
}

#[derive(Debug, Clone)]
pub struct RawBody {
    pub content_type: String,
    pub filename: Option<String>,
    pub bytes: Vec<u8>,
}

pub struct RouteResult {
    pub response: wit::Response,
    pub raw_body: Option<RawBody>,
}

pub async fn on_route(
    plugin: &Arc<LoadedPlugin>,
    scope: AccessScope,
    request: &wit::Request,
) -> Result<RouteResult, AppError> {
    let execution = execution_context(plugin, "route", Some(&scope), None, None)?;
    let (mut store, bindings) = instantiate(plugin, scope, ROUTE_TIMEOUT, false).await?;
    store.data_mut().route_call = true;
    let response = execution
        .scope(async {
            bindings
                .call_on_route(&mut store, request)
                .await
                .map_err(|error| failure(plugin, error))?
                .map_err(plugin_error)
        })
        .await?;
    Ok(RouteResult {
        response,
        raw_body: store.data_mut().raw_body.take(),
    })
}

fn actor_parts(actor: &Actor) -> (wit::ActorKind, Option<String>) {
    match actor {
        Actor::User(user_id) => (wit::ActorKind::User, Some(user_id.as_str().to_owned())),
        Actor::ApiToken(token_id) => (wit::ActorKind::ApiToken, Some(token_id.clone())),
        Actor::Automation(automation_id) => {
            (wit::ActorKind::Automation, Some(automation_id.clone()))
        }
        Actor::Plugin(plugin_key) => (wit::ActorKind::Plugin, Some(plugin_key.clone())),
        Actor::System => (wit::ActorKind::System, None),
    }
}

pub fn anonymous_actor() -> wit::Actor {
    wit::Actor {
        kind: wit::ActorKind::Anonymous,
        id: None,
        user_id: None,
    }
}

pub fn scope_actor(scope: &AccessScope) -> wit::Actor {
    let (kind, id) = actor_parts(&Actor::from_scope(scope));
    let user_id = (!scope.is_system() && scope.plugin_key().is_none())
        .then(|| scope.user_id().as_str().to_owned());
    wit::Actor { kind, id, user_id }
}

pub fn event_actor(actor: &Actor) -> wit::Actor {
    let (kind, id) = actor_parts(actor);
    let user_id = match actor {
        Actor::User(user_id) => Some(user_id.as_str().to_owned()),
        _ => None,
    };
    wit::Actor { kind, id, user_id }
}

fn intern(code: &str) -> &'static str {
    static CODES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let valid = !code.is_empty()
        && code.len() <= 64
        && code.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
    if !valid {
        return "plugin_error";
    }
    let Ok(mut codes) = CODES.get_or_init(Default::default).lock() else {
        return "plugin_error";
    };
    if let Some(existing) = codes.get(code) {
        return existing;
    }
    let leaked: &'static str = Box::leak(code.to_owned().into_boxed_str());
    codes.insert(leaked);
    leaked
}

pub fn plugin_error(error: wit::PluginError) -> AppError {
    let code = intern(&error.code);
    let fields: Vec<FieldViolation> = error
        .fields
        .into_iter()
        .map(|field| FieldViolation::new(field.field, field.message))
        .collect();
    if !fields.is_empty() {
        return AppError::invalid_fields_code(code, error.message, fields);
    }
    match error.status {
        Some(401) => AppError::unauthorized_code(code, error.message),
        Some(403) => AppError::forbidden_code(code, error.message),
        Some(404) => AppError::not_found_code(code, error.message),
        Some(409) => AppError::conflict_code(code, error.message),
        Some(413) => AppError::payload_too_large_code(code, error.message),
        Some(429) => AppError::too_many_requests_code(code, error.message),
        Some(503) => AppError::unavailable_code(code, error.message),
        _ => AppError::validation_code(code, error.message),
    }
}

fn coded_status(kind: CodedKind) -> u16 {
    match kind {
        CodedKind::Conflict => 409,
        CodedKind::Forbidden => 403,
        CodedKind::NotFound => 404,
        CodedKind::Unauthorized => 401,
        CodedKind::Validation => 422,
        CodedKind::TooManyRequests => 429,
        CodedKind::PayloadTooLarge => 413,
        CodedKind::Unavailable => 503,
    }
}

const CONCURRENT_WRITE: &str =
    "a concurrent write got in the way; reload and apply the change again";
const INTERNAL_MESSAGE: &str = "internal error";

fn is_lock_conflict(error: &AppError) -> bool {
    matches!(error, AppError::Persistence(message)
        if message.contains("Deadlock found") || message.contains("Lock wait timeout"))
}

pub fn to_plugin_error(error: &AppError) -> wit::PluginError {
    if is_lock_conflict(error) {
        return wit::PluginError {
            code: "version_conflict".to_owned(),
            message: CONCURRENT_WRITE.to_owned(),
            status: Some(409),
            fields: Vec::new(),
        };
    }
    let (code, status) = match error {
        AppError::CodedFields { code, .. } => (*code, 422),
        AppError::Coded { code, kind, .. } | AppError::CodedDetails { code, kind, .. } => {
            (*code, coded_status(*kind))
        }
        AppError::NotFound(_) => ("not_found", 404),
        AppError::Forbidden(_) => ("forbidden", 403),
        AppError::Unauthorized(_) => ("unauthorized", 401),
        AppError::Conflict(_) => ("conflict", 409),
        AppError::RateLimited(_) => ("rate_limited", 429),
        AppError::Validation(_) | AppError::InvalidFields { .. } => ("validation_error", 422),
        _ => ("internal_error", 500),
    };
    let fields = match error {
        AppError::CodedFields { fields, .. } | AppError::InvalidFields { fields, .. } => fields
            .iter()
            .map(|field| wit::FieldError {
                field: field.field.clone(),
                message: field.message.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let message = if status == 500 {
        INTERNAL_MESSAGE
    } else {
        error.client_message()
    };
    wit::PluginError {
        code: code.to_owned(),
        message: message.to_owned(),
        status: Some(status),
        fields,
    }
}
