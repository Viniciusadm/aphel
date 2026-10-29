use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::tracking::TrackingKind;
use aphel_kernel::errors::AppError;

pub(crate) const TRACKING_KEY_BYTES: usize = 32;
const OPEN_MARKER: u16 = u16::MAX;
const PAYLOAD_BYTES: usize = 18;
const SIGNATURE_BYTES: usize = 16;
pub(crate) const OPEN_PATH: &str = "/api/v1/e/o";
pub(crate) const CLICK_PATH: &str = "/api/v1/e/c";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrackingToken {
    pub(crate) email_id: Uuid,
    pub(crate) kind: TrackingKind,
}

impl TrackingToken {
    fn payload(&self) -> [u8; PAYLOAD_BYTES] {
        let mut payload = [0u8; PAYLOAD_BYTES];
        payload[..16].copy_from_slice(self.email_id.as_bytes());
        let marker = match self.kind {
            TrackingKind::Open => OPEN_MARKER,
            TrackingKind::Click(position) => position.min(OPEN_MARKER - 1),
        };
        payload[16..].copy_from_slice(&marker.to_be_bytes());
        payload
    }

    pub(crate) fn encode(&self, key: &[u8]) -> Result<String, AppError> {
        let payload = self.payload();
        let signature = sign(key, &payload)?;
        let mut bytes = Vec::with_capacity(PAYLOAD_BYTES + SIGNATURE_BYTES);
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(&signature[..SIGNATURE_BYTES]);
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn url(&self, base_url: &str, key: &[u8]) -> Result<String, AppError> {
        let path = match self.kind {
            TrackingKind::Open => OPEN_PATH,
            TrackingKind::Click(_) => CLICK_PATH,
        };
        Ok(format!(
            "{}{path}/{}",
            base_url.trim_end_matches('/'),
            self.encode(key)?
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignedToken {
    pub(crate) token: TrackingToken,
    payload: Vec<u8>,
    signature: Vec<u8>,
}

impl SignedToken {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.len() > 128 {
            return None;
        }
        let bytes = URL_SAFE_NO_PAD.decode(raw).ok()?;
        if bytes.len() != PAYLOAD_BYTES + SIGNATURE_BYTES {
            return None;
        }
        let (payload, signature) = bytes.split_at(PAYLOAD_BYTES);
        let email_id = Uuid::from_slice(&payload[..16]).ok()?;
        let marker = u16::from_be_bytes([payload[16], payload[17]]);
        let kind = if marker == OPEN_MARKER {
            TrackingKind::Open
        } else {
            TrackingKind::Click(marker)
        };
        Some(Self {
            token: TrackingToken { email_id, kind },
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

fn sign(key: &[u8], payload: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|error| AppError::Internal(format!("invalid e-mail tracking key: {error}")))?;
    mac.update(payload);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub(crate) fn generate_key() -> Result<String, AppError> {
    let mut bytes = [0u8; TRACKING_KEY_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub(crate) fn decode_key(value: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value.trim())
        .ok()
        .filter(|bytes| bytes.len() == TRACKING_KEY_BYTES)
}
