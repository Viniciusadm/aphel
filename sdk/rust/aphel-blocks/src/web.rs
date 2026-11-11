#![allow(clippy::missing_safety_doc)]

use serde_json::{Value, json};

use crate::{BlockError, blocks, slug};

#[unsafe(no_mangle)]
pub extern "C" fn blocks_alloc(length: usize) -> *mut u8 {
    let mut buffer = std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(length));
    buffer.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blocks_free(pointer: *mut u8, length: usize) {
    if !pointer.is_null() {
        drop(unsafe { Vec::from_raw_parts(pointer, 0, length) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blocks_validate(pointer: *const u8, length: usize) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(pointer, length) };
    let mut answer = std::mem::ManuallyDrop::new(handle(input).to_string().into_bytes());
    answer.shrink_to_fit();
    let length = answer.len();
    let pointer = answer.as_mut_ptr();
    ((pointer as u64) << 32) | length as u64
}

fn failure(error: BlockError) -> Value {
    json!({ "ok": false, "error": { "code": error.code, "field": error.field, "message": error.message } })
}

fn answer(result: Result<Value, BlockError>) -> Value {
    match result {
        Ok(normalized) => json!({ "ok": true, "normalized": normalized }),
        Err(error) => failure(error),
    }
}

fn found(result: Option<(&str, String)>) -> Value {
    match result {
        Some((provider, id)) => {
            json!({ "ok": true, "normalized": { "provider": provider, "id": id } })
        }
        None => json!({ "ok": false }),
    }
}

fn handle(input: &[u8]) -> Value {
    let Ok(request) = serde_json::from_slice::<Value>(input) else {
        return failure(BlockError::new(
            "block_invalid",
            "blocks",
            "the request is not valid JSON",
        ));
    };
    let value = &request["value"];
    let text = value.as_str().unwrap_or_default();
    match request["kind"].as_str().unwrap_or_default() {
        "blocks" => answer(blocks::normalize(text).map(Value::String)),
        "block" => answer(blocks::block(value, 0).map(Option::unwrap_or_default)),
        "slug" => answer(slug::check(text).map(Value::String)),
        "suggest_slug" => {
            json!({ "ok": true, "normalized": slug::suggest(text, request["fallback"].as_str().unwrap_or("item")) })
        }
        "link" => json!({ "ok": text.trim().is_empty() || blocks::is_safe_link(text.trim()) }),
        "video" => found(blocks::video_of(text)),
        "embed" => found(blocks::embed_of(text)),
        "email" => answer(blocks::email(Some(value))),
        "phone" => answer(blocks::phone(Some(value))),
        _ => failure(BlockError::new(
            "block_invalid",
            "blocks",
            "unknown validation kind",
        )),
    }
}
