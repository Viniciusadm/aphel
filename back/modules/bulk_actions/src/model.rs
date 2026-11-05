use aphel_contract_crm::CrmPermissions;
use aphel_contract_crm::traits;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::exports::ExportFormat;
use aphel_sdk::platform::application::fields::{EntityTraits, RecordTypeRegistry};
use aphel_sdk::platform::application::query::{ArchivedFilter, Filter};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::Permission;

pub(crate) const BULK_RUN_JOB: &str = "bulk.run";
pub(crate) const BULK_INLINE_LIMIT: usize = 200;
pub(crate) const MAX_BULK_RECORDS: u64 = 10_000;
pub(crate) const MAX_SELECTED_IDS: usize = 10_000;
pub(crate) const BULK_PAGE_SIZE: u64 = 500;

pub(crate) const BULK_OPERATION_RECORD_TYPE: RecordType =
    RecordType::from_static("bulk_actions.bulk_operation");
pub(crate) const BULK_OPERATION_REQUESTED: &str = "bulk_operation.requested";
pub(crate) const BULK_OPERATION_COMPLETED: &str = "bulk_operation.completed";
pub(crate) const BULK_OPERATION_FAILED: &str = "bulk_operation.failed";

pub(crate) fn bulk_capabilities(
    registry: &RecordTypeRegistry,
    record_type: &str,
) -> Option<EntityTraits> {
    registry
        .capabilities(record_type)
        .filter(|capabilities| capabilities.allows_bulk())
        .cloned()
}

pub(crate) fn is_bulk_field(registry: &RecordTypeRegistry, record_type: &str, field: &str) -> bool {
    bulk_capabilities(registry, record_type)
        .is_some_and(|capabilities| capabilities.bulk_fields.iter().any(|key| key == field))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum BulkAction {
    AssignOwner {
        owner_id: String,
    },
    SetField {
        field: String,
        #[serde(default)]
        value: Value,
        #[serde(default)]
        custom: bool,
    },
    AddTag {
        tag_id: String,
    },
    RemoveTag {
        tag_id: String,
    },
    MoveStage {
        stage_id: String,
    },
    EnrollInSequence {
        sequence_id: String,
    },
    Archive,
    Unarchive,
    Delete,
}

impl BulkAction {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::AssignOwner { .. } => "assign_owner",
            Self::SetField { .. } => "set_field",
            Self::AddTag { .. } => "add_tag",
            Self::RemoveTag { .. } => "remove_tag",
            Self::MoveStage { .. } => "move_stage",
            Self::EnrollInSequence { .. } => "enroll_in_sequence",
            Self::Archive => "archive",
            Self::Unarchive => "unarchive",
            Self::Delete => "delete",
        }
    }

    pub(crate) fn required_permission(
        &self,
        registry: &RecordTypeRegistry,
        record_type: &str,
    ) -> Result<Permission, AppError> {
        let unsupported = |operation: &str| {
            AppError::validation_code(
                BULK_NOT_SUPPORTED,
                format!("{record_type} records do not support bulk {operation}"),
            )
        };
        let capabilities =
            bulk_capabilities(registry, record_type).ok_or_else(|| unsupported("actions"))?;
        if let Self::EnrollInSequence { .. } = self {
            return capabilities
                .has_trait(traits::SEQUENCEABLE)
                .then_some(Permission::SEQUENCES_ENROLL)
                .ok_or_else(|| unsupported("sequence enrollment"));
        }
        let definition = registry.require(record_type)?;
        match self {
            Self::Archive | Self::Unarchive => definition
                .archived_column
                .as_ref()
                .and_then(|_| definition.action_permission("archive"))
                .ok_or_else(|| unsupported("archiving")),
            Self::Delete => definition
                .action_permission("delete")
                .ok_or_else(|| unsupported("deletion")),
            _ => definition
                .update_permission
                .clone()
                .or_else(|| definition.action_permission("update"))
                .ok_or_else(|| unsupported("updates")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct BulkQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) search: Option<String>,
    pub(crate) archived: ArchivedFilter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum BulkSelection {
    Ids {
        #[serde(with = "uuid_list")]
        ids: Vec<Uuid>,
    },
    Query {
        query: BulkQuery,
    },
}

mod uuid_list {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    pub(crate) fn serialize<S: Serializer>(ids: &[Uuid], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(ids.iter().map(Uuid::to_string))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Uuid>, D::Error> {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|value| Uuid::parse_str(value).map_err(serde::de::Error::custom))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BulkStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl BulkStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    pub(crate) fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemOutcome {
    Succeeded,
    Skipped,
    Failed,
}

impl ItemOutcome {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "skipped" => Some(Self::Skipped),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BulkItem {
    pub(crate) record_id: Uuid,
    pub(crate) outcome: ItemOutcome,
    pub(crate) code: Option<String>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BulkOperation {
    pub(crate) id: Uuid,
    pub(crate) record_type: String,
    pub(crate) action: BulkAction,
    pub(crate) selection: BulkSelection,
    pub(crate) status: BulkStatus,
    pub(crate) total: u64,
    pub(crate) processed: u64,
    pub(crate) succeeded: u64,
    pub(crate) skipped: u64,
    pub(crate) failed: u64,
    pub(crate) failure_reason: Option<String>,
    pub(crate) requested_by: UserId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

pub(crate) const BULK_OPERATION_NOT_FOUND: &str = "bulk_operation_not_found";
pub(crate) const BULK_NOT_SUPPORTED: &str = "bulk_not_supported";
pub(crate) const BULK_TOO_LARGE: &str = "bulk_too_large";
pub(crate) const BULK_EMPTY_SELECTION: &str = "bulk_empty_selection";
pub(crate) const GENERIC_FAILURE: &str = "the bulk operation could not be completed";

pub(crate) fn bulk_operation_not_found() -> AppError {
    AppError::not_found_code(BULK_OPERATION_NOT_FOUND, "bulk operation was not found")
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StartBulkCommand {
    pub(crate) record_type: String,
    pub(crate) action: BulkAction,
    pub(crate) selection: BulkSelection,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BulkExportCommand {
    pub(crate) record_type: String,
    pub(crate) selection: BulkSelection,
    pub(crate) format: ExportFormat,
    pub(crate) columns: Vec<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunBulkRequest {
    pub(crate) operation_id: Uuid,
    pub(crate) requested_by: UserId,
    pub(crate) api_token_id: Option<Uuid>,
}

impl RunBulkRequest {
    pub(crate) fn payload(&self) -> Value {
        json!({
            "operation_id": self.operation_id.to_string(),
            "requested_by": self.requested_by.as_str(),
            "api_token_id": self.api_token_id.map(|id| id.to_string()),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::Internal(format!("bulk job payload misses {key}")))
        };
        Ok(Self {
            operation_id: Uuid::parse_str(text("operation_id")?).map_err(|_| {
                AppError::Internal("invalid bulk operation id in job payload".to_owned())
            })?,
            requested_by: UserId::new(text("requested_by")?)?,
            api_token_id: match payload.get("api_token_id").and_then(Value::as_str) {
                Some(id) => Some(Uuid::parse_str(id).map_err(|_| {
                    AppError::Internal("invalid API token id in job payload".to_owned())
                })?),
                None => None,
            },
        })
    }
}

pub(crate) fn classify_error(error: &AppError) -> (ItemOutcome, String, String) {
    let outcome = if matches!(
        error,
        AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Validation(_)
            | AppError::Coded { .. }
            | AppError::Forbidden(_)
            | AppError::NotFound(_)
            | AppError::Conflict(_)
    ) {
        ItemOutcome::Skipped
    } else {
        ItemOutcome::Failed
    };
    let code = match error {
        AppError::Coded { code, .. } | AppError::CodedFields { code, .. } => (*code).to_owned(),
        AppError::InvalidFields { .. } => "invalid_fields".to_owned(),
        AppError::Validation(_) => "validation_failed".to_owned(),
        AppError::Forbidden(_) => "permission_denied".to_owned(),
        AppError::NotFound(_) => "not_found".to_owned(),
        AppError::Conflict(_) => "conflict".to_owned(),
        _ => "internal_error".to_owned(),
    };
    let message = match error {
        AppError::InvalidFields { fields, .. } | AppError::CodedFields { fields, .. }
            if !fields.is_empty() =>
        {
            fields
                .iter()
                .map(|violation| format!("{}: {}", violation.field, violation.message))
                .collect::<Vec<_>>()
                .join("; ")
        }
        AppError::InvalidFields { message, .. } => message.clone(),
        AppError::Internal(_) | AppError::Persistence(_) | AppError::Configuration(_) => {
            GENERIC_FAILURE.to_owned()
        }
        other => other.client_message().to_owned(),
    };
    (outcome, code, message.chars().take(500).collect())
}
