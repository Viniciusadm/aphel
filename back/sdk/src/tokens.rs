use crate::AppError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
const KEY_BYTES: usize = 32;
pub fn generate_key() -> Result<String, AppError> {
    let mut bytes = [0u8; KEY_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn decode_key(value: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value.trim())
        .ok()
        .filter(|bytes| bytes.len() == KEY_BYTES)
}
