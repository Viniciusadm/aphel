use serde_json::{Map, Value};
use uuid::Uuid;

use crate::incoming::{
    ActionFieldError, IncomingWebhook, RequestStatus, SignatureFailure, WEBHOOK_TOKEN_PREFIX,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::domain::access::{AccessScope, PermissionSet, RoleKey};

pub(crate) const INCOMING_WEBHOOK_DISABLED: &str = "incoming_webhook_disabled";
pub(crate) const INVALID_WEBHOOK_SIGNATURE: &str = "invalid_webhook_signature";
pub(crate) const WEBHOOK_REPLAYED: &str = "webhook_replayed";
pub(crate) const INVALID_WEBHOOK_PAYLOAD: &str = "invalid_webhook_payload";
pub(crate) const WEBHOOK_PAYLOAD_TOO_LARGE: &str = "webhook_payload_too_large";
pub(crate) const INCOMING_WEBHOOK_OWNER_UNAVAILABLE: &str = "incoming_webhook_owner_unavailable";
pub(crate) const INCOMING_WEBHOOK_PERMISSION_DENIED: &str = "incoming_webhook_permission_denied";
pub(crate) const WEBHOOK_MAPPING_FAILED: &str = "webhook_mapping_failed";
pub(crate) const INTERNAL_ERROR: &str = "internal_error";
pub(crate) const MAX_DELIVERY_ID_LENGTH: usize = 200;
pub(crate) const INCOMING_WEBHOOK_PROCESSED: &str = "incoming_webhook.processed";
pub(crate) const MAX_PROCESSED_BODY_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct IncomingDelivery {
    pub(crate) token: String,
    pub(crate) content_type: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) delivery_id: Option<String>,
    pub(crate) headers: Map<String, Value>,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) body_bytes: u64,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceiveFailure {
    pub(crate) http_status: u16,
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) fields: Vec<ActionFieldError>,
}

impl ReceiveFailure {
    pub(crate) fn new(http_status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            http_status,
            code,
            message: message.into(),
            fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceiveOutcome {
    pub(crate) request_id: Uuid,
    pub(crate) status: RequestStatus,
    pub(crate) failure: Option<ReceiveFailure>,
}

pub(crate) fn looks_like_token(token: &str) -> bool {
    token
        .strip_prefix(WEBHOOK_TOKEN_PREFIX)
        .is_some_and(|rest| {
            rest.len() >= 32 && rest.chars().all(|character| character.is_ascii_hexdigit())
        })
}

pub(crate) fn log_scope(webhook: &IncomingWebhook) -> Result<AccessScope, AppError> {
    Ok(AccessScope::new(
        webhook.created_by.clone(),
        RoleKey::new("system")?,
        Vec::new(),
        PermissionSet::default(),
    ))
}

pub(crate) fn signature_failure(failure: SignatureFailure) -> ReceiveFailure {
    ReceiveFailure::new(401, INVALID_WEBHOOK_SIGNATURE, failure.message())
}
