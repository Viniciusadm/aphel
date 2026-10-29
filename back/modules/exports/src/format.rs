use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{ExportCell, ID_COLUMN};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::fields::permissions::{NOT_ACCESSIBLE_MESSAGE, query_error};
use aphel_kernel::domain::fields::{FieldDefinition, FieldType};
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const MAX_EXPORT_COLUMNS: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ColumnSpec {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field: Option<FieldDefinition>,
}

pub(crate) fn resolve_columns(
    fields: &FieldSet,
    requested: &[String],
) -> Result<Vec<ColumnSpec>, AppError> {
    if requested.is_empty() {
        return Ok(fields
            .active()
            .map(|field| ColumnSpec {
                key: field.key().to_owned(),
                label: field.definition.label.clone(),
                field: Some(field.definition.clone()),
            })
            .collect());
    }
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    if requested.len() > MAX_EXPORT_COLUMNS {
        violations.push(FieldViolation::new(
            "columns",
            format!("must have at most {MAX_EXPORT_COLUMNS} columns"),
        ));
    }
    for (index, key) in requested.iter().enumerate() {
        let key = key.trim();
        if !seen.insert(key.to_owned()) {
            continue;
        }
        if key == ID_COLUMN {
            columns.push(ColumnSpec {
                key: ID_COLUMN.to_owned(),
                label: "ID".to_owned(),
                field: None,
            });
            continue;
        }
        match fields.find_active(key) {
            Some(field) => columns.push(ColumnSpec {
                key: key.to_owned(),
                label: field.definition.label.clone(),
                field: Some(field.definition.clone()),
            }),
            None if fields.is_hidden(key) => violations.push(FieldViolation::new(
                format!("columns.{index}"),
                NOT_ACCESSIBLE_MESSAGE,
            )),
            None => violations.push(FieldViolation::new(
                format!("columns.{index}"),
                "is not an active field",
            )),
        }
    }
    if !violations.is_empty() {
        return Err(query_error("invalid export columns", violations));
    }
    Ok(columns)
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct References {
    pub(crate) users: BTreeSet<String>,
    pub(crate) records: BTreeMap<String, BTreeSet<Uuid>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LabelSet {
    pub(crate) users: HashMap<String, String>,
    pub(crate) records: HashMap<(String, Uuid), String>,
}

fn texts(value: &Value) -> Vec<&str> {
    match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn collect_references(
    columns: &[ColumnSpec],
    rows: &[Map<String, Value>],
) -> References {
    let mut references = References::default();
    for column in columns {
        let Some(field) = &column.field else { continue };
        for row in rows {
            let Some(value) = row.get(&column.key) else {
                continue;
            };
            match field.field_type {
                FieldType::User => references
                    .users
                    .extend(texts(value).into_iter().map(str::to_owned)),
                FieldType::Record => {
                    if let Some(target) = &field.config.target_record_type {
                        references
                            .records
                            .entry(target.clone())
                            .or_default()
                            .extend(
                                texts(value)
                                    .into_iter()
                                    .filter_map(|text| Uuid::parse_str(text).ok()),
                            );
                    }
                }
                _ => {}
            }
        }
    }
    references
}

fn number_cell(value: &Value) -> ExportCell {
    match value {
        Value::Number(number) => number
            .as_f64()
            .map_or(ExportCell::Text(number.to_string()), ExportCell::Number),
        Value::String(text) => text
            .trim()
            .parse::<f64>()
            .map_or_else(|_| ExportCell::Text(text.clone()), ExportCell::Number),
        other => ExportCell::Text(other.to_string()),
    }
}

fn datetime_cell(text: &str, timezone: Tz) -> ExportCell {
    let instant = DateTime::parse_from_rfc3339(text)
        .map(|instant| instant.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S"]
                .iter()
                .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok())
                .map(|naive| naive.and_utc())
        });
    match instant {
        Some(instant) => ExportCell::Text(
            instant
                .with_timezone(&timezone)
                .format("%Y-%m-%d %H:%M")
                .to_string(),
        ),
        None => ExportCell::Text(text.to_owned()),
    }
}

fn option_label(field: &FieldDefinition, value: &str) -> String {
    field
        .option_by_value(value)
        .map_or_else(|| value.to_owned(), |option| option.label.clone())
}

pub(crate) fn format_cell(
    column: &ColumnSpec,
    record_id: Uuid,
    value: Option<&Value>,
    labels: &LabelSet,
    timezone: Tz,
) -> ExportCell {
    let Some(field) = &column.field else {
        return ExportCell::Text(record_id.to_string());
    };
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return ExportCell::Empty;
    };
    if let Value::String(text) = value
        && text.is_empty()
    {
        return ExportCell::Empty;
    }
    match field.field_type {
        FieldType::Number | FieldType::Currency => number_cell(value),
        FieldType::Checkbox => {
            let checked = match value {
                Value::Bool(checked) => *checked,
                Value::String(text) => matches!(text.as_str(), "1" | "true"),
                Value::Number(number) => number.as_i64() == Some(1),
                _ => false,
            };
            ExportCell::Text(if checked { "Sim" } else { "Não" }.to_owned())
        }
        FieldType::DateTime => match value {
            Value::String(text) => datetime_cell(text, timezone),
            other => ExportCell::Text(other.to_string()),
        },
        FieldType::Select => match value {
            Value::String(text) => ExportCell::Text(option_label(field, text)),
            other => ExportCell::Text(other.to_string()),
        },
        FieldType::MultiSelect => ExportCell::Text(
            texts(value)
                .into_iter()
                .map(|text| option_label(field, text))
                .collect::<Vec<_>>()
                .join("; "),
        ),
        FieldType::User => match value {
            Value::String(id) => {
                ExportCell::Text(labels.users.get(id).cloned().unwrap_or_else(|| id.clone()))
            }
            other => ExportCell::Text(other.to_string()),
        },
        FieldType::Record => match value {
            Value::String(id) => {
                let label = field
                    .config
                    .target_record_type
                    .as_ref()
                    .zip(Uuid::parse_str(id).ok())
                    .and_then(|(target, uuid)| labels.records.get(&(target.clone(), uuid)))
                    .cloned();
                ExportCell::Text(label.unwrap_or_else(|| id.clone()))
            }
            other => ExportCell::Text(other.to_string()),
        },
        FieldType::Text
        | FieldType::LongText
        | FieldType::Email
        | FieldType::Url
        | FieldType::Phone
        | FieldType::Date => match value {
            Value::String(text) => ExportCell::Text(text.clone()),
            other => ExportCell::Text(other.to_string()),
        },
    }
}
