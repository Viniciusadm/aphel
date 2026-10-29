use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::errors::AppError;

pub const SIGNATURE_HEADER: &str = "X-Aphel-Signature";
pub const EVENT_HEADER: &str = "X-Aphel-Event";
pub const DELIVERY_HEADER: &str = "X-Aphel-Delivery";

pub fn sign_payload(secret: &str, timestamp: i64, body: &str) -> Result<String, AppError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|error| AppError::Internal(format!("invalid webhook signing key: {error}")))?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body.as_bytes());
    let digest = mac.finalize().into_bytes();
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("t={timestamp},v1={hex}"))
}
