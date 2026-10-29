use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use aphel_kernel::errors::AppError;

const PUBLIC_KEY_BYTES: usize = 16;
const SIGNATURE_BYTES: usize = 16;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(value: &str) -> Option<Vec<u8>> {
    if value.len() != PUBLIC_KEY_BYTES * 2 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect()
}

fn mac(key: &[u8]) -> Result<Hmac<Sha256>, AppError> {
    Hmac::<Sha256>::new_from_slice(key)
        .map_err(|error| AppError::Internal(format!("invalid quote link key: {error}")))
}

pub(crate) fn generate_public_key() -> Result<String, AppError> {
    let mut bytes = [0u8; PUBLIC_KEY_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
    Ok(hex(&bytes))
}

pub(crate) fn encode_link_token(public_key: &str, key: &[u8]) -> Result<String, AppError> {
    let payload = unhex(public_key)
        .ok_or_else(|| AppError::Internal("invalid quote public key".to_owned()))?;
    let mut signer = mac(key)?;
    signer.update(&payload);
    let signature = signer.finalize().into_bytes();
    let mut bytes = Vec::with_capacity(PUBLIC_KEY_BYTES + SIGNATURE_BYTES);
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&signature[..SIGNATURE_BYTES]);
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignedLinkToken {
    pub(crate) public_key: String,
    payload: Vec<u8>,
    signature: Vec<u8>,
}

impl SignedLinkToken {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.len() > 64 {
            return None;
        }
        let bytes = URL_SAFE_NO_PAD.decode(raw).ok()?;
        if bytes.len() != PUBLIC_KEY_BYTES + SIGNATURE_BYTES {
            return None;
        }
        let (payload, signature) = bytes.split_at(PUBLIC_KEY_BYTES);
        Some(Self {
            public_key: hex(payload),
            payload: payload.to_vec(),
            signature: signature.to_vec(),
        })
    }

    pub(crate) fn verify(&self, key: &[u8]) -> bool {
        let Ok(mut verifier) = mac(key) else {
            return false;
        };
        verifier.update(&self.payload);
        verifier.verify_truncated_left(&self.signature).is_ok()
    }
}
