use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    AttachmentData, CreatePrivacyRequestCommand, ImportRowTarget, PrivacyRequest,
    PrivacyRequestDetail, PrivacyRequestFilter, PrivacyRequestStatus, PrivacyTargets,
    RetentionPolicy, RetentionPolicyCommand, RetentionPreview, RetentionRun, RetentionRunFilter,
    SubjectActivity, SubjectImportRow, SubjectNote, SubjectRecord, SubjectSearchResult,
};
use aphel_kernel::api::dto::common::{OwnerResponse, timestamp};
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::errors::{AppError, FieldViolation};

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct SubjectSearchRequest {
    #[schema(example = "maria@example.com")]
    pub(crate) term: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordRefResponse {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) id: String,
}

impl From<&RecordRef> for RecordRefResponse {
    fn from(record: &RecordRef) -> Self {
        Self {
            record_type: record.record_type.as_str().to_owned(),
            id: record.record_id.to_string(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SubjectRecordResponse {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) matched_fields: Vec<String>,
    pub(crate) archived: bool,
    pub(crate) deleted: bool,
}

impl From<SubjectRecord> for SubjectRecordResponse {
    fn from(record: SubjectRecord) -> Self {
        Self {
            record_type: record.record.record_type.as_str().to_owned(),
            id: record.record.record_id.to_string(),
            title: record.title,
            subtitle: record.subtitle,
            matched_fields: record.matched_fields,
            archived: record.archived,
            deleted: record.deleted,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SubjectNoteResponse {
    pub(crate) id: String,
    pub(crate) record: RecordRefResponse,
    pub(crate) excerpt: String,
    pub(crate) created_at: String,
    #[schema(example = "mention")]
    pub(crate) reason: String,
}

impl From<SubjectNote> for SubjectNoteResponse {
    fn from(note: SubjectNote) -> Self {
        Self {
            id: note.id.to_string(),
            record: (&note.record).into(),
            excerpt: note.excerpt,
            created_at: timestamp(note.created_at),
            reason: note.reason.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SubjectAttachmentResponse {
    pub(crate) id: String,
    pub(crate) file_id: String,
    pub(crate) file_name: String,
    pub(crate) mime_type: String,
    pub(crate) size: u64,
    pub(crate) record: RecordRefResponse,
    pub(crate) created_at: String,
}

impl From<AttachmentData> for SubjectAttachmentResponse {
    fn from(attachment: AttachmentData) -> Self {
        Self {
            id: attachment.id.to_string(),
            file_id: attachment.file_id.to_string(),
            file_name: attachment.file_name,
            mime_type: attachment.mime_type,
            size: attachment.size,
            record: (&attachment.record).into(),
            created_at: timestamp(attachment.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SubjectActivityResponse {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) record: Option<RecordRefResponse>,
    pub(crate) due_at: Option<String>,
    pub(crate) reason: String,
}

impl From<SubjectActivity> for SubjectActivityResponse {
    fn from(activity: SubjectActivity) -> Self {
        Self {
            id: activity.id.to_string(),
            title: activity.title,
            status: activity.status,
            record: activity.record.as_ref().map(Into::into),
            due_at: activity.due_at.map(timestamp),
            reason: activity.reason.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SubjectImportRowResponse {
    pub(crate) import_job_id: String,
    pub(crate) row_index: u32,
    pub(crate) record_type: String,
    pub(crate) record_id: Option<String>,
    pub(crate) status: String,
    pub(crate) file_name: Option<String>,
    pub(crate) created_at: String,
}

impl From<SubjectImportRow> for SubjectImportRowResponse {
    fn from(row: SubjectImportRow) -> Self {
        Self {
            import_job_id: row.target.import_job_id.to_string(),
            row_index: row.target.row_index,
            record_type: row.record_type,
            record_id: row.record_id.map(|id| id.to_string()),
            status: row.status,
            file_name: row.file_name,
            created_at: timestamp(row.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SubjectSearchResponse {
    #[schema(example = "email")]
    pub(crate) kind: String,
    #[schema(example = "m***@e***.com")]
    pub(crate) label: String,
    pub(crate) records: Vec<SubjectRecordResponse>,
    pub(crate) notes: Vec<SubjectNoteResponse>,
    pub(crate) attachments: Vec<SubjectAttachmentResponse>,
    pub(crate) activities: Vec<SubjectActivityResponse>,
    pub(crate) import_rows: Vec<SubjectImportRowResponse>,
}

impl From<SubjectSearchResult> for SubjectSearchResponse {
    fn from(result: SubjectSearchResult) -> Self {
        Self {
            kind: result.kind.as_str().to_owned(),
            label: result.label,
            records: result.records.into_iter().map(Into::into).collect(),
            notes: result.notes.into_iter().map(Into::into).collect(),
            attachments: result.attachments.into_iter().map(Into::into).collect(),
            activities: result.activities.into_iter().map(Into::into).collect(),
            import_rows: result.import_rows.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct RecordRefBody {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ImportRowBody {
    pub(crate) import_job_id: String,
    pub(crate) row_index: u32,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PrivacyTargetsBody {
    #[serde(default)]
    pub(crate) records: Vec<RecordRefBody>,
    #[serde(default)]
    pub(crate) notes: Vec<String>,
    #[serde(default)]
    pub(crate) import_rows: Vec<ImportRowBody>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreatePrivacyRequestBody {
    #[schema(example = "export")]
    pub(crate) kind: String,
    #[schema(example = "maria@example.com")]
    pub(crate) term: String,
    #[serde(default)]
    pub(crate) targets: PrivacyTargetsBody,
    pub(crate) due_at: Option<String>,
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid privacy request",
        vec![FieldViolation::new(field, message)],
    )
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| invalid(field, "must be a UUID"))
}

impl CreatePrivacyRequestBody {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
    ) -> Result<CreatePrivacyRequestCommand, AppError> {
        let records = self
            .targets
            .records
            .iter()
            .map(|record| {
                RecordRef::parse(record.record_type.trim(), record.id.trim())
                    .map_err(|_| invalid("targets.records", "must reference valid records"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let notes = self
            .targets
            .notes
            .iter()
            .map(|id| parse_uuid(id, "targets.notes"))
            .collect::<Result<Vec<_>, _>>()?;
        let import_rows = self
            .targets
            .import_rows
            .iter()
            .map(|row| {
                Ok(ImportRowTarget {
                    import_job_id: parse_uuid(&row.import_job_id, "targets.import_rows")?,
                    row_index: row.row_index,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let due_at = match self.due_at.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => Some(
                DateTime::parse_from_rfc3339(value)
                    .map(|value| value.with_timezone(&Utc))
                    .map_err(|_| invalid("due_at", "must be an RFC 3339 date-time"))?,
            ),
        };
        Ok(CreatePrivacyRequestCommand {
            kind: self.kind,
            term: self.term,
            targets: PrivacyTargets {
                records,
                notes,
                import_rows,
            },
            due_at,
            ip,
        })
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PrivacyTargetCountsResponse {
    pub(crate) records: usize,
    pub(crate) notes: usize,
    pub(crate) import_rows: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PrivacyReportResponse {
    pub(crate) records: u64,
    pub(crate) notes: u64,
    pub(crate) attachments: u64,
    pub(crate) files: u64,
    pub(crate) activities: u64,
    pub(crate) import_rows: u64,
    pub(crate) history_entries: u64,
    pub(crate) external_items: u64,
    pub(crate) skipped: u64,
    pub(crate) errors: u64,
}

impl From<&Value> for PrivacyReportResponse {
    fn from(value: &Value) -> Self {
        let number = |key: &str| value.get(key).and_then(Value::as_u64).unwrap_or(0);
        Self {
            records: number("records"),
            notes: number("notes"),
            attachments: number("attachments"),
            files: number("files"),
            activities: number("activities"),
            import_rows: number("import_rows"),
            history_entries: number("history_entries"),
            external_items: number("external_items"),
            skipped: number("skipped"),
            errors: number("errors"),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PrivacyRequestResponse {
    pub(crate) id: String,
    #[schema(example = "export")]
    pub(crate) kind: String,
    #[schema(example = "pending")]
    pub(crate) status: String,
    #[schema(example = "email")]
    pub(crate) subject_kind: String,
    #[schema(example = "m***@e***.com")]
    pub(crate) subject_label: String,
    pub(crate) due_at: String,
    pub(crate) overdue: bool,
    pub(crate) targets: PrivacyTargetCountsResponse,
    pub(crate) requested_by: Option<OwnerResponse>,
    pub(crate) executed_by: Option<OwnerResponse>,
    pub(crate) result: Option<PrivacyReportResponse>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) download_available: bool,
    pub(crate) created_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) finished_at: Option<String>,
}

impl From<PrivacyRequest> for PrivacyRequestResponse {
    fn from(request: PrivacyRequest) -> Self {
        Self {
            id: request.id.to_string(),
            kind: request.kind.as_str().to_owned(),
            status: request.status.as_str().to_owned(),
            subject_kind: request.subject_kind.as_str().to_owned(),
            subject_label: request.subject_label.clone(),
            due_at: timestamp(request.due_at),
            overdue: request.is_overdue(Utc::now()),
            targets: PrivacyTargetCountsResponse {
                records: request.targets.records.len(),
                notes: request.targets.notes.len(),
                import_rows: request.targets.import_rows.len(),
            },
            requested_by: request.requested_by.map(Into::into),
            executed_by: request.executed_by.map(Into::into),
            result: request.result.as_ref().map(Into::into),
            failure_reason: request.failure_reason,
            download_available: request.file_id.is_some()
                && request.status == PrivacyRequestStatus::Completed,
            created_at: timestamp(request.created_at),
            started_at: request.started_at.map(timestamp),
            finished_at: request.finished_at.map(timestamp),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PrivacyTargetRecordResponse {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) exists: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PrivacyImportRowResponse {
    pub(crate) import_job_id: String,
    pub(crate) row_index: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PrivacyRequestDetailResponse {
    pub(crate) request: PrivacyRequestResponse,
    pub(crate) records: Vec<PrivacyTargetRecordResponse>,
    pub(crate) notes: Vec<String>,
    pub(crate) import_rows: Vec<PrivacyImportRowResponse>,
}

impl From<PrivacyRequestDetail> for PrivacyRequestDetailResponse {
    fn from(detail: PrivacyRequestDetail) -> Self {
        let notes = detail
            .request
            .targets
            .notes
            .iter()
            .map(Uuid::to_string)
            .collect();
        let import_rows = detail
            .request
            .targets
            .import_rows
            .iter()
            .map(|row| PrivacyImportRowResponse {
                import_job_id: row.import_job_id.to_string(),
                row_index: row.row_index,
            })
            .collect();
        Self {
            records: detail
                .records
                .into_iter()
                .map(|label| PrivacyTargetRecordResponse {
                    record_type: label.record.record_type.as_str().to_owned(),
                    id: label.record.record_id.to_string(),
                    title: label.title,
                    exists: label.exists,
                })
                .collect(),
            notes,
            import_rows,
            request: detail.request.into(),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PrivacyRequestListParams {
    #[param(example = "pending")]
    pub(crate) status: Option<String>,
}

impl PrivacyRequestListParams {
    pub(crate) fn into_filter(self) -> Result<PrivacyRequestFilter, AppError> {
        let status = match self.status.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => Some(PrivacyRequestStatus::parse(value).ok_or_else(|| {
                invalid(
                    "status",
                    "must be pending, processing, completed, failed or cancelled",
                )
            })?),
        };
        Ok(PrivacyRequestFilter { status })
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateRetentionPolicyBody {
    #[schema(example = "Archive inactive contacts")]
    pub(crate) name: String,
    #[schema(example = "archive_inactive")]
    pub(crate) action: String,
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: Option<String>,
    #[schema(example = 1095, minimum = 1, maximum = 36500)]
    pub(crate) threshold_days: u32,
}

impl CreateRetentionPolicyBody {
    pub(crate) fn into_command(self, ip: Option<String>) -> RetentionPolicyCommand {
        RetentionPolicyCommand {
            name: Some(self.name),
            action: Some(self.action),
            record_type: self.record_type,
            threshold_days: Some(self.threshold_days),
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateRetentionPolicyBody {
    pub(crate) name: Option<String>,
    #[schema(minimum = 1, maximum = 36500)]
    pub(crate) threshold_days: Option<u32>,
}

impl UpdateRetentionPolicyBody {
    pub(crate) fn into_command(self, ip: Option<String>) -> RetentionPolicyCommand {
        RetentionPolicyCommand {
            name: self.name,
            action: None,
            record_type: None,
            threshold_days: self.threshold_days,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RetentionPolicyResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    #[schema(example = "archive_inactive")]
    pub(crate) action: String,
    pub(crate) record_type: Option<String>,
    pub(crate) threshold_days: u32,
    pub(crate) is_enabled: bool,
    pub(crate) previewed_count: Option<u64>,
    pub(crate) previewed_at: Option<String>,
    pub(crate) preview_fresh: bool,
    pub(crate) last_run_at: Option<String>,
    pub(crate) created_by: Option<OwnerResponse>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<RetentionPolicy> for RetentionPolicyResponse {
    fn from(policy: RetentionPolicy) -> Self {
        Self {
            preview_fresh: policy.has_fresh_preview(Utc::now()),
            id: policy.id.to_string(),
            name: policy.name,
            action: policy.action.as_str().to_owned(),
            record_type: policy.record_type,
            threshold_days: policy.threshold_days,
            is_enabled: policy.is_enabled,
            previewed_count: policy.previewed_count,
            previewed_at: policy.previewed_at.map(timestamp),
            last_run_at: policy.last_run_at.map(timestamp),
            created_by: policy.created_by.map(Into::into),
            created_at: timestamp(policy.created_at),
            updated_at: timestamp(policy.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RetentionPreviewResponse {
    pub(crate) affected: u64,
    pub(crate) threshold_at: String,
    pub(crate) previewed_at: String,
}

impl From<RetentionPreview> for RetentionPreviewResponse {
    fn from(preview: RetentionPreview) -> Self {
        Self {
            affected: preview.affected,
            threshold_at: timestamp(preview.threshold_at),
            previewed_at: timestamp(preview.previewed_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RetentionRunResponse {
    pub(crate) id: String,
    pub(crate) policy_id: Option<String>,
    pub(crate) policy_name: String,
    pub(crate) action: String,
    pub(crate) record_type: Option<String>,
    pub(crate) threshold_days: u32,
    #[schema(example = "schedule")]
    pub(crate) trigger: String,
    #[schema(example = "completed")]
    pub(crate) status: String,
    pub(crate) affected: u64,
    pub(crate) files_removed: u64,
    pub(crate) errors: u32,
    pub(crate) error_message: Option<String>,
    pub(crate) requested_by: Option<OwnerResponse>,
    pub(crate) created_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) finished_at: Option<String>,
}

impl From<RetentionRun> for RetentionRunResponse {
    fn from(run: RetentionRun) -> Self {
        Self {
            id: run.id.to_string(),
            policy_id: run.policy_id.map(|id| id.to_string()),
            policy_name: run.policy_name,
            action: run.action.as_str().to_owned(),
            record_type: run.record_type,
            threshold_days: run.threshold_days,
            trigger: run.trigger.as_str().to_owned(),
            status: run.status.as_str().to_owned(),
            affected: run.affected,
            files_removed: run.files_removed,
            errors: run.errors,
            error_message: run.error_message,
            requested_by: run.requested_by.map(Into::into),
            created_at: timestamp(run.created_at),
            started_at: run.started_at.map(timestamp),
            finished_at: run.finished_at.map(timestamp),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct RetentionRunListParams {
    pub(crate) policy_id: Option<String>,
}

impl RetentionRunListParams {
    pub(crate) fn into_filter(self) -> Result<RetentionRunFilter, AppError> {
        let policy_id = match self.policy_id.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => Some(parse_uuid(value, "policy_id")?),
        };
        Ok(RetentionRunFilter { policy_id })
    }
}
