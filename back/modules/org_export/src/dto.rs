use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ExportSummary, OrgExport};

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OrgExportRequest {
    #[schema(example = true)]
    pub(crate) include_files: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OrgExportSummaryResponse {
    #[schema(example = json!({ "contact": 60, "company": 20, "deal": 40 }))]
    pub(crate) records: BTreeMap<String, u64>,
    #[schema(example = json!({ "notes": 25, "attachments": 3 }))]
    pub(crate) datasets: BTreeMap<String, u64>,
    pub(crate) members: u64,
    pub(crate) files: u64,
    pub(crate) file_bytes: u64,
    pub(crate) missing_files: u64,
    pub(crate) archive_bytes: u64,
}

impl From<ExportSummary> for OrgExportSummaryResponse {
    fn from(summary: ExportSummary) -> Self {
        Self {
            records: summary.records,
            datasets: summary.datasets,
            members: summary.members,
            files: summary.files,
            file_bytes: summary.file_bytes,
            missing_files: summary.missing_files,
            archive_bytes: summary.archive_bytes,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OrgExportRequesterResponse {
    pub(crate) id: String,
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OrgExportResponse {
    pub(crate) id: String,
    #[schema(example = "completed")]
    pub(crate) status: String,
    pub(crate) include_files: bool,
    pub(crate) summary: Option<OrgExportSummaryResponse>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) requested_by: OrgExportRequesterResponse,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) finished_at: Option<String>,
}

impl From<OrgExport> for OrgExportResponse {
    fn from(export: OrgExport) -> Self {
        Self {
            id: export.id.to_string(),
            status: export.status.as_str().to_owned(),
            include_files: export.include_files,
            summary: export.summary.map(Into::into),
            failure_reason: export.failure_reason,
            requested_by: OrgExportRequesterResponse {
                id: export.requested_by.as_str().to_owned(),
                display_name: export.requested_by_name,
            },
            created_at: timestamp(export.created_at),
            updated_at: timestamp(export.updated_at),
            started_at: export.started_at.map(timestamp),
            finished_at: export.finished_at.map(timestamp),
        }
    }
}
