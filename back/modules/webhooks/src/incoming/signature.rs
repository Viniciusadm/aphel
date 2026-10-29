use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

pub(crate) const SIGNATURE_HEADER: &str = "x-aphel-signature";
pub(crate) const DELIVERY_HEADER: &str = "x-aphel-delivery";
pub(crate) const DEFAULT_TOLERANCE_SECONDS: u32 = 300;
pub(crate) const MIN_TOLERANCE_SECONDS: u32 = 30;
pub(crate) const MAX_TOLERANCE_SECONDS: u32 = 86_400;
const MAX_SIGNATURES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedSignature {
    pub(crate) timestamp: i64,
    pub(crate) signatures: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureFailure {
    Missing,
    Malformed,
    OutsideWindow,
    Mismatch,
}

impl SignatureFailure {
    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::Missing => "the X-Aphel-Signature header is required",
            Self::Malformed => "the X-Aphel-Signature header must look like t=<unix>,v1=<hex>",
            Self::OutsideWindow => "the signature timestamp is outside the accepted window",
            Self::Mismatch => "the signature does not match the request body",
        }
    }
}

pub(crate) fn parse_signature_header(header: &str) -> Result<ParsedSignature, SignatureFailure> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        let Some((name, value)) = part.trim().split_once('=') else {
            continue;
        };
        match name.trim() {
            "t" => {
                timestamp = Some(
                    value
                        .trim()
                        .parse::<i64>()
                        .map_err(|_| SignatureFailure::Malformed)?,
                );
            }
            "v1" if signatures.len() < MAX_SIGNATURES => {
                signatures.push(decode_hex(value.trim()).ok_or(SignatureFailure::Malformed)?);
            }
            _ => {}
        }
    }
    match timestamp {
        Some(timestamp) if !signatures.is_empty() => Ok(ParsedSignature {
            timestamp,
            signatures,
        }),
        _ => Err(SignatureFailure::Malformed),
    }
}

pub(crate) fn verify_signature(
    secret: &str,
    header: Option<&str>,
    body: &[u8],
    now: DateTime<Utc>,
    tolerance_seconds: u32,
) -> Result<ParsedSignature, SignatureFailure> {
    let header = header
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(SignatureFailure::Missing)?;
    let parsed = parse_signature_header(header)?;
    if (now.timestamp() - parsed.timestamp).abs() > i64::from(tolerance_seconds) {
        return Err(SignatureFailure::OutsideWindow);
    }
    let matches = parsed.signatures.iter().any(|candidate| {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(parsed.timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        mac.verify_slice(candidate).is_ok()
    });
    if matches {
        Ok(parsed)
    } else {
        Err(SignatureFailure::Mismatch)
    }
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.is_empty() || !value.len().is_multiple_of(2) || value.len() > 256 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect()
}
