use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{SourceRow, TextEncoding};
use aphel_kernel::application::fields::{EntityDefinition, LabelNormalization, SystemField};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::fields::FieldType;

pub(crate) const IMPORT_RECORD_TYPE: RecordType = RecordType::from_static("imports.import");
pub(crate) const IMPORT_RUN_JOB: &str = "import.run";
pub(crate) const IMPORT_BATCH_SIZE: usize = 200;

pub(crate) const IMPORT_STARTED: &str = "import.started";
pub(crate) const IMPORT_COMPLETED: &str = "import.completed";
pub(crate) const IMPORT_FAILED: &str = "import.failed";
pub(crate) const IMPORT_RETRIED: &str = "import.retried";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportStatus {
    Draft,
    Queued,
    Running,
    Completed,
    Failed,
}

impl ImportStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
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
pub(crate) enum RowStatus {
    Pending,
    Imported,
    Skipped,
    Failed,
}

impl RowStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Imported => "imported",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "imported" => Some(Self::Imported),
            "skipped" => Some(Self::Skipped),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowAction {
    Created,
    Updated,
    Skipped,
}

impl RowAction {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Skipped => "skipped",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(Self::Created),
            "updated" => Some(Self::Updated),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKey {
    #[default]
    None,
    Email,
    Phone,
    Domain,
    Name,
}

pub(crate) const DUPLICATE_KEYS: [DuplicateKey; 4] = [
    DuplicateKey::Domain,
    DuplicateKey::Email,
    DuplicateKey::Phone,
    DuplicateKey::Name,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DuplicateColumn {
    pub(crate) field: String,
    pub(crate) column: String,
}

impl DuplicateKey {
    pub(crate) fn columns(&self, definition: &EntityDefinition) -> Vec<DuplicateColumn> {
        let stored =
            |field: &&SystemField| field.computed.is_none() && field.contributed_by.is_none();
        let of_type = |field_type: FieldType| {
            definition
                .system_fields
                .iter()
                .filter(stored)
                .filter(|field| field.field_type == field_type)
                .map(|field| DuplicateColumn {
                    field: field.key.as_str().to_owned(),
                    column: field.column.clone(),
                })
                .collect::<Vec<_>>()
        };
        match self {
            Self::None => Vec::new(),
            Self::Email => of_type(FieldType::Email),
            Self::Phone => of_type(FieldType::Phone),
            Self::Domain => definition
                .capabilities
                .label_aliases
                .iter()
                .filter(|alias| alias.normalization == LabelNormalization::Domain)
                .filter_map(|alias| {
                    definition
                        .system_fields
                        .iter()
                        .filter(stored)
                        .find(|field| field.column == alias.column)
                        .map(|field| DuplicateColumn {
                            field: field.key.as_str().to_owned(),
                            column: field.column.clone(),
                        })
                })
                .collect(),
            Self::Name => definition
                .title_field
                .as_deref()
                .and_then(|key| definition.system_field_by_key(key))
                .filter(stored)
                .map(|field| DuplicateColumn {
                    field: field.key.as_str().to_owned(),
                    column: field.column.clone(),
                })
                .into_iter()
                .collect(),
        }
    }

    pub(crate) fn fallback_fields(&self, definition: &EntityDefinition) -> Vec<String> {
        match self {
            Self::Domain => definition
                .system_fields
                .iter()
                .filter(|field| {
                    field.field_type.base_type() == FieldType::Url && field.computed.is_none()
                })
                .map(|field| field.key.as_str().to_owned())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn supported_by(&self, definition: &EntityDefinition) -> bool {
        *self == Self::None || !self.columns(definition).is_empty()
    }

    pub(crate) fn supported(definition: Option<&EntityDefinition>) -> Vec<Self> {
        let mut keys: Vec<Self> = definition
            .map(|definition| {
                DUPLICATE_KEYS
                    .into_iter()
                    .filter(|key| key.supported_by(definition))
                    .collect()
            })
            .unwrap_or_default();
        keys.push(Self::None);
        keys
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Domain => "domain",
            Self::Name => "name",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateStrategy {
    #[default]
    Skip,
    Update,
    Create,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportOptions {
    pub duplicate_key: DuplicateKey,
    pub duplicate_strategy: DuplicateStrategy,
    pub create_missing_companies: bool,
    pub distribution_rule_id: Option<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            duplicate_key: DuplicateKey::None,
            duplicate_strategy: DuplicateStrategy::Skip,
            create_missing_companies: true,
            distribution_rule_id: None,
        }
    }
}

impl ImportOptions {
    pub fn default_for(record_type: &str) -> Self {
        Self::default_for_definition(
            aphel_kernel::modules::app()
                .record_types()
                .require(record_type)
                .ok(),
        )
    }

    pub(crate) fn default_for_definition(definition: Option<&EntityDefinition>) -> Self {
        let duplicate_key = definition
            .and_then(|definition| {
                [DuplicateKey::Domain, DuplicateKey::Email]
                    .into_iter()
                    .find(|key| key.supported_by(definition))
            })
            .unwrap_or(DuplicateKey::None);
        Self {
            duplicate_key,
            ..Self::default()
        }
    }

    pub(crate) fn checks_duplicates(&self) -> bool {
        self.duplicate_key != DuplicateKey::None
            && self.duplicate_strategy != DuplicateStrategy::Create
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ImportTotals {
    pub(crate) total: u64,
    pub(crate) pending: u64,
    pub(crate) imported: u64,
    pub(crate) updated: u64,
    pub(crate) skipped: u64,
    pub(crate) failed: u64,
}

impl ImportTotals {
    pub(crate) fn processed(&self) -> u64 {
        self.imported + self.updated + self.skipped + self.failed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RowError {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) column: Option<String>,
    pub(crate) message: String,
}

impl RowError {
    pub(crate) fn general(message: impl Into<String>) -> Self {
        Self {
            field: None,
            column: None,
            message: message.into(),
        }
    }

    pub(crate) fn describe(&self) -> String {
        match (&self.column, &self.field) {
            (Some(column), _) => format!("{column}: {}", self.message),
            (None, Some(field)) => format!("{field}: {}", self.message),
            (None, None) => self.message.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportTarget {
    pub key: String,
    pub(crate) label: String,
    pub(crate) field_type: FieldType,
    pub(crate) required: bool,
    pub(crate) is_custom: bool,
    pub(crate) relation_target: Option<String>,
    pub(crate) multiple: bool,
    pub(crate) satisfied_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportJob {
    pub id: Uuid,
    pub(crate) record_type: String,
    pub(crate) file_id: Uuid,
    pub(crate) file_name: String,
    pub(crate) status: ImportStatus,
    pub(crate) encoding: TextEncoding,
    pub(crate) delimiter: u8,
    pub headers: Vec<String>,
    pub(crate) preview: Vec<SourceRow>,
    pub(crate) row_count: u32,
    pub mapping: Vec<Option<String>>,
    pub(crate) options: ImportOptions,
    pub(crate) totals: ImportTotals,
    pub(crate) error_file_id: Option<Uuid>,
    pub(crate) retry_of: Option<Uuid>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_by: UserId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportRow {
    pub(crate) row_index: u32,
    pub(crate) status: RowStatus,
    pub(crate) action: Option<RowAction>,
    pub(crate) record_id: Option<Uuid>,
    pub(crate) errors: Vec<RowError>,
    pub(crate) cells: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowOutcome {
    pub(crate) row_index: u32,
    pub(crate) status: RowStatus,
    pub(crate) action: Option<RowAction>,
    pub(crate) record_id: Option<Uuid>,
    pub(crate) errors: Vec<RowError>,
}

impl RowOutcome {
    pub(crate) fn failed(row_index: u32, errors: Vec<RowError>) -> Self {
        Self {
            row_index,
            status: RowStatus::Failed,
            action: None,
            record_id: None,
            errors,
        }
    }

    pub(crate) fn done(row_index: u32, action: RowAction, record_id: Uuid) -> Self {
        Self {
            row_index,
            status: match action {
                RowAction::Skipped => RowStatus::Skipped,
                RowAction::Created | RowAction::Updated => RowStatus::Imported,
            },
            action: Some(action),
            record_id: Some(record_id),
            errors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ImportJobFilter {
    pub(crate) created_by: Option<UserId>,
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowReport {
    pub(crate) row_index: u32,
    pub(crate) errors: Vec<RowError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidationReport {
    pub(crate) total_rows: u64,
    pub(crate) valid_rows: u64,
    pub(crate) invalid_rows: u64,
    pub(crate) rows: Vec<RowReport>,
}
