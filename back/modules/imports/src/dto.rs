pub use aphel_sdk::imports::dto::{
    DuplicateKeyDto, DuplicateStrategyDto, ImportJobResponse, ImportOptionsDto, ImportPreviewRow,
    ImportTargetResponse, ImportTotalsResponse,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use aphel_sdk::platform::api::dto::files::FormFile;

use crate::{
    ConfigureImportCommand, DuplicateKey, DuplicateStrategy, ImportDetails, ImportJob,
    ImportOptions, ImportRow, ImportTarget, ImportTotals, RowError, RowReport, ValidationReport,
    delimiter_label,
};

pub(crate) const FILE_FIELD: &str = "file";
pub(crate) const RECORD_TYPE_FIELD: &str = "record_type";
pub(crate) const RETRY_OF_FIELD: &str = "retry_of";

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, ToSchema)]
pub(crate) struct ImportUploadForm {
    #[schema(value_type = String, format = Binary)]
    pub(crate) file: FormFile,
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) retry_of: Option<String>,
}

impl From<DuplicateKey> for DuplicateKeyDto {
    fn from(key: DuplicateKey) -> Self {
        match key {
            DuplicateKey::None => Self::None,
            DuplicateKey::Email => Self::Email,
            DuplicateKey::Phone => Self::Phone,
            DuplicateKey::Domain => Self::Domain,
            DuplicateKey::Name => Self::Name,
        }
    }
}

impl From<DuplicateKeyDto> for DuplicateKey {
    fn from(key: DuplicateKeyDto) -> Self {
        match key {
            DuplicateKeyDto::None => Self::None,
            DuplicateKeyDto::Email => Self::Email,
            DuplicateKeyDto::Phone => Self::Phone,
            DuplicateKeyDto::Domain => Self::Domain,
            DuplicateKeyDto::Name => Self::Name,
        }
    }
}

impl From<DuplicateStrategy> for DuplicateStrategyDto {
    fn from(strategy: DuplicateStrategy) -> Self {
        match strategy {
            DuplicateStrategy::Skip => Self::Skip,
            DuplicateStrategy::Update => Self::Update,
            DuplicateStrategy::Create => Self::Create,
        }
    }
}

impl From<DuplicateStrategyDto> for DuplicateStrategy {
    fn from(strategy: DuplicateStrategyDto) -> Self {
        match strategy {
            DuplicateStrategyDto::Skip => Self::Skip,
            DuplicateStrategyDto::Update => Self::Update,
            DuplicateStrategyDto::Create => Self::Create,
        }
    }
}

impl From<ImportOptions> for ImportOptionsDto {
    fn from(options: ImportOptions) -> Self {
        Self {
            duplicate_key: options.duplicate_key.into(),
            duplicate_strategy: options.duplicate_strategy.into(),
            create_missing_references: options.create_missing_references,
            distribution_rule_id: options.distribution_rule_id,
        }
    }
}

impl From<ImportOptionsDto> for ImportOptions {
    fn from(options: ImportOptionsDto) -> Self {
        Self {
            duplicate_key: options.duplicate_key.into(),
            duplicate_strategy: options.duplicate_strategy.into(),
            create_missing_references: options.create_missing_references,
            distribution_rule_id: options
                .distribution_rule_id
                .map(|id| id.trim().to_owned())
                .filter(|id| !id.is_empty()),
        }
    }
}

impl From<ImportTotals> for ImportTotalsResponse {
    fn from(totals: ImportTotals) -> Self {
        Self {
            total: totals.total,
            pending: totals.pending,
            imported: totals.imported,
            updated: totals.updated,
            skipped: totals.skipped,
            failed: totals.failed,
        }
    }
}

impl From<ImportTarget> for ImportTargetResponse {
    fn from(target: ImportTarget) -> Self {
        Self {
            can_create_reference: target.can_create_reference,
            key: target.key,
            label: target.label,
            field_type: target.field_type.as_str().to_owned(),
            required: target.required,
            is_custom: target.is_custom,
            relation_target: target.relation_target,
            multiple: target.multiple,
            satisfied_by: target.satisfied_by,
        }
    }
}

impl From<ImportJob> for ImportJobResponse {
    fn from(job: ImportJob) -> Self {
        Self {
            id: job.id.to_string(),
            record_type: job.record_type,
            status: job.status.as_str().to_owned(),
            file_id: job.file_id.map(|id| id.to_string()),
            privacy_redacted_at: job.privacy_redacted_at.map(timestamp),
            file_name: job.file_name,
            encoding: job.encoding.as_str().to_owned(),
            delimiter: delimiter_label(job.delimiter),
            headers: job.headers,
            preview: job
                .preview
                .into_iter()
                .map(|row| ImportPreviewRow {
                    row_index: row.row_index,
                    cells: row.cells,
                })
                .collect(),
            row_count: job.row_count,
            mapping: job.mapping,
            options: job.options.into(),
            totals: job.totals.into(),
            error_file_id: job.error_file_id.map(|id| id.to_string()),
            retry_of: job.retry_of.map(|id| id.to_string()),
            failure_reason: job.failure_reason,
            created_by: job.created_by.into_inner(),
            created_at: timestamp(job.created_at),
            updated_at: timestamp(job.updated_at),
            started_at: job.started_at.map(timestamp),
            finished_at: job.finished_at.map(timestamp),
            targets: Vec::new(),
            duplicate_keys: Vec::new(),
        }
    }
}

impl From<ImportDetails> for ImportJobResponse {
    fn from(details: ImportDetails) -> Self {
        let mut response = Self::from(details.job);
        response.targets = details.targets.into_iter().map(Into::into).collect();
        response.duplicate_keys = details.duplicate_keys.into_iter().map(Into::into).collect();
        response
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ConfigureImportRequest {
    pub(crate) mapping: Vec<Option<String>>,
    pub(crate) options: ImportOptionsDto,
}

impl From<ConfigureImportRequest> for ConfigureImportCommand {
    fn from(request: ConfigureImportRequest) -> Self {
        Self {
            mapping: request.mapping,
            options: request.options.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportRowErrorResponse {
    pub(crate) field: Option<String>,
    pub(crate) column: Option<String>,
    pub(crate) message: String,
}

impl From<RowError> for ImportRowErrorResponse {
    fn from(error: RowError) -> Self {
        Self {
            field: error.field,
            column: error.column,
            message: error.message,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportRowReportResponse {
    pub(crate) row_index: u32,
    pub(crate) errors: Vec<ImportRowErrorResponse>,
}

impl From<RowReport> for ImportRowReportResponse {
    fn from(report: RowReport) -> Self {
        Self {
            row_index: report.row_index,
            errors: report.errors.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportValidationResponse {
    pub(crate) total_rows: u64,
    pub(crate) valid_rows: u64,
    pub(crate) invalid_rows: u64,
    pub(crate) rows: Vec<ImportRowReportResponse>,
}

impl From<ValidationReport> for ImportValidationResponse {
    fn from(report: ValidationReport) -> Self {
        Self {
            total_rows: report.total_rows,
            valid_rows: report.valid_rows,
            invalid_rows: report.invalid_rows,
            rows: report.rows.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportRowResponse {
    pub(crate) row_index: u32,
    #[schema(example = "failed")]
    pub(crate) status: String,
    #[schema(example = "created")]
    pub(crate) action: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) errors: Vec<ImportRowErrorResponse>,
    pub(crate) cells: Vec<String>,
}

impl From<ImportRow> for ImportRowResponse {
    fn from(row: ImportRow) -> Self {
        Self {
            row_index: row.row_index,
            status: row.status.as_str().to_owned(),
            action: row.action.map(|action| action.as_str().to_owned()),
            record_id: row.record_id.map(|id| id.to_string()),
            errors: row.errors.into_iter().map(Into::into).collect(),
            cells: row.cells,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ImportListParams {
    #[param(example = "people.person")]
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ImportRowsParams {
    #[param(example = "failed")]
    pub(crate) status: Option<String>,
}
