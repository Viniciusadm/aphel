use aphel_sdk::imports::ImportTraits;
use std::collections::HashMap;

use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    CellValue, DuplicateKey, ImportTarget, RecordValues, RowError, RowOutcome, check_value,
    coerce_cell,
};
use aphel_kernel::application::custom_objects::resolve_relation;

use aphel_kernel::application::fields::{EntityDefinition, FieldSet, RecordTypeRegistry};
use aphel_kernel::domain::custom_objects::RelationSide;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::domain::fields::{normalize_email, normalize_phone};
use aphel_kernel::errors::AppError;
use aphel_kernel::records::normalize_domain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResolveMode {
    Validate,
    Run,
}

#[derive(Debug, Default)]
pub(super) struct ResolutionCache {
    pub(super) users: HashMap<String, Option<String>>,
    pub(super) records: HashMap<(String, String), Option<Uuid>>,
    pub(super) creatable: HashMap<String, Option<CreationPlan>>,
}

#[derive(Debug, Clone)]
pub(super) struct CreationPlan {
    pub(super) title: String,
    pub(super) domain: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct PendingUser {
    pub(super) key: String,
    pub(super) is_system: bool,
    pub(super) email: String,
    pub(super) column: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct PendingRecord {
    pub(super) key: String,
    pub(super) is_system: bool,
    pub(super) multiple: bool,
    pub(super) target: String,
    pub(super) label: String,
    pub(super) column: Option<String>,
}

pub(super) enum PreparedRow {
    Done(RowOutcome),
    Ready {
        row_index: u32,
        values: RecordValues,
        duplicate: Option<String>,
    },
}

#[derive(Debug, Default)]
pub(super) struct PlannedRow {
    pub(super) values: RecordValues,
    pub(super) users: Vec<PendingUser>,
    pub(super) records: Vec<PendingRecord>,
    pub(super) errors: Vec<RowError>,
}

impl PlannedRow {
    pub(super) fn set_record(&mut self, record: &PendingRecord, id: Uuid) {
        let value = Value::String(id.to_string());
        if !record.multiple {
            self.set(&record.key, record.is_system, value);
            return;
        }
        if let Some(relation) = record.key.strip_prefix(RELATION_TARGET_PREFIX) {
            let entry = self
                .values
                .relations
                .entry(relation.to_owned())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(ids) = entry
                && !ids.contains(&value)
            {
                ids.push(value);
            }
        }
    }

    pub(super) fn set(&mut self, key: &str, is_system: bool, value: Value) {
        if let Some(relation) = key.strip_prefix(RELATION_TARGET_PREFIX) {
            self.values.relations.insert(relation.to_owned(), value);
            return;
        }
        let target = if is_system {
            &mut self.values.system
        } else {
            &mut self.values.custom
        };
        target.insert(key.to_owned(), value);
    }

    fn has(&self, key: &str) -> bool {
        self.values.system.contains_key(key)
            || self.values.custom.contains_key(key)
            || self.users.iter().any(|user| user.key == key)
            || self.records.iter().any(|record| record.key == key)
    }
}

pub(super) struct RowEvaluator<'a> {
    pub(super) fields: &'a FieldSet,
    pub(super) targets: &'a [ImportTarget],
    pub(super) headers: &'a [String],
    pub(super) mapping: &'a [Option<String>],
    pub(super) timezone: Tz,
}

impl RowEvaluator<'_> {
    fn column_of(&self, key: &str) -> Option<String> {
        let normalized = key.strip_prefix("custom_fields.").unwrap_or(key);
        self.mapping
            .iter()
            .position(|target| target.as_deref() == Some(normalized))
            .and_then(|index| self.headers.get(index).cloned())
    }

    pub(super) fn plan(&self, cells: &[String]) -> PlannedRow {
        let mut planned = PlannedRow::default();
        for (index, target) in self.mapping.iter().enumerate() {
            let Some(key) = target else { continue };
            if let Some(target) = self
                .targets
                .iter()
                .find(|candidate| &candidate.key == key && candidate.relation_target.is_some())
            {
                let relation_target = target.relation_target.clone().unwrap_or_default();
                let cell = cells.get(index).map(|cell| cell.trim()).unwrap_or_default();
                let labels: Vec<&str> = if target.multiple {
                    split_labels(cell)
                } else {
                    vec![cell]
                };
                for label in labels.into_iter().filter(|label| !label.is_empty()) {
                    planned.records.push(PendingRecord {
                        key: key.clone(),
                        is_system: false,
                        multiple: target.multiple,
                        target: relation_target.clone(),
                        label: label.to_owned(),
                        column: self.headers.get(index).cloned(),
                    });
                }
                continue;
            }
            let Some(field) = self.fields.find_active(key) else {
                continue;
            };
            let column = self.headers.get(index).cloned();
            let raw = cells.get(index).map(String::as_str).unwrap_or_default();
            let error = |message: String| RowError {
                field: Some(key.clone()),
                column: column.clone(),
                message,
            };
            match coerce_cell(&field.definition, raw, self.timezone) {
                Err(message) => planned.errors.push(error(message)),
                Ok(None) => {}
                Ok(Some(CellValue::Value(value))) => match check_value(&field.definition, &value) {
                    Ok(()) => planned.set(key, field.is_system(), value),
                    Err(message) => planned.errors.push(error(message)),
                },
                Ok(Some(CellValue::UserEmail(email))) => planned.users.push(PendingUser {
                    key: key.clone(),
                    is_system: field.is_system(),
                    email,
                    column,
                }),
                Ok(Some(CellValue::RecordLabel { target, label })) => {
                    planned.records.push(PendingRecord {
                        key: key.clone(),
                        is_system: field.is_system(),
                        multiple: false,
                        target,
                        label,
                        column,
                    })
                }
            }
        }
        for target in self.targets.iter().filter(|target| target.required) {
            if planned.has(&target.key)
                || target
                    .satisfied_by
                    .as_deref()
                    .is_some_and(|key| planned.has(key))
            {
                continue;
            }
            if planned
                .errors
                .iter()
                .any(|error| error.field.as_deref() == Some(target.key.as_str()))
            {
                continue;
            }
            planned.errors.push(RowError {
                field: Some(target.key.clone()),
                column: self.column_of(&target.key),
                message: "is required".to_owned(),
            });
        }
        planned
    }

    pub(super) fn errors_from(&self, error: AppError) -> Vec<RowError> {
        match error {
            AppError::InvalidFields { fields, .. } if !fields.is_empty() => fields
                .into_iter()
                .map(|violation| {
                    let key = violation
                        .field
                        .strip_prefix("custom_fields.")
                        .unwrap_or(&violation.field)
                        .to_owned();
                    RowError {
                        column: self.column_of(&key),
                        field: Some(key),
                        message: violation.message,
                    }
                })
                .collect(),
            other => vec![RowError::general(describe(&other))],
        }
    }
}

pub const RELATION_TARGET_PREFIX: &str = "relations.";

pub fn import_targets(
    fields: &FieldSet,
    registry: &RecordTypeRegistry,
) -> Result<Vec<ImportTarget>, AppError> {
    let policy = fields.record_type.capabilities.import_policy_data()?;
    let creatable = |target: Option<&str>| -> Result<bool, AppError> {
        target
            .and_then(|key| registry.require(key).ok())
            .map(|definition| {
                definition
                    .capabilities
                    .import_policy_data()
                    .map(|policy| policy.label_creation)
            })
            .unwrap_or(Ok(false))
    };
    let mut targets: Vec<_> = fields
        .active()
        .filter(|field| {
            field.is_editable() && !policy.excluded_fields.iter().any(|key| key == field.key())
        })
        .map(|field| {
            Ok(ImportTarget {
                key: field.key().to_owned(),
                label: field.definition.label.clone(),
                field_type: field.definition.field_type,
                required: field.definition.is_required,
                is_custom: !field.is_system(),
                relation_target: None,
                multiple: false,
                satisfied_by: policy.satisfied_by.get(field.key()).cloned(),
                aliases: policy.aliases.get(field.key()).cloned().unwrap_or_default(),
                can_create_reference: creatable(
                    field.definition.config.target_record_type.as_deref(),
                )?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    for relation in &fields.record_type.relations {
        let Some(resolved) = resolve_relation(&registry.custom_schema().relationships, relation)
        else {
            continue;
        };
        let key = format!("{RELATION_TARGET_PREFIX}{}", relation.key);
        targets.push(ImportTarget {
            aliases: policy.aliases.get(&key).cloned().unwrap_or_default(),
            can_create_reference: creatable(Some(relation.target.as_str()))?,
            key,
            label: relation.label.clone(),
            field_type: FieldType::Record,
            required: resolved.relationship.required && resolved.side == RelationSide::To,
            is_custom: false,
            relation_target: Some(relation.target.as_str().to_owned()),
            multiple: !resolved.relationship.single(resolved.side),
            satisfied_by: None,
        });
    }
    Ok(targets)
}

pub(crate) const MULTIPLE_VALUE_SEPARATORS: [char; 3] = [';', '|', '\n'];

fn split_labels(cell: &str) -> Vec<&str> {
    let mut labels: Vec<&str> = Vec::new();
    for label in cell.split(MULTIPLE_VALUE_SEPARATORS).map(str::trim) {
        if !label.is_empty() && !labels.contains(&label) {
            labels.push(label);
        }
    }
    labels
}

pub(super) fn duplicate_value(
    key: DuplicateKey,
    definition: &EntityDefinition,
    values: &RecordValues,
) -> Option<String> {
    let mut fields: Vec<String> = key
        .columns(definition)
        .into_iter()
        .map(|column| column.field)
        .collect();
    fields.extend(key.fallback_fields(definition));
    let raw = fields.iter().find_map(|field| values.text(field))?;
    match key {
        DuplicateKey::None => None,
        DuplicateKey::Email => normalize_email(raw).ok(),
        DuplicateKey::Phone => normalize_phone(raw, "BR").ok(),
        DuplicateKey::Domain => normalize_domain(raw),
        DuplicateKey::Name => Some(raw.to_owned()),
    }
}

pub(super) fn file_stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

pub(super) fn describe(error: &AppError) -> String {
    match error {
        AppError::InvalidFields { fields, message }
        | AppError::CodedFields {
            fields, message, ..
        } => {
            if fields.is_empty() {
                message.clone()
            } else {
                fields
                    .iter()
                    .map(|field| format!("{} {}", field.field, field.message))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
        AppError::Coded { message, .. } => message.clone(),
        other => other.to_string(),
    }
}
