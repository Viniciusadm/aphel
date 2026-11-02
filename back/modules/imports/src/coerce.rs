use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use crate::normalize_label;
use aphel_kernel::application::fields::raw_from_json;
use aphel_kernel::domain::fields::{FieldDefinition, FieldType, validate_value};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CellValue {
    Value(Value),
    UserEmail(String),
    RecordLabel { target: String, label: String },
}

const TRUE_WORDS: &[&str] = &["sim", "s", "yes", "y", "true", "1", "x", "verdadeiro", "v"];
const FALSE_WORDS: &[&str] = &["nao", "n", "no", "false", "0", "falso", "f"];
const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y", "%Y/%m/%d"];
const TIME_FORMATS: &[&str] = &["%H:%M:%S", "%H:%M"];

pub(crate) fn coerce_cell(
    definition: &FieldDefinition,
    raw: &str,
    timezone: Tz,
) -> Result<Option<CellValue>, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value = match definition.field_type.base_type() {
        FieldType::Custom { .. } => unreachable!("native field representation"),
        FieldType::Number | FieldType::Currency => {
            CellValue::Value(Value::String(parse_number(text)?))
        }
        FieldType::Date => CellValue::Value(Value::String(
            parse_date(text)?.format("%Y-%m-%d").to_string(),
        )),
        FieldType::DateTime => CellValue::Value(Value::String(parse_datetime(text, timezone)?)),
        FieldType::Checkbox => CellValue::Value(Value::Bool(parse_bool(text)?)),
        FieldType::Select => CellValue::Value(Value::String(match_option(definition, text)?)),
        FieldType::MultiSelect => CellValue::Value(Value::Array(
            split_list(text)
                .into_iter()
                .map(|item| match_option(definition, item).map(Value::String))
                .collect::<Result<_, _>>()?,
        )),
        FieldType::User => {
            if Uuid::parse_str(text).is_ok() {
                CellValue::Value(Value::String(text.to_owned()))
            } else if text.contains('@') {
                CellValue::UserEmail(text.to_lowercase())
            } else {
                return Err("must be the email of a member of the organization".to_owned());
            }
        }
        FieldType::Record => {
            if Uuid::parse_str(text).is_ok() {
                CellValue::Value(Value::String(text.to_owned()))
            } else {
                let Some(target) = definition.config.target_record_type.clone() else {
                    return Err("must be a record id".to_owned());
                };
                CellValue::RecordLabel {
                    target,
                    label: text.to_owned(),
                }
            }
        }
        FieldType::Text
        | FieldType::LongText
        | FieldType::Email
        | FieldType::Url
        | FieldType::Phone => CellValue::Value(Value::String(text.to_owned())),
    };
    Ok(Some(value))
}

pub(crate) fn check_value(definition: &FieldDefinition, value: &Value) -> Result<(), String> {
    let Some(raw) = raw_from_json(value) else {
        return Err("has an unsupported value".to_owned());
    };
    validate_value(definition, &raw)
        .map(|_| ())
        .map_err(|error| error.message())
}

pub(crate) fn parse_number(text: &str) -> Result<String, String> {
    let cleaned: String = text
        .chars()
        .filter(|character| {
            character.is_ascii_digit() || matches!(character, '.' | ',' | '-' | '+')
        })
        .collect();
    let invalid = || "must be a number".to_owned();
    if cleaned.is_empty()
        || text
            .chars()
            .any(|character| character.is_alphabetic() && !matches!(character, 'R' | 'r'))
    {
        return Err(invalid());
    }
    let last_dot = cleaned.rfind('.');
    let last_comma = cleaned.rfind(',');
    let normalized = match (last_dot, last_comma) {
        (Some(dot), Some(comma)) if comma > dot => cleaned.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => cleaned.replace(',', ""),
        (None, Some(_)) => {
            if cleaned.matches(',').count() > 1 {
                cleaned.replace(',', "")
            } else {
                cleaned.replace(',', ".")
            }
        }
        (Some(_), None) if cleaned.matches('.').count() > 1 => cleaned.replace('.', ""),
        _ => cleaned,
    };
    let normalized = normalized.trim_start_matches('+').to_owned();
    if normalized.parse::<f64>().is_err() {
        return Err(invalid());
    }
    Ok(normalized)
}

pub(crate) fn parse_date(text: &str) -> Result<NaiveDate, String> {
    let date_part = text.split(['T', ' ']).next().unwrap_or(text).trim();
    DATE_FORMATS
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(date_part, format).ok())
        .ok_or_else(|| "must be a date such as 31/12/2026 or 2026-12-31".to_owned())
}

pub(crate) fn parse_datetime(text: &str, timezone: Tz) -> Result<String, String> {
    if let Ok(instant) = chrono::DateTime::parse_from_rfc3339(text) {
        return Ok(instant.with_timezone(&Utc).to_rfc3339());
    }
    let invalid = || "must be a date and time such as 31/12/2026 14:30".to_owned();
    let mut parts = text.splitn(2, [' ', 'T']);
    let date = parse_date(parts.next().unwrap_or_default()).map_err(|_| invalid())?;
    let time = match parts.next().map(str::trim) {
        None | Some("") => NaiveTime::MIN,
        Some(time) => TIME_FORMATS
            .iter()
            .find_map(|format| NaiveTime::parse_from_str(time, format).ok())
            .ok_or_else(invalid)?,
    };
    let local = NaiveDateTime::new(date, time);
    timezone
        .from_local_datetime(&local)
        .earliest()
        .map(|instant| instant.with_timezone(&Utc).to_rfc3339())
        .ok_or_else(invalid)
}

pub(crate) fn parse_bool(text: &str) -> Result<bool, String> {
    let normalized = normalize_label(text);
    if TRUE_WORDS.contains(&normalized.as_str()) {
        Ok(true)
    } else if FALSE_WORDS.contains(&normalized.as_str()) {
        Ok(false)
    } else {
        Err("must be sim or não".to_owned())
    }
}

fn match_option(definition: &FieldDefinition, text: &str) -> Result<String, String> {
    let text = text.trim();
    let normalized = normalize_label(text);
    definition
        .options
        .iter()
        .filter(|option| option.is_active)
        .find(|option| option.value == text || normalize_label(&option.label) == normalized)
        .or_else(|| {
            definition
                .options
                .iter()
                .filter(|option| option.is_active)
                .find(|option| normalize_label(&option.value) == normalized)
        })
        .map(|option| option.value.clone())
        .ok_or_else(|| format!("option {text:?} does not exist"))
}

fn split_list(text: &str) -> Vec<&str> {
    let separator: &[char] = if text.contains(';') || text.contains('|') {
        &[';', '|']
    } else {
        &[',']
    };
    text.split(separator)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect()
}
