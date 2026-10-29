use crate::application::fields::SystemFieldOption;
use crate::application::shared::RecordType;
use crate::domain::fields::{FieldConfig, FieldKey, FieldType};
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelNormalization {
    Exact,
    Lowercase,
    Domain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelAlias {
    pub column: String,
    pub normalization: LabelNormalization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedRecord {
    pub column: String,
    pub record_type: RecordType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluralLabels {
    pub portuguese: String,
    pub english: String,
}

pub const DEFAULT_RESTORE_ORDER: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCapabilities {
    pub importable: bool,
    pub exportable: bool,
    pub supports_stages: bool,
    pub supports_sequences: bool,
    pub personal_data: bool,
    pub notifies: bool,
    pub automatable: bool,
    pub attached_to_parent: bool,
    pub relatable: bool,
    pub validation_rules: bool,
    pub bulk_fields: Vec<String>,
    pub web_path: Option<String>,
    pub plural_labels: Option<PluralLabels>,
    pub contact_columns: Vec<String>,
    pub label_aliases: Vec<LabelAlias>,
    pub related_records: Vec<RelatedRecord>,
    pub free_text_fields: Vec<String>,
    pub locked_fields: Vec<String>,
    pub restore_order: u8,
}

impl Default for RecordCapabilities {
    fn default() -> Self {
        Self {
            importable: false,
            exportable: false,
            supports_stages: false,
            supports_sequences: false,
            personal_data: false,
            notifies: false,
            automatable: false,
            attached_to_parent: false,
            relatable: false,
            validation_rules: false,
            bulk_fields: Vec::new(),
            web_path: None,
            plural_labels: None,
            contact_columns: Vec::new(),
            label_aliases: Vec::new(),
            related_records: Vec::new(),
            free_text_fields: Vec::new(),
            locked_fields: Vec::new(),
            restore_order: DEFAULT_RESTORE_ORDER,
        }
    }
}

impl RecordCapabilities {
    pub fn importable(mut self) -> Self {
        self.importable = true;
        self
    }

    pub fn exportable(mut self) -> Self {
        self.exportable = true;
        self
    }

    pub fn stages(mut self) -> Self {
        self.supports_stages = true;
        self
    }

    pub fn sequences(mut self) -> Self {
        self.supports_sequences = true;
        self
    }

    pub fn personal_data(mut self) -> Self {
        self.personal_data = true;
        self
    }

    pub fn notifies(mut self) -> Self {
        self.notifies = true;
        self
    }

    pub fn automatable(mut self) -> Self {
        self.automatable = true;
        self
    }

    pub fn attached_to_parent(mut self) -> Self {
        self.attached_to_parent = true;
        self
    }

    pub fn relatable(mut self) -> Self {
        self.relatable = true;
        self
    }

    pub fn validation_rules(mut self) -> Self {
        self.validation_rules = true;
        self
    }

    pub fn bulk_fields(mut self, fields: &[&str]) -> Self {
        self.bulk_fields = fields.iter().map(|field| (*field).to_owned()).collect();
        self
    }

    pub fn web_path(mut self, path: &str) -> Self {
        self.web_path = Some(path.to_owned());
        self
    }

    pub fn plural_labels(mut self, portuguese: &str, english: &str) -> Self {
        self.plural_labels = Some(PluralLabels {
            portuguese: portuguese.to_owned(),
            english: english.to_owned(),
        });
        self
    }

    pub fn contact_columns(mut self, columns: &[&str]) -> Self {
        self.contact_columns = columns.iter().map(|column| (*column).to_owned()).collect();
        self
    }

    pub fn label_alias(mut self, column: &str, normalization: LabelNormalization) -> Self {
        self.label_aliases.push(LabelAlias {
            column: column.to_owned(),
            normalization,
        });
        self
    }

    pub fn related_record(mut self, column: &str, record_type: RecordType) -> Self {
        self.related_records.push(RelatedRecord {
            column: column.to_owned(),
            record_type,
        });
        self
    }

    pub fn free_text_fields(mut self, fields: &[&str]) -> Self {
        self.free_text_fields = fields.iter().map(|field| (*field).to_owned()).collect();
        self
    }

    pub fn locked_fields(mut self, fields: &[&str]) -> Self {
        self.locked_fields = fields.iter().map(|field| (*field).to_owned()).collect();
        self
    }

    pub fn restore_order(mut self, order: u8) -> Self {
        self.restore_order = order;
        self
    }

    pub fn allows_bulk(&self) -> bool {
        !self.bulk_fields.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub key: FieldKey,
    pub label: String,
    pub field_type: FieldType,
    pub config: FieldConfig,
    pub options: Vec<SystemFieldOption>,
}

impl FieldSpec {
    pub fn new(key: &str, label: &str, field_type: FieldType) -> Result<Self, AppError> {
        Ok(Self {
            key: FieldKey::new(key).map_err(|_| {
                AppError::Configuration(format!("invalid managed field key {key:?}"))
            })?,
            label: label.to_owned(),
            field_type,
            config: FieldConfig::default(),
            options: Vec::new(),
        })
    }

    pub fn with_config(mut self, config: FieldConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_option(mut self, value: &str, label: &str, color: Option<&str>) -> Self {
        self.options.push(SystemFieldOption {
            value: value.to_owned(),
            label: label.to_owned(),
            color: color.map(str::to_owned),
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManagedField {
    pub module: &'static str,
    pub spec: FieldSpec,
}
