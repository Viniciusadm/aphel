use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::{ExportFormat, ExportJob, ExportQuery, RequestExportCommand};
use aphel_kernel::api::dto::query::FilterNodeSchema;
use aphel_kernel::application::query::{ArchivedFilter, Filter, parse_sort};
use aphel_kernel::errors::{AppError, FieldViolation};

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormatDto {
    Csv,
    Xlsx,
}

impl From<ExportFormatDto> for ExportFormat {
    fn from(format: ExportFormatDto) -> Self {
        match format {
            ExportFormatDto::Csv => Self::Csv,
            ExportFormatDto::Xlsx => Self::Xlsx,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExportRequest {
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) format: ExportFormatDto,
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub(crate) filter: Option<Value>,
    #[schema(example = "-created_at,name")]
    pub(crate) sort: Option<String>,
    pub(crate) search: Option<String>,
    #[schema(example = "exclude")]
    pub(crate) archived: Option<String>,
    #[serde(default)]
    #[schema(example = json!(["name", "email", "owner_id"]))]
    pub(crate) columns: Vec<String>,
}

impl ExportRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> Result<RequestExportCommand, AppError> {
        let mut violations = Vec::new();
        let filter = match self.filter {
            None | Some(Value::Null) => None,
            Some(value) => match serde_json::from_value::<Filter>(value) {
                Ok(filter) => Some(filter),
                Err(_) => {
                    violations.push(FieldViolation::new(
                        "filter",
                        "must be a filter group or condition",
                    ));
                    None
                }
            },
        };
        let archived = match self.archived.as_deref().map(str::trim) {
            None | Some("") => ArchivedFilter::Exclude,
            Some(value) => ArchivedFilter::parse(value).unwrap_or_else(|| {
                violations.push(FieldViolation::new(
                    "archived",
                    "must be exclude, include or only",
                ));
                ArchivedFilter::Exclude
            }),
        };
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid export", violations));
        }
        Ok(RequestExportCommand {
            record_type: self.record_type.trim().to_owned(),
            format: self.format.into(),
            query: ExportQuery {
                filter,
                sort: self.sort.as_deref().map(parse_sort).unwrap_or_default(),
                search: self
                    .search
                    .map(|search| search.trim().to_owned())
                    .filter(|search| !search.is_empty()),
                archived,
            },
            columns: self.columns,
            ip,
        })
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportJobResponse {
    pub(crate) id: String,
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    #[schema(example = "xlsx")]
    pub(crate) format: String,
    #[schema(example = "queued")]
    pub(crate) status: String,
    pub(crate) columns: Vec<String>,
    pub(crate) file_id: Option<String>,
    pub(crate) row_count: Option<u64>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) requested_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) finished_at: Option<String>,
}

impl From<ExportJob> for ExportJobResponse {
    fn from(job: ExportJob) -> Self {
        Self {
            id: job.id.to_string(),
            record_type: job.record_type,
            format: job.format.as_str().to_owned(),
            status: job.status.as_str().to_owned(),
            columns: job.columns,
            file_id: job.file_id.map(|id| id.to_string()),
            row_count: job.row_count,
            failure_reason: job.failure_reason,
            requested_by: job.requested_by.into_inner(),
            created_at: timestamp(job.created_at),
            updated_at: timestamp(job.updated_at),
            finished_at: job.finished_at.map(timestamp),
        }
    }
}
