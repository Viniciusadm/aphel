use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::registry::LoadedPlugin;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

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
const TICK: Duration = Duration::from_millis(10);
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const PLUGIN_FAILED: &str = "plugin_failed";
const INVALID_BACKEND: &str = "plugin_backend_invalid";

pub type Backend = PluginPre<CallState>;

pub struct CallState {
    pub plugin: Arc<LoadedPlugin>,
    pub scope: AccessScope,
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

pub async fn compile(bytes: Vec<u8>) -> Result<Backend, AppError> {
    let component = tokio::task::spawn_blocking(move || Component::new(engine(), &bytes))
        .await
        .map_err(|error| AppError::Internal(format!("plugin compilation panicked: {error}")))?
        .map_err(invalid_backend)?;
    let pre = linker()
        .instantiate_pre(&component)
        .map_err(invalid_backend)?;
    PluginPre::new(pre).map_err(invalid_backend)
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

async fn instantiate(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    timeout: Duration,
) -> Result<(Store<CallState>, Plugin), AppError> {
    let Some(backend) = &plugin.backend else {
        return Err(AppError::Configuration(format!(
            "the plugin {} has no backend",
            plugin.key
        )));
    };
    let mut store = Store::new(
        engine(),
        CallState {
            plugin: plugin.clone(),
            scope: scope.clone(),
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

pub async fn on_hook(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    call: &wit::HookCall,
) -> Result<wit::HookOutcome, AppError> {
    let (mut store, bindings) = instantiate(plugin, scope, HOOK_TIMEOUT).await?;
    bindings
        .call_on_hook(&mut store, call)
        .await
        .map_err(|error| failure(plugin, error))?
        .map_err(plugin_error)
}

pub async fn on_event(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    event: &wit::Event,
) -> Result<(), AppError> {
    let (mut store, bindings) = instantiate(plugin, scope, EVENT_TIMEOUT).await?;
    bindings
        .call_on_event(&mut store, event)
        .await
        .map_err(|error| failure(plugin, error))?
        .map_err(plugin_error)
}

pub async fn on_route(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    request: &wit::Request,
) -> Result<wit::Response, AppError> {
    let (mut store, bindings) = instantiate(plugin, scope, ROUTE_TIMEOUT).await?;
    bindings
        .call_on_route(&mut store, request)
        .await
        .map_err(|error| failure(plugin, error))?
        .map_err(plugin_error)
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
        Some(404) => AppError::not_found_code(code, error.message),
        Some(403) => AppError::forbidden_code(code, error.message),
        Some(409) => AppError::conflict_code(code, error.message),
        _ => AppError::validation_code(code, error.message),
    }
}

pub fn to_plugin_error(error: &AppError) -> wit::PluginError {
    let (code, status) = match error {
        AppError::CodedFields { code, .. } => (*code, 422),
        AppError::Coded { code, .. } | AppError::CodedDetails { code, .. } => (*code, 400),
        AppError::NotFound(_) => ("not_found", 404),
        AppError::Forbidden(_) => ("forbidden", 403),
        AppError::Conflict(_) => ("conflict", 409),
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
    wit::PluginError {
        code: code.to_owned(),
        message: error.client_message().to_owned(),
        status: Some(status),
        fields,
    }
}
