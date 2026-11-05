use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordType;

pub(crate) const ORG_EXPORT_FORMAT: &str = "aphel.organization_export";
pub(crate) const ORG_EXPORT_FORMAT_VERSION: u32 = 1;
pub(crate) const ORG_EXPORT_RECORD_TYPE: RecordType =
    RecordType::from_static("org_export.organization_export");
pub(crate) const ORG_EXPORT_RUN_JOB: &str = "organization_export.run";
pub(crate) const ORG_EXPORT_REQUESTED: &str = "organization_export.requested";
pub(crate) const ORG_EXPORT_COMPLETED: &str = "organization_export.completed";
pub(crate) const ORG_EXPORT_FAILED: &str = "organization_export.failed";
pub(crate) const ORGANIZATION_RESTORED: &str = "organization.restored";
pub(crate) const ZIP_MIME_TYPE: &str = "application/zip";

pub(crate) const ORG_EXPORT_NOT_FOUND: &str = "organization_export_not_found";
pub(crate) const ORG_EXPORT_IN_PROGRESS: &str = "organization_export_in_progress";
pub(crate) const ORG_EXPORT_NOT_READY: &str = "organization_export_not_ready";
pub(crate) const INVALID_ORG_EXPORT: &str = "invalid_organization_export";
pub(crate) const ORGANIZATION_NOT_EMPTY: &str = "organization_not_empty";

pub(crate) const GENERIC_FAILURE: &str = "the organization export could not be completed";

pub(crate) const MANIFEST_FILE: &str = "manifest.json";
pub(crate) const CONFIG_PACKAGE_FILE: &str = "config/package.json";
pub(crate) const CONFIG_KEYS_FILE: &str = "config/keys.json";
pub(crate) const CONFIG_ITEMS_DIR: &str = "config/items";
pub(crate) const MEMBERS_FILE: &str = "members.jsonl";
pub(crate) const RECORDS_DIR: &str = "records";
pub(crate) const FILES_FILE: &str = "files/files.jsonl";
pub(crate) const FILE_BLOBS_DIR: &str = "files/blobs";

pub(crate) fn org_export_not_found() -> AppError {
    AppError::not_found_code(ORG_EXPORT_NOT_FOUND, "organization export was not found")
}

pub(crate) fn invalid_archive(message: impl Into<String>) -> AppError {
    AppError::validation_code(INVALID_ORG_EXPORT, message)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrgExportStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl OrgExportStatus {
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ExportSummary {
    pub(crate) records: BTreeMap<String, u64>,
    pub(crate) datasets: BTreeMap<String, u64>,
    pub(crate) members: u64,
    pub(crate) files: u64,
    pub(crate) file_bytes: u64,
    pub(crate) missing_files: u64,
    pub(crate) archive_bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OrgExport {
    pub(crate) id: Uuid,
    pub(crate) status: OrgExportStatus,
    pub(crate) include_files: bool,
    pub(crate) file_id: Option<Uuid>,
    pub(crate) summary: Option<ExportSummary>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) requested_by: UserId,
    pub(crate) requested_by_name: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManifestOrganization {
    pub(crate) name: String,
    pub(crate) locale: String,
    pub(crate) timezone: String,
    pub(crate) currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManifestEntry {
    pub(crate) path: String,
    pub(crate) kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    pub(crate) count: u64,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) format: String,
    pub(crate) format_version: u32,
    pub(crate) aphel_version: String,
    pub(crate) exported_at: String,
    pub(crate) export_id: String,
    pub(crate) organization: ManifestOrganization,
    pub(crate) include_files: bool,
    pub(crate) counts: ExportSummary,
    pub(crate) entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrganizationProfile {
    pub(crate) name: String,
    pub(crate) locale: String,
    pub(crate) timezone: String,
    pub(crate) currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MemberRow {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) password_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnInfo {
    pub(crate) name: String,
    pub(crate) nullable: bool,
    pub(crate) generated: bool,
    pub(crate) kind: ColumnKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnKind {
    Text,
    Number,
    Decimal,
    Date,
    DateTime,
    Json,
}
