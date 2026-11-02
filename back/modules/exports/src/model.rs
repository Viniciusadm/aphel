use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::application::query::{ArchivedFilter, Filter, SortSpec};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::domain::custom_objects::{is_custom_record_type, object_permission};
use aphel_kernel::errors::AppError;

pub(crate) const EXPORT_RECORD_TYPE: RecordType = RecordType::from_static("export");
pub(crate) const EXPORT_RUN_JOB: &str = "export.run";
pub(crate) const EXPORT_PAGE_SIZE: u64 = 500;
pub(crate) const MAX_EXPORT_ROWS: u64 = 100_000;
pub(crate) const ID_COLUMN: &str = "id";

pub(crate) const EXPORT_REQUESTED: &str = "export.requested";
pub(crate) const EXPORT_COMPLETED: &str = "export.completed";
pub(crate) const EXPORT_FAILED: &str = "export.failed";

pub(crate) fn export_permission(record_type: &str) -> Option<Permission> {
    if is_custom_record_type(record_type) {
        return object_permission(record_type, "export");
    }
    aphel_kernel::modules::app()
        .record_types()
        .require(record_type)
        .ok()
        .filter(|definition| definition.capabilities.has_trait("exports.exportable"))?
        .action_permission("export")
}

pub(crate) fn is_exportable(record_type: &str) -> bool {
    is_custom_record_type(record_type)
        || aphel_kernel::modules::app()
            .record_types()
            .capabilities(record_type)
            .is_some_and(|capabilities| capabilities.has_trait("exports.exportable"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Xlsx,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "csv" => Some(Self::Csv),
            "xlsx" => Some(Self::Xlsx),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl ExportStatus {
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    pub sort: Vec<SortSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    pub archived: ArchivedFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportJob {
    pub(crate) id: Uuid,
    pub(crate) record_type: String,
    pub(crate) format: ExportFormat,
    pub(crate) status: ExportStatus,
    pub(crate) query: ExportQuery,
    pub(crate) columns: Vec<String>,
    pub(crate) file_id: Option<Uuid>,
    pub(crate) row_count: Option<u64>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) requested_by: UserId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestExportCommand {
    pub record_type: String,
    pub format: ExportFormat,
    pub query: ExportQuery,
    pub columns: Vec<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunExportRequest {
    pub(crate) export_id: Uuid,
    pub(crate) requested_by: UserId,
}

impl RunExportRequest {
    pub(crate) fn payload(&self) -> Value {
        json!({
            "export_id": self.export_id.to_string(),
            "requested_by": self.requested_by.as_str(),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::Internal(format!("export job payload misses {key}")))
        };
        Ok(Self {
            export_id: Uuid::parse_str(text("export_id")?)
                .map_err(|_| AppError::Internal("invalid export id in job payload".to_owned()))?,
            requested_by: UserId::new(text("requested_by")?)?,
        })
    }
}
