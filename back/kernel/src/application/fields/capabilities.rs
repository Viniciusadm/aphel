use std::collections::BTreeMap;

use serde_json::{Value, json};

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

pub const DEFAULT_RESTORE_ORDER: u8 = 6;
pub const WEB_PATH_TRAIT: &str = "ui.web_path";
pub const PLURAL_LABELS_TRAIT: &str = "ui.plural_labels";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityTraits {
    pub bulk_fields: Vec<String>,
    pub traits: BTreeMap<String, Value>,
    pub label_aliases: Vec<LabelAlias>,
    pub related_records: Vec<RelatedRecord>,
    pub free_text_fields: Vec<String>,
    pub locked_fields: Vec<String>,
    pub restore_order: u8,
}

impl Default for EntityTraits {
    fn default() -> Self {
        Self {
            bulk_fields: Vec::new(),
            traits: BTreeMap::new(),
            label_aliases: Vec::new(),
            related_records: Vec::new(),
            free_text_fields: Vec::new(),
            locked_fields: Vec::new(),
            restore_order: DEFAULT_RESTORE_ORDER,
        }
    }
}

impl EntityTraits {
    pub fn importable(self) -> Self {
        self.with_trait("imports.importable")
    }

    pub fn exportable(self) -> Self {
        self.with_trait("exports.exportable")
    }

    pub fn personal_data(self) -> Self {
        self.with_trait("privacy.personal_data")
    }

    pub fn notifies(self) -> Self {
        self.with_trait("notifications.subject")
    }

    pub fn automatable(self) -> Self {
        self.with_trait("automations.subject")
    }

    pub fn attached_to_parent(self) -> Self {
        self.with_trait("records.attached_to_parent")
    }

    pub fn relatable(self) -> Self {
        self.with_trait("records.relatable")
    }

    pub fn validation_rules(self) -> Self {
        self.with_trait("validation_rules.subject")
    }

    pub fn bulk_fields(mut self, fields: &[&str]) -> Self {
        self.bulk_fields = fields.iter().map(|field| (*field).to_owned()).collect();
        self
    }

    pub fn with_trait(mut self, key: &str) -> Self {
        self.traits.insert(key.to_owned(), Value::Null);
        self
    }

    pub fn with_trait_data(mut self, key: &str, data: Value) -> Self {
        self.traits.insert(key.to_owned(), data);
        self
    }

    pub fn has_trait(&self, key: &str) -> bool {
        self.traits.contains_key(key)
    }

    pub fn trait_data(&self, key: &str) -> Option<&Value> {
        self.traits.get(key).filter(|data| !data.is_null())
    }

    pub fn web_path(self, path: &str) -> Self {
        self.with_trait_data(WEB_PATH_TRAIT, json!(path))
    }

    pub fn localized_labels(self, labels: super::LocalizedLabels) -> Self {
        self.with_trait_data(
            PLURAL_LABELS_TRAIT,
            serde_json::to_value(labels).expect("label map"),
        )
    }

    pub fn trait_columns_mapping(self, trait_key: &str, columns: &[&str]) -> Self {
        self.with_trait_data(trait_key, json!({ "columns": columns }))
    }

    pub fn web_path_value(&self) -> Option<&str> {
        self.trait_data(WEB_PATH_TRAIT).and_then(Value::as_str)
    }

    pub fn plural_labels_value(&self) -> Option<super::LocalizedLabels> {
        serde_json::from_value(self.trait_data(PLURAL_LABELS_TRAIT)?.clone()).ok()
    }

    pub fn trait_columns(&self, trait_key: &str) -> Vec<String> {
        self.trait_data(trait_key)
            .and_then(|data| data.get("columns"))
            .and_then(Value::as_array)
            .map(|columns| {
                columns
                    .iter()
                    .filter_map(|column| column.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
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
    pub labels: super::LocalizedLabels,
    pub field_type: FieldType,
    pub config: FieldConfig,
    pub options: Vec<SystemFieldOption>,
}

impl FieldSpec {
    pub fn new(
        key: &str,
        label: impl Into<super::LocalizedLabels>,
        field_type: FieldType,
    ) -> Result<Self, AppError> {
        let labels = label.into();
        Ok(Self {
            key: FieldKey::new(key).map_err(|_| {
                AppError::Configuration(format!("invalid managed field key {key:?}"))
            })?,
            label: labels.default_text().to_owned(),
            labels,
            field_type,
            config: FieldConfig::default(),
            options: Vec::new(),
        })
    }

    pub fn with_config(mut self, config: FieldConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_option(
        mut self,
        value: &str,
        label: impl Into<super::LocalizedLabels>,
        color: Option<&str>,
    ) -> Self {
        let labels = label.into();
        self.options.push(SystemFieldOption {
            value: value.to_owned(),
            label: labels.default_text().to_owned(),
            labels,
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
