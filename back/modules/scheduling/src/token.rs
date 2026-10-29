use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

use aphel_kernel::errors::AppError;

pub(crate) const TOKEN_KEY_BYTES: usize = 32;
const PAYLOAD_BYTES: usize = 16;
const SIGNATURE_BYTES: usize = 16;
pub(crate) const MANAGE_PATH: &str = "/book/manage";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManageToken {
    pub(crate) booking_id: Uuid,
}

impl ManageToken {
    fn payload(&self) -> [u8; PAYLOAD_BYTES] {
        *self.booking_id.as_bytes()
    }

    pub(crate) fn encode(&self, key: &[u8]) -> Result<String, AppError> {
        let payload = self.payload();
        let mut mac = Hmac::<Sha256>::new_from_slice(key)
            .map_err(|error| AppError::Internal(format!("invalid booking token key: {error}")))?;
        mac.update(&payload);
        let signature = mac.finalize().into_bytes();
        let mut bytes = Vec::with_capacity(PAYLOAD_BYTES + SIGNATURE_BYTES);
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(&signature[..SIGNATURE_BYTES]);
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn url(&self, base_url: &str, key: &[u8]) -> Result<String, AppError> {
        Ok(format!(
            "{}{MANAGE_PATH}/{}",
            base_url.trim_end_matches('/'),
            self.encode(key)?
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignedManageToken {
    pub(crate) token: ManageToken,
    payload: Vec<u8>,
    signature: Vec<u8>,
}

impl SignedManageToken {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.len() > 96 {
            return None;
        }
        let bytes = URL_SAFE_NO_PAD.decode(raw).ok()?;
        if bytes.len() != PAYLOAD_BYTES + SIGNATURE_BYTES {
            return None;
        }
        let (payload, signature) = bytes.split_at(PAYLOAD_BYTES);
        Some(Self {
            token: ManageToken {
                booking_id: Uuid::from_slice(payload).ok()?,
            },
            payload: payload.to_vec(),
            signature: signature.to_vec(),
        })
    }

    pub(crate) fn verify(&self, key: &[u8]) -> bool {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(key) else {
            return false;
        };
        mac.update(&self.payload);
        mac.verify_truncated_left(&self.signature).is_ok()
    }
}

pub fn generate_key() -> Result<String, AppError> {
    let mut bytes = [0u8; TOKEN_KEY_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn decode_key(value: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value.trim())
        .ok()
        .filter(|bytes| bytes.len() == TOKEN_KEY_BYTES)
}
