use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use aphel_kernel::api::dto::files::FormFile;

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
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) retry_of: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DuplicateKeyDto {
    None,
    Email,
    Phone,
    Domain,
    Name,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DuplicateStrategyDto {
    Skip,
    Update,
    Create,
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct ImportOptionsDto {
    pub(crate) duplicate_key: DuplicateKeyDto,
    pub(crate) duplicate_strategy: DuplicateStrategyDto,
    pub(crate) create_missing_companies: bool,
    #[serde(default)]
    pub(crate) distribution_rule_id: Option<String>,
}

impl From<ImportOptions> for ImportOptionsDto {
    fn from(options: ImportOptions) -> Self {
        Self {
            duplicate_key: options.duplicate_key.into(),
            duplicate_strategy: options.duplicate_strategy.into(),
            create_missing_companies: options.create_missing_companies,
            distribution_rule_id: options.distribution_rule_id,
        }
    }
}

impl From<ImportOptionsDto> for ImportOptions {
    fn from(options: ImportOptionsDto) -> Self {
        Self {
            duplicate_key: options.duplicate_key.into(),
            duplicate_strategy: options.duplicate_strategy.into(),
            create_missing_companies: options.create_missing_companies,
            distribution_rule_id: options
                .distribution_rule_id
                .map(|id| id.trim().to_owned())
                .filter(|id| !id.is_empty()),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportTotalsResponse {
    pub(crate) total: u64,
    pub(crate) pending: u64,
    pub(crate) imported: u64,
    pub(crate) updated: u64,
    pub(crate) skipped: u64,
    pub(crate) failed: u64,
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

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportTargetResponse {
    #[schema(example = "email")]
    pub(crate) key: String,
    #[schema(example = "Email")]
    pub(crate) label: String,
    #[schema(example = "email")]
    pub(crate) field_type: String,
    pub(crate) required: bool,
    pub(crate) is_custom: bool,
    #[schema(example = "companies.company")]
    pub(crate) relation_target: Option<String>,
    pub(crate) multiple: bool,
    #[schema(example = "name")]
    pub(crate) satisfied_by: Option<String>,
}

impl From<ImportTarget> for ImportTargetResponse {
    fn from(target: ImportTarget) -> Self {
        Self {
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

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ImportPreviewRow {
    pub(crate) row_index: u32,
    pub(crate) cells: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportJobResponse {
    pub(crate) id: String,
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    #[schema(example = "draft")]
    pub(crate) status: String,
    pub(crate) file_id: String,
    pub(crate) file_name: String,
    #[schema(example = "utf-8")]
    pub(crate) encoding: String,
    #[schema(example = ";")]
    pub(crate) delimiter: String,
    pub(crate) headers: Vec<String>,
    pub(crate) preview: Vec<ImportPreviewRow>,
    pub(crate) row_count: u32,
    pub(crate) mapping: Vec<Option<String>>,
    pub(crate) options: ImportOptionsDto,
    pub(crate) totals: ImportTotalsResponse,
    pub(crate) error_file_id: Option<String>,
    pub(crate) retry_of: Option<String>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) finished_at: Option<String>,
    pub(crate) targets: Vec<ImportTargetResponse>,
    pub(crate) duplicate_keys: Vec<DuplicateKeyDto>,
}

impl From<ImportJob> for ImportJobResponse {
    fn from(job: ImportJob) -> Self {
        Self {
            id: job.id.to_string(),
            record_type: job.record_type,
            status: job.status.as_str().to_owned(),
            file_id: job.file_id.to_string(),
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
    #[param(example = "contacts.contact")]
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ImportRowsParams {
    #[param(example = "failed")]
    pub(crate) status: Option<String>,
}
