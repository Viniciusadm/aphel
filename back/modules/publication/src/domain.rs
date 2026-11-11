use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use aphel_contract_publication::{RequestAction, RequestStatus};
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_sdk::platform::application::shared::RecordRef;

pub(crate) const ACTIVATE_JOB: &str = "publication.activate";
pub(crate) const COLLECT_JOB: &str = "publication.collect";
pub(crate) const COLLECT_CRON: &str = "0 30 3 * * *";
pub(crate) const APPROVALS_CONSUMER: &str = "publication.approvals";
pub(crate) const MAX_ATTEMPTS: u32 = 10;
pub(crate) const MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const RETAIN_DAYS: i64 = 90;
pub(crate) const RETAIN_COUNT: u32 = 50;
pub(crate) const MAX_NOTE_CHARS: usize = 280;
pub(crate) const MAX_IDEMPOTENCY_KEY_CHARS: usize = 80;
pub(crate) const MAX_SCHEDULE_DAYS: i64 = 366;

pub(crate) const DEFINITION_INVALID: &str = "publication_definition_invalid";
pub(crate) const NOT_PUBLISHABLE: &str = "publication_not_publishable";
pub(crate) const RECORD_NOT_FOUND: &str = "publication_record_not_found";
pub(crate) const REVISION_NOT_FOUND: &str = "publication_revision_not_found";
pub(crate) const REQUEST_NOT_FOUND: &str = "publication_request_not_found";
pub(crate) const REQUEST_NOT_OPEN: &str = "publication_request_not_open";
pub(crate) const REQUEST_NOT_FAILED: &str = "publication_request_not_failed";
pub(crate) const IDEMPOTENCY_CONFLICT: &str = "publication_idempotency_conflict";
pub(crate) const PAYLOAD_TOO_LARGE: &str = "publication_payload_too_large";
pub(crate) const REQUESTER_UNAUTHORIZED: &str = "publication_requester_unauthorized";
pub(crate) const APPROVALS_UNAVAILABLE: &str = "publication_approvals_unavailable";
pub(crate) const APPROVAL_REJECTED: &str = "approval_rejected";
pub(crate) const APPROVAL_WITHDRAWN: &str = "approval_cancelled";
pub(crate) const NOTHING_PUBLISHED: &str = "publication_nothing_published";

#[derive(Debug, Clone)]
pub(crate) struct Subject {
    pub(crate) active_revision_id: Option<Uuid>,
    pub(crate) generation: u64,
    pub(crate) first_published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Revision {
    pub(crate) id: Uuid,
    pub(crate) record: RecordRef,
    pub(crate) number: u32,
    pub(crate) format: u32,
    pub(crate) payload: Value,
    pub(crate) payload_hash: String,
    pub(crate) draft_version: DateTime<Utc>,
    pub(crate) note: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PublicationRequest {
    pub(crate) id: Uuid,
    pub(crate) record: RecordRef,
    pub(crate) action: RequestAction,
    pub(crate) revision_id: Option<Uuid>,
    pub(crate) generation: u64,
    pub(crate) expected_active_revision_id: Option<Uuid>,
    pub(crate) status: RequestStatus,
    pub(crate) run_at: Option<DateTime<Utc>>,
    pub(crate) requested_by: Option<String>,
    pub(crate) idempotency_key: String,
    pub(crate) input_hash: String,
    pub(crate) attempts: u32,
    pub(crate) reason_code: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
}

pub(crate) fn coded(kind: CodedKind, code: &'static str, message: impl Into<String>) -> AppError {
    AppError::Coded {
        kind,
        code,
        message: message.into(),
    }
}

pub(crate) fn not_publishable(record_type: &str) -> AppError {
    coded(
        CodedKind::Validation,
        NOT_PUBLISHABLE,
        format!("{record_type} does not take part in publication"),
    )
}

pub(crate) fn record_not_found() -> AppError {
    coded(CodedKind::NotFound, RECORD_NOT_FOUND, "record not found")
}

pub(crate) fn revision_not_found() -> AppError {
    coded(
        CodedKind::NotFound,
        REVISION_NOT_FOUND,
        "revision not found",
    )
}

pub(crate) fn request_not_found() -> AppError {
    coded(
        CodedKind::NotFound,
        REQUEST_NOT_FOUND,
        "publication request not found",
    )
}

pub(crate) fn error_code(error: &AppError) -> &str {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => code,
        AppError::Forbidden(_) => "permission_denied",
        AppError::NotFound(_) => "not_found",
        AppError::Validation(_) | AppError::InvalidFields { .. } => "validation_error",
        AppError::Conflict(_) => "conflict",
        _ => "internal_error",
    }
}

pub(crate) fn is_transient(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Internal(_)
            | AppError::Persistence(_)
            | AppError::Coded {
                kind: CodedKind::Unavailable | CodedKind::TooManyRequests,
                ..
            }
            | AppError::CodedDetails {
                kind: CodedKind::Unavailable | CodedKind::TooManyRequests,
                ..
            }
    )
}
