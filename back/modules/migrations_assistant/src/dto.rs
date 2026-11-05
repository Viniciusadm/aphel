use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    DateOrder, DecimalStyle, HistoryReport, MigrationHistory, MigrationImport, MigrationPreview,
    SourceTool, ValueMaps,
};
use aphel_sdk::imports::dto::ImportJobResponse;
use aphel_sdk::platform::api::dto::files::FormFile;

pub(crate) const SOURCE_FIELD: &str = "source";
pub(crate) const STEP_FIELD: &str = "step";
pub(crate) const MAPS_FIELD: &str = "maps";
pub(crate) const FILE_FIELD: &str = "file";
const PREVIEW_SAMPLE_ROWS: usize = 5;

#[derive(Debug, ToSchema)]
pub(crate) struct MigrationUploadForm {
    #[schema(value_type = String, format = Binary)]
    pub(crate) file: FormFile,
    #[schema(example = "pipedrive")]
    pub(crate) source: String,
    #[schema(example = "deals")]
    pub(crate) step: String,
    #[schema(
        example = "{\"stage_id\":{\"Qualified\":\"Qualification\"},\"owner_id\":{\"Ana Souza\":\"ana@company.com\"}}"
    )]
    pub(crate) maps: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ValueMapsInput {
    #[serde(default)]
    pub(crate) stage_id: HashMap<String, String>,
    #[serde(default)]
    pub(crate) pipeline_id: HashMap<String, String>,
    #[serde(default)]
    pub(crate) owner_id: HashMap<String, String>,
}

impl From<ValueMapsInput> for ValueMaps {
    fn from(input: ValueMapsInput) -> Self {
        Self {
            stages: input.stage_id,
            workflow: input.pipeline_id,
            owners: input.owner_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MigrationColumnResponse {
    #[schema(example = json!(["Deal - Value", "Negócio - Valor"]))]
    pub(crate) headers: Vec<String>,
    #[schema(example = "amount")]
    pub(crate) target: String,
    #[schema(example = "money")]
    pub(crate) transform: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MigrationStepResponse {
    #[schema(example = "deals")]
    pub(crate) key: String,
    #[schema(example = "deals.deal")]
    pub(crate) record_type: String,
    #[schema(example = "import")]
    pub(crate) kind: String,
    #[schema(example = "deals.csv")]
    pub(crate) file_hint: String,
    pub(crate) links: Vec<String>,
    pub(crate) columns: Vec<MigrationColumnResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MigrationSourceResponse {
    #[schema(example = "pipedrive")]
    pub(crate) key: String,
    #[schema(example = "Pipedrive")]
    pub(crate) name: String,
    #[schema(example = "month_first")]
    pub(crate) date_order: String,
    #[schema(example = "dot")]
    pub(crate) decimal: String,
    pub(crate) steps: Vec<MigrationStepResponse>,
}

impl From<&SourceTool> for MigrationSourceResponse {
    fn from(tool: &SourceTool) -> Self {
        Self {
            key: tool.key.to_owned(),
            name: tool.name.to_owned(),
            date_order: match tool.date_order {
                DateOrder::DayFirst => "day_first",
                DateOrder::MonthFirst => "month_first",
            }
            .to_owned(),
            decimal: match tool.decimal {
                DecimalStyle::Comma => "comma",
                DecimalStyle::Dot => "dot",
            }
            .to_owned(),
            steps: tool
                .steps
                .iter()
                .map(|step| MigrationStepResponse {
                    key: step.key.to_owned(),
                    record_type: step.record_type.to_owned(),
                    kind: step.kind.as_str().to_owned(),
                    file_hint: step.file_hint.to_owned(),
                    links: step.links.iter().map(|link| (*link).to_owned()).collect(),
                    columns: step
                        .columns
                        .iter()
                        .map(|column| MigrationColumnResponse {
                            headers: column.headers.iter().map(|h| (*h).to_owned()).collect(),
                            target: column.target.to_owned(),
                            transform: column.transform.as_str().to_owned(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecognizedColumnResponse {
    pub(crate) index: usize,
    #[schema(example = "Deal - Value")]
    pub(crate) header: String,
    #[schema(example = "amount")]
    pub(crate) target: String,
    #[schema(example = "money")]
    pub(crate) transform: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MigrationPreviewResponse {
    pub(crate) source: String,
    pub(crate) step: String,
    pub(crate) record_type: String,
    pub(crate) row_count: u64,
    pub(crate) headers: Vec<String>,
    pub(crate) recognized: Vec<RecognizedColumnResponse>,
    pub(crate) unrecognized: Vec<String>,
    pub(crate) changed_cells: u64,
    #[schema(example = json!({ "owner_id": 2 }))]
    pub(crate) issues: BTreeMap<String, u64>,
    #[schema(example = json!({ "stage_id": ["Qualified", "Proposal Made"] }))]
    pub(crate) distinct: BTreeMap<String, Vec<String>>,
    pub(crate) sample: Vec<Vec<String>>,
}

impl From<MigrationPreview> for MigrationPreviewResponse {
    fn from(preview: MigrationPreview) -> Self {
        Self {
            source: preview.tool.key.to_owned(),
            step: preview.step.key.to_owned(),
            record_type: preview.step.record_type.to_owned(),
            row_count: preview.row_count,
            headers: preview.file.headers,
            recognized: preview
                .file
                .recognized
                .into_iter()
                .map(|column| RecognizedColumnResponse {
                    index: column.index,
                    header: column.header,
                    target: column.target,
                    transform: column.transform.as_str().to_owned(),
                })
                .collect(),
            unrecognized: preview.file.unrecognized,
            changed_cells: preview.file.changed_cells,
            issues: preview.file.issues,
            distinct: preview.file.distinct,
            sample: preview
                .file
                .rows
                .into_iter()
                .take(PREVIEW_SAMPLE_ROWS)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MigrationImportResponse {
    pub(crate) import: ImportJobResponse,
    pub(crate) preview: MigrationPreviewResponse,
}

impl From<MigrationImport> for MigrationImportResponse {
    fn from(result: MigrationImport) -> Self {
        Self {
            import: result.import,
            preview: result.preview.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct HistoryRowErrorResponse {
    #[schema(example = 12)]
    pub(crate) row: usize,
    pub(crate) message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct HistoryReportResponse {
    pub(crate) total: u64,
    pub(crate) created: u64,
    pub(crate) skipped: u64,
    pub(crate) failed: u64,
    pub(crate) errors: Vec<HistoryRowErrorResponse>,
}

impl From<HistoryReport> for HistoryReportResponse {
    fn from(report: HistoryReport) -> Self {
        Self {
            total: report.total,
            created: report.created,
            skipped: report.skipped,
            failed: report.failed,
            errors: report
                .errors
                .into_iter()
                .map(|error| HistoryRowErrorResponse {
                    row: error.row,
                    message: error.message,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MigrationHistoryResponse {
    pub(crate) report: HistoryReportResponse,
    pub(crate) preview: MigrationPreviewResponse,
}

impl From<MigrationHistory> for MigrationHistoryResponse {
    fn from(result: MigrationHistory) -> Self {
        Self {
            report: result.report.into(),
            preview: result.preview.into(),
        }
    }
}
