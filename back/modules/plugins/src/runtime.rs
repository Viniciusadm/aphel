use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Value, json};
use wasmtime::{
    Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

use crate::host;
use crate::registry::LoadedPlugin;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub const HOOK_TIMEOUT: Duration = Duration::from_secs(2);
pub const ROUTE_TIMEOUT: Duration = Duration::from_secs(10);
pub const EVENT_TIMEOUT: Duration = Duration::from_secs(30);
const TICK: Duration = Duration::from_millis(10);
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
const PLUGIN_FAILED: &str = "plugin_failed";
const ALLOC_EXPORT: &str = "aphel_alloc";
const CALL_EXPORT: &str = "aphel_call";

pub struct CallState {
    pub plugin: Arc<LoadedPlugin>,
    pub scope: AccessScope,
    limits: StoreLimits,
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
        linker
            .func_wrap_async(
                "aphel",
                "host_call",
                |mut caller: Caller<'_, CallState>, (pointer, length): (i32, i32)| {
                    Box::new(async move {
                        let request = read_guest(&mut caller, pointer, length)?;
                        let plugin = caller.data().plugin.clone();
                        let scope = caller.data().scope.clone();
                        let response = host::dispatch(&plugin, &scope, request).await;
                        write_guest(&mut caller, &response).await
                    })
                },
            )
            .expect("the plugin host functions are defined once");
        linker
    })
}

pub async fn compile(bytes: Vec<u8>) -> Result<Module, AppError> {
    tokio::task::spawn_blocking(move || Module::new(engine(), &bytes))
        .await
        .map_err(|error| AppError::Internal(format!("plugin compilation panicked: {error}")))?
        .map_err(|error| {
            AppError::validation_code(
                "plugin_backend_invalid",
                format!("backend.wasm is not a valid WebAssembly module: {error}"),
            )
        })
}

pub async fn validate(bytes: Vec<u8>) -> Result<(), AppError> {
    let module = compile(bytes).await?;
    for export in [ALLOC_EXPORT, CALL_EXPORT, "memory"] {
        if module.get_export(export).is_none() {
            return Err(AppError::validation_code(
                "plugin_backend_invalid",
                format!("backend.wasm does not export {export}; build it with aphel-plugin-sdk"),
            ));
        }
    }
    Ok(())
}

fn pack(pointer: i32, length: usize) -> i64 {
    (((pointer as u32 as u64) << 32) | (length as u32 as u64)) as i64
}

fn unpack(value: i64) -> (usize, usize) {
    let value = value as u64;
    ((value >> 32) as usize, (value & 0xffff_ffff) as usize)
}

fn memory(caller: &mut Caller<'_, CallState>) -> wasmtime::Result<wasmtime::Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => Ok(memory),
        _ => Err(wasmtime::Error::msg(
            "the plugin does not export its memory",
        )),
    }
}

fn read_guest(
    caller: &mut Caller<'_, CallState>,
    pointer: i32,
    length: i32,
) -> wasmtime::Result<Value> {
    let length = usize::try_from(length).unwrap_or(usize::MAX);
    if length > MAX_MESSAGE_BYTES {
        return Err(wasmtime::Error::msg("the plugin message is too large"));
    }
    let memory = memory(caller)?;
    let start = pointer as u32 as usize;
    let bytes = memory
        .data(&*caller)
        .get(start..start + length)
        .ok_or_else(|| wasmtime::Error::msg("the plugin message is out of bounds"))?
        .to_vec();
    Ok(serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn write_guest(caller: &mut Caller<'_, CallState>, value: &Value) -> wasmtime::Result<i64> {
    let bytes = serde_json::to_vec(value)?;
    let alloc = match caller.get_export(ALLOC_EXPORT) {
        Some(Extern::Func(function)) => function.typed::<i32, i32>(&*caller)?,
        _ => {
            return Err(wasmtime::Error::msg(
                "the plugin does not export aphel_alloc",
            ));
        }
    };
    let pointer = alloc.call_async(&mut *caller, bytes.len() as i32).await?;
    memory(caller)?.write(&mut *caller, pointer as u32 as usize, &bytes)?;
    Ok(pack(pointer, bytes.len()))
}

pub async fn invoke(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    input: &Value,
    timeout: Duration,
) -> Result<Value, AppError> {
    let Some(module) = &plugin.module else {
        return Err(AppError::Configuration(format!(
            "the plugin {} has no backend",
            plugin.key
        )));
    };
    let failure = |error: wasmtime::Error| {
        tracing::warn!(plugin = %plugin.key, error = %error, "plugin call failed");
        AppError::unavailable_code(
            PLUGIN_FAILED,
            format!("the plugin {} failed to run", plugin.key),
        )
    };
    let mut store = Store::new(
        engine(),
        CallState {
            plugin: plugin.clone(),
            scope: scope.clone(),
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_LIMIT)
                .instances(1)
                .build(),
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_epoch_deadline((timeout.as_millis() / TICK.as_millis()).max(1) as u64);
    let instance = linker()
        .instantiate_async(&mut store, module)
        .await
        .map_err(failure)?;
    let alloc = instance
        .get_typed_func::<i32, i32>(&mut store, ALLOC_EXPORT)
        .map_err(failure)?;
    let call = instance
        .get_typed_func::<(i32, i32), i64>(&mut store, CALL_EXPORT)
        .map_err(failure)?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| failure(wasmtime::Error::msg("missing memory export")))?;
    let bytes = serde_json::to_vec(input).map_err(|error| AppError::Internal(error.to_string()))?;
    let pointer = alloc
        .call_async(&mut store, bytes.len() as i32)
        .await
        .map_err(failure)?;
    memory
        .write(&mut store, pointer as u32 as usize, &bytes)
        .map_err(|error| failure(error.into()))?;
    let packed = call
        .call_async(&mut store, (pointer, bytes.len() as i32))
        .await
        .map_err(failure)?;
    let (start, length) = unpack(packed);
    if length > MAX_MESSAGE_BYTES {
        return Err(failure(wasmtime::Error::msg(
            "the plugin answer is too large",
        )));
    }
    let output = memory
        .data(&store)
        .get(start..start + length)
        .ok_or_else(|| failure(wasmtime::Error::msg("the plugin answer is out of bounds")))?;
    let output: Value = serde_json::from_slice(output).map_err(|error| {
        failure(wasmtime::Error::msg(format!(
            "invalid plugin answer: {error}"
        )))
    })?;
    match output.get("error") {
        Some(error) => Err(plugin_error(error)),
        None => Ok(output.get("ok").cloned().unwrap_or(Value::Null)),
    }
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

pub fn plugin_error(error: &Value) -> AppError {
    let code = intern(
        error
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("plugin_error"),
    );
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("the plugin rejected the operation")
        .to_owned();
    let fields: Vec<FieldViolation> = error
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| {
                    Some(FieldViolation::new(
                        field.get("field")?.as_str()?,
                        field.get("message")?.as_str()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    if !fields.is_empty() {
        return AppError::invalid_fields_code(code, message, fields);
    }
    match error.get("status").and_then(Value::as_u64) {
        Some(404) => AppError::not_found_code(code, message),
        Some(403) => AppError::forbidden_code(code, message),
        Some(409) => AppError::conflict_code(code, message),
        _ => AppError::validation_code(code, message),
    }
}

pub fn error_json(error: &AppError) -> Value {
    let code = match error {
        AppError::CodedFields { code, .. }
        | AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. } => *code,
        AppError::NotFound(_) => "not_found",
        AppError::Forbidden(_) => "forbidden",
        AppError::Conflict(_) => "conflict",
        AppError::Validation(_) | AppError::InvalidFields { .. } => "validation_error",
        _ => "internal_error",
    };
    let fields: Vec<Value> = match error {
        AppError::CodedFields { fields, .. } | AppError::InvalidFields { fields, .. } => fields
            .iter()
            .map(|field| json!({ "field": field.field, "message": field.message }))
            .collect(),
        _ => Vec::new(),
    };
    json!({ "error": { "code": code, "message": error.client_message(), "fields": fields } })
}
