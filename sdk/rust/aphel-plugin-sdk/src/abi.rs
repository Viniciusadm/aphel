use serde_json::{Value, json};

use crate::{Ctx, Event, HookCall, Plugin, PluginError, Request};

#[link(wasm_import_module = "aphel")]
unsafe extern "C" {
    fn host_call(pointer: i32, length: i32) -> i64;
}

pub fn alloc(length: i32) -> i32 {
    let mut buffer = Vec::<u8>::with_capacity(length.max(0) as usize);
    let pointer = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    pointer as i32
}

fn pack(bytes: Vec<u8>) -> i64 {
    let length = bytes.len();
    let pointer = bytes.leak().as_ptr() as u32 as u64;
    ((pointer << 32) | length as u64) as i64
}

fn unpack(value: i64) -> &'static [u8] {
    let value = value as u64;
    let pointer = (value >> 32) as usize as *const u8;
    let length = (value & 0xffff_ffff) as usize;
    unsafe { std::slice::from_raw_parts(pointer, length) }
}

pub(crate) fn call_host(request: &Value) -> Result<Value, PluginError> {
    let bytes =
        serde_json::to_vec(request).map_err(|error| PluginError::internal(error.to_string()))?;
    let packed = unsafe { host_call(bytes.as_ptr() as i32, bytes.len() as i32) };
    let response: Value = serde_json::from_slice(unpack(packed))
        .map_err(|error| PluginError::internal(error.to_string()))?;
    match response.get("error") {
        Some(error) => Err(serde_json::from_value(error.clone())
            .unwrap_or_else(|_| PluginError::internal("the host failed"))),
        None => Ok(response.get("ok").cloned().unwrap_or(Value::Null)),
    }
}

fn run<P: Plugin>(plugin: &P, input: Value) -> Result<Value, PluginError> {
    let ctx = Ctx::new();
    match input.get("kind").and_then(Value::as_str) {
        Some("hook") => {
            let call: HookCall = serde_json::from_value(input)
                .map_err(|error| PluginError::internal(format!("invalid hook call: {error}")))?;
            let outcome = plugin.on_hook(&ctx, &call)?;
            Ok(json!({ "values": outcome.values, "custom_fields": outcome.custom_fields }))
        }
        Some("event") => {
            let event: Event =
                serde_json::from_value(input.get("event").cloned().unwrap_or_default())
                    .map_err(|error| PluginError::internal(format!("invalid event: {error}")))?;
            plugin.on_event(&ctx, &event)?;
            Ok(Value::Null)
        }
        Some("route") => {
            let request: Request = serde_json::from_value(input)
                .map_err(|error| PluginError::internal(format!("invalid request: {error}")))?;
            let response = plugin.on_route(&ctx, &request)?;
            Ok(json!({ "status": response.status, "body": response.body }))
        }
        _ => Err(PluginError::internal("unknown call kind")),
    }
}

pub fn call<P: Plugin>(plugin: &P, pointer: i32, length: i32) -> i64 {
    let input =
        unsafe { std::slice::from_raw_parts(pointer as usize as *const u8, length as usize) };
    let output = match serde_json::from_slice::<Value>(input) {
        Ok(input) => match run(plugin, input) {
            Ok(value) => json!({ "ok": value }),
            Err(error) => json!({ "error": error }),
        },
        Err(error) => json!({ "error": PluginError::internal(error.to_string()) }),
    };
    pack(serde_json::to_vec(&output).unwrap_or_else(|_| b"{\"ok\":null}".to_vec()))
}
