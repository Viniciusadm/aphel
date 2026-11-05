use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::fields::EntityDefinition;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::shared::{RecordRef, RecordType};
use aphel_sdk::platform::domain::fields::FieldType;

pub(crate) const PRIVACY_REQUEST_RECORD_TYPE: RecordType =
    RecordType::from_static("privacy.privacy_request");
pub(crate) const RETENTION_POLICY_RECORD_TYPE: RecordType =
    RecordType::from_static("privacy.retention_policy");
pub(crate) const RETENTION_RUN_RECORD_TYPE: RecordType =
    RecordType::from_static("privacy.retention_run");

pub(crate) const PRIVACY_SUBJECT_SEARCHED: &str = "privacy.subject_searched";
pub(crate) const PRIVACY_REQUEST_CREATED: &str = "privacy_request.created";
pub(crate) const PRIVACY_REQUEST_CANCELLED: &str = "privacy_request.cancelled";
pub(crate) const PRIVACY_REQUEST_STARTED: &str = "privacy_request.started";
pub(crate) const PRIVACY_REQUEST_COMPLETED: &str = "privacy_request.completed";
pub(crate) const PRIVACY_REQUEST_FAILED: &str = "privacy_request.failed";
pub(crate) const PRIVACY_REQUEST_DOWNLOADED: &str = "privacy_request.downloaded";
pub(crate) const PRIVACY_EXPORT_EXPIRED: &str = "privacy_request.export_expired";

pub(crate) const RETENTION_POLICY_CREATED: &str = "retention_policy.created";
pub(crate) const RETENTION_POLICY_UPDATED: &str = "retention_policy.updated";
pub(crate) const RETENTION_POLICY_DELETED: &str = "retention_policy.deleted";
pub(crate) const RETENTION_POLICY_ENABLED: &str = "retention_policy.enabled";
pub(crate) const RETENTION_POLICY_DISABLED: &str = "retention_policy.disabled";
pub(crate) const RETENTION_RUN_REQUESTED: &str = "retention_run.requested";
pub(crate) const RETENTION_RUN_COMPLETED: &str = "retention_run.completed";
pub(crate) const RETENTION_RUN_FAILED: &str = "retention_run.failed";

pub(crate) const PRIVACY_REQUEST_RUN_JOB: &str = "privacy.request.run";
pub(crate) const PRIVACY_RETENTION_JOB: &str = "privacy.retention";
pub(crate) const PRIVACY_RETENTION_CRON: &str = "0 0 4 * * *";
pub(crate) const PRIVACY_RETENTION_RUN_JOB: &str = "privacy.retention.run";
pub(crate) const PRIVACY_RETENTION_SWEEP_JOB: &str = "privacy.retention.sweep";
pub(crate) const PRIVACY_RETENTION_SWEEP_CRON: &str = "0 */15 * * * *";

pub(crate) const PRIVACY_REQUEST_NOT_FOUND: &str = "privacy_request_not_found";
pub(crate) const PRIVACY_REQUEST_NOT_PENDING: &str = "privacy_request_not_pending";
pub(crate) const PRIVACY_EXPORT_UNAVAILABLE: &str = "privacy_export_unavailable";
pub(crate) const INVALID_SUBJECT_TERM: &str = "invalid_subject_term";
pub(crate) const PRIVACY_TARGETS_EMPTY: &str = "privacy_targets_empty";
pub(crate) const RETENTION_POLICY_NOT_FOUND: &str = "retention_policy_not_found";
pub(crate) const RETENTION_PREVIEW_REQUIRED: &str = "retention_preview_required";
pub(crate) const RETENTION_POLICY_LIMIT: &str = "retention_policy_limit";

pub(crate) const DEFAULT_DUE_DAYS: i64 = 15;
pub(crate) const MAX_DUE_DAYS: i64 = 365;
pub(crate) const EXPORT_FILE_TTL_DAYS: i64 = 30;
pub(crate) const PREVIEW_VALIDITY_MINUTES: i64 = 60;
pub(crate) const SEARCH_LIMIT: u32 = 100;
pub(crate) const MAX_TARGETS: usize = 500;
pub(crate) const MAX_POLICIES: u64 = 50;
pub(crate) const MIN_THRESHOLD_DAYS: u32 = 1;
pub(crate) const MAX_THRESHOLD_DAYS: u32 = 36_500;
pub(crate) const MAX_EXPORT_ATTACHMENT_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) fn privacy_request_not_found() -> AppError {
    AppError::not_found_code(PRIVACY_REQUEST_NOT_FOUND, "privacy request was not found")
}

pub(crate) fn retention_policy_not_found() -> AppError {
    AppError::not_found_code(RETENTION_POLICY_NOT_FOUND, "retention policy was not found")
}

fn invalid(field: &str, message: impl Into<String>) -> AppError {
    AppError::invalid_fields(
        "invalid privacy data",
        vec![FieldViolation::new(field, message)],
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrivacyRequestKind {
    Export,
    Anonymize,
    Delete,
}

impl PrivacyRequestKind {
    pub(crate) const ALL: [Self; 3] = [Self::Export, Self::Anonymize, Self::Delete];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Anonymize => "anonymize",
            Self::Delete => "delete",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, AppError> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value.trim())
            .ok_or_else(|| invalid("kind", "must be export, anonymize or delete"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrivacyRequestStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl PrivacyRequestStatus {
    pub(crate) const ALL: [Self; 5] = [
        Self::Pending,
        Self::Processing,
        Self::Completed,
        Self::Failed,
        Self::Cancelled,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub(crate) fn is_open(&self) -> bool {
        matches!(self, Self::Pending | Self::Processing)
    }
}

pub(crate) use aphel_sdk::privacy::SubjectKind;

pub(crate) use aphel_sdk::privacy::SourceTarget;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PrivacyTargets {
    pub(crate) records: Vec<RecordRef>,
    pub(crate) items: Vec<SourceTarget>,
}
impl PrivacyTargets {
    pub(crate) fn is_empty(&self) -> bool {
        self.records.is_empty() && self.items.is_empty()
    }
    pub(crate) fn len(&self) -> usize {
        self.records.len() + self.items.len()
    }
    pub(crate) fn to_json(&self) -> Value {
        json!({"records":self.records.iter().map(|record|json!({"record_type":record.record_type.as_str(),"id":record.record_id.to_string()})).collect::<Vec<_>>(),"items":self.items})
    }
    pub(crate) fn from_json(value: &Value) -> Result<Self, AppError> {
        let malformed = || AppError::Internal("invalid stored privacy targets".to_owned());
        let records = value
            .get("records")
            .and_then(Value::as_array)
            .ok_or_else(malformed)?
            .iter()
            .map(|item| {
                RecordRef::parse(
                    item.get("record_type")
                        .and_then(Value::as_str)
                        .ok_or_else(malformed)?,
                    item.get("id")
                        .and_then(Value::as_str)
                        .ok_or_else(malformed)?,
                )
                .map_err(|_| malformed())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let items: Vec<aphel_sdk::privacy::SourceTarget> =
            serde_json::from_value(value.get("items").cloned().ok_or_else(malformed)?)
                .map_err(|_| malformed())?;
        if items.iter().any(|item| {
            item.source.is_empty()
                || item.source.len() > 64
                || !item.source.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'.' | b'-')
                })
                || item.id.is_empty()
                || item.id.len() > 200
                || item.id.chars().any(char::is_control)
        }) {
            return Err(malformed());
        }
        Ok(Self { records, items })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PrivacyRequest {
    pub(crate) id: Uuid,
    pub(crate) kind: PrivacyRequestKind,
    pub(crate) status: PrivacyRequestStatus,
    pub(crate) subject_kind: SubjectKind,
    pub(crate) subject_label: String,
    pub(crate) subject_hash: String,
    pub(crate) targets: PrivacyTargets,
    pub(crate) due_at: DateTime<Utc>,
    pub(crate) requested_by: Option<UserReference>,
    pub(crate) executed_by: Option<UserReference>,
    pub(crate) result: Option<Value>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) file_id: Option<Uuid>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

impl PrivacyRequest {
    pub(crate) fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        self.status.is_open() && self.due_at < now
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetLabel {
    pub(crate) record: RecordRef,
    pub(crate) title: String,
    pub(crate) exists: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PrivacyRequestDetail {
    pub(crate) request: PrivacyRequest,
    pub(crate) records: Vec<TargetLabel>,
    pub(crate) sources: Vec<PrivacySourceInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PrivacyRequestFilter {
    pub(crate) status: Option<PrivacyRequestStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FieldSource {
    Column(String),
    Custom(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersonalField {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field_type: FieldType,
    pub(crate) required: bool,
    pub(crate) sensitive: bool,
    pub(crate) source: FieldSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubjectRecord {
    pub(crate) record: RecordRef,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) matched_fields: Vec<String>,
    pub(crate) archived: bool,
    pub(crate) deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachmentData {
    pub(crate) id: Uuid,
    pub(crate) file_id: Uuid,
    pub(crate) storage_key: String,
    pub(crate) file_name: String,
    pub(crate) mime_type: String,
    pub(crate) size: u64,
    pub(crate) record: RecordRef,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrivacySourceInfo {
    pub(crate) key: String,
    pub(crate) labels: std::collections::BTreeMap<String, String>,
}
#[derive(Debug, Clone)]
pub(crate) struct SubjectSearchResult {
    pub(crate) kind: SubjectKind,
    pub(crate) label: String,
    pub(crate) records: Vec<SubjectRecord>,
    pub(crate) items: Vec<aphel_sdk::privacy::PrivacyItem>,
    pub(crate) sources: Vec<PrivacySourceInfo>,
    pub(crate) attachments: Vec<AttachmentData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredFileRef {
    pub(crate) file_id: Uuid,
    pub(crate) storage_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatePrivacyRequestCommand {
    pub(crate) kind: String,
    pub(crate) term: String,
    pub(crate) targets: PrivacyTargets,
    pub(crate) due_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionAction {
    ArchiveInactive,
    PurgeArchived,
    PurgeAuditLogs,
    PurgeSecurityEvents,
    PurgeDomainEvents,
    PurgeOrphanFiles,
}

impl RetentionAction {
    pub(crate) const ALL: [Self; 6] = [
        Self::ArchiveInactive,
        Self::PurgeArchived,
        Self::PurgeAuditLogs,
        Self::PurgeSecurityEvents,
        Self::PurgeDomainEvents,
        Self::PurgeOrphanFiles,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::ArchiveInactive => "archive_inactive",
            Self::PurgeArchived => "purge_archived",
            Self::PurgeAuditLogs => "purge_audit_logs",
            Self::PurgeSecurityEvents => "purge_security_events",
            Self::PurgeDomainEvents => "purge_domain_events",
            Self::PurgeOrphanFiles => "purge_orphan_files",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|action| action.as_str() == value)
    }

    pub(crate) fn needs_record_type(&self) -> bool {
        matches!(self, Self::ArchiveInactive | Self::PurgeArchived)
    }
}

pub(crate) fn supports_retention(definition: &EntityDefinition) -> bool {
    definition.deleted_column.is_some() && definition.archived_column.is_some()
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RetentionPolicy {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) action: RetentionAction,
    pub(crate) record_type: Option<String>,
    pub(crate) threshold_days: u32,
    pub(crate) is_enabled: bool,
    pub(crate) previewed_count: Option<u64>,
    pub(crate) previewed_at: Option<DateTime<Utc>>,
    pub(crate) last_run_at: Option<DateTime<Utc>>,
    pub(crate) created_by: Option<UserReference>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl RetentionPolicy {
    pub(crate) fn threshold(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - chrono::Duration::days(i64::from(self.threshold_days))
    }

    pub(crate) fn has_fresh_preview(&self, now: DateTime<Utc>) -> bool {
        self.previewed_at.is_some_and(|previewed| {
            previewed >= self.updated_at
                && now - previewed <= chrono::Duration::minutes(PREVIEW_VALIDITY_MINUTES)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetentionPolicyCommand {
    pub(crate) name: Option<String>,
    pub(crate) action: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) threshold_days: Option<u32>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetentionPreview {
    pub(crate) affected: u64,
    pub(crate) threshold_at: DateTime<Utc>,
    pub(crate) previewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionTrigger {
    Schedule,
    Manual,
}

impl RetentionTrigger {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Manual => "manual",
        }
    }

    pub(crate) fn parse(value: &str) -> Self {
        if value == "manual" {
            Self::Manual
        } else {
            Self::Schedule
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionRunStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl RetentionRunStatus {
    pub(crate) const ALL: [Self; 4] = [Self::Queued, Self::Running, Self::Completed, Self::Failed];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub(crate) fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RetentionRun {
    pub(crate) id: Uuid,
    pub(crate) policy_id: Option<Uuid>,
    pub(crate) policy_name: String,
    pub(crate) action: RetentionAction,
    pub(crate) record_type: Option<String>,
    pub(crate) threshold_days: u32,
    pub(crate) trigger: RetentionTrigger,
    pub(crate) status: RetentionRunStatus,
    pub(crate) affected: u64,
    pub(crate) files_removed: u64,
    pub(crate) errors: u32,
    pub(crate) error_message: Option<String>,
    pub(crate) requested_by: Option<UserReference>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

impl RetentionRun {
    pub(crate) fn for_policy(
        policy: &RetentionPolicy,
        trigger: RetentionTrigger,
        requested_by: Option<UserReference>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            policy_id: Some(policy.id),
            policy_name: policy.name.clone(),
            action: policy.action,
            record_type: policy.record_type.clone(),
            threshold_days: policy.threshold_days,
            trigger,
            status: RetentionRunStatus::Queued,
            affected: 0,
            files_removed: 0,
            errors: 0,
            error_message: None,
            requested_by,
            created_at: now,
            started_at: None,
            finished_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RetentionRunFilter {
    pub(crate) policy_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RunReport {
    pub(crate) records: u64,
    pub(crate) source_items: std::collections::BTreeMap<String, u64>,
    pub(crate) attachments: u64,
    pub(crate) files: u64,
    pub(crate) history_entries: u64,
    pub(crate) external_items: u64,
    pub(crate) skipped: u64,
    pub(crate) errors: u64,
}
impl RunReport {
    pub(crate) fn to_json(&self) -> Value {
        json!({"records":self.records,"source_items":self.source_items,"attachments":self.attachments,"files":self.files,"history_entries":self.history_entries,"external_items":self.external_items,"skipped":self.skipped,"errors":self.errors})
    }
}
