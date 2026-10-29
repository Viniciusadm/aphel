use crate::modules::manifest::{SettingField, SettingKind};
use serde_json::{Map, Value};

use crate::errors::FieldViolation;

pub const MAX_TEXT_LENGTH: usize = 500;
pub const MAX_LONG_TEXT_LENGTH: usize = 20_000;
pub const MAX_SECRET_LENGTH: usize = 4096;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SettingsChange {
    pub values: Map<String, Value>,
    pub secrets: Vec<(String, Option<String>)>,
}

pub const SETTING_UNKNOWN: &str = "setting_unknown";
pub const SETTING_REQUIRED: &str = "setting_required";
pub const SETTING_TOO_LONG: &str = "setting_too_long";
pub const SETTING_NOT_TEXT: &str = "setting_not_text";
pub const SETTING_INVALID_EMAIL: &str = "setting_invalid_email";
pub const SETTING_INVALID_URL: &str = "setting_invalid_url";
pub const SETTING_NOT_NUMBER: &str = "setting_not_number";
pub const SETTING_OUT_OF_RANGE: &str = "setting_out_of_range";
pub const SETTING_NOT_BOOLEAN: &str = "setting_not_boolean";
pub const SETTING_INVALID_OPTION: &str = "setting_invalid_option";
pub const SETTING_INVALID_FIELD: &str = "setting_invalid_field";

fn path(key: &str) -> String {
    format!("settings.{key}")
}

type Rejection = (&'static str, String);

fn reject(code: &'static str, message: impl Into<String>) -> Rejection {
    (code, message.into())
}

fn violation(key: &str, (code, message): Rejection) -> FieldViolation {
    FieldViolation::new(path(key), message).with_code(code)
}

fn text(value: &Value, max: usize) -> Result<Value, Rejection> {
    match value {
        Value::String(text) if text.chars().count() <= max => {
            Ok(Value::String(text.trim().to_owned()))
        }
        Value::String(_) => Err(reject(
            SETTING_TOO_LONG,
            format!("must have at most {max} characters"),
        )),
        _ => Err(reject(SETTING_NOT_TEXT, "must be a string")),
    }
}

fn normalize(field: &SettingField, value: &Value) -> Result<Value, Rejection> {
    match &field.kind {
        SettingKind::Text => text(value, MAX_TEXT_LENGTH),
        SettingKind::LongText => text(value, MAX_LONG_TEXT_LENGTH),
        SettingKind::Email => {
            let value = text(value, MAX_TEXT_LENGTH)?;
            let valid = value
                .as_str()
                .and_then(|email| email.split_once('@'))
                .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
            if valid {
                Ok(value)
            } else {
                Err(reject(
                    SETTING_INVALID_EMAIL,
                    "must be a valid email address",
                ))
            }
        }
        SettingKind::Url => {
            let value = text(value, MAX_TEXT_LENGTH)?;
            let valid = value
                .as_str()
                .is_some_and(|url| url.starts_with("https://") || url.starts_with("http://"));
            if valid {
                Ok(value)
            } else {
                Err(reject(SETTING_INVALID_URL, "must be an http(s) URL"))
            }
        }
        SettingKind::Number { min, max } => {
            let number = value
                .as_f64()
                .ok_or_else(|| reject(SETTING_NOT_NUMBER, "must be a number"))?;
            if min.is_some_and(|min| number < min as f64)
                || max.is_some_and(|max| number > max as f64)
            {
                return Err(reject(
                    SETTING_OUT_OF_RANGE,
                    format!(
                        "must be between {} and {}",
                        min.map(|value| value.to_string())
                            .unwrap_or_else(|| "-∞".to_owned()),
                        max.map(|value| value.to_string())
                            .unwrap_or_else(|| "∞".to_owned())
                    ),
                ));
            }
            Ok(value.clone())
        }
        SettingKind::Boolean => match value {
            Value::Bool(_) => Ok(value.clone()),
            _ => Err(reject(SETTING_NOT_BOOLEAN, "must be a boolean")),
        },
        SettingKind::Select { options } => match value.as_str() {
            Some(selected) if options.iter().any(|option| option.value == selected) => {
                Ok(value.clone())
            }
            _ => Err(reject(SETTING_INVALID_OPTION, "is not one of the options")),
        },
        SettingKind::Field { .. } => match value.as_str() {
            Some(key)
                if !key.is_empty()
                    && key.len() <= 64
                    && key.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '_'
                    }) =>
            {
                Ok(value.clone())
            }
            _ => Err(reject(SETTING_INVALID_FIELD, "must be a field key")),
        },
        SettingKind::Secret => text(value, MAX_SECRET_LENGTH),
    }
}

pub fn parse_settings(
    fields: &[SettingField],
    current: &Map<String, Value>,
    input: &Map<String, Value>,
) -> Result<SettingsChange, Vec<FieldViolation>> {
    let mut change = SettingsChange {
        values: current.clone(),
        secrets: Vec::new(),
    };
    let mut violations = Vec::new();
    for (key, value) in input {
        let Some(field) = fields.iter().find(|field| field.key == key) else {
            violations.push(violation(
                key,
                reject(SETTING_UNKNOWN, "is not a setting of this module"),
            ));
            continue;
        };
        if field.is_secret() {
            match value {
                Value::Null => change.secrets.push((key.clone(), None)),
                Value::String(secret) if secret.trim().is_empty() => {}
                _ => match normalize(field, value) {
                    Ok(Value::String(secret)) => change.secrets.push((key.clone(), Some(secret))),
                    Ok(_) => {}
                    Err(rejection) => violations.push(violation(key, rejection)),
                },
            }
            continue;
        }
        if value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()) {
            change.values.remove(key);
            continue;
        }
        match normalize(field, value) {
            Ok(normalized) => {
                change.values.insert(key.clone(), normalized);
            }
            Err(rejection) => violations.push(violation(key, rejection)),
        }
    }
    change.values.retain(|key, _| {
        fields
            .iter()
            .any(|field| field.key == key && !field.is_secret())
    });
    if violations.is_empty() {
        Ok(change)
    } else {
        Err(violations)
    }
}

pub fn with_defaults(fields: &[SettingField], settings: &Map<String, Value>) -> Map<String, Value> {
    let mut resolved = settings.clone();
    for field in fields {
        if field.is_secret() || resolved.contains_key(field.key) {
            continue;
        }
        if let Some(default) = &field.default {
            resolved.insert(field.key.to_owned(), default.clone());
        }
    }
    resolved
}

pub fn missing_required(
    fields: &[SettingField],
    settings: &Map<String, Value>,
    configured_secrets: &[String],
) -> Vec<FieldViolation> {
    let resolved = with_defaults(fields, settings);
    fields
        .iter()
        .filter(|field| field.required)
        .filter(|field| {
            if field.is_secret() {
                !configured_secrets.iter().any(|key| key == field.key)
            } else {
                resolved.get(field.key).is_none_or(Value::is_null)
            }
        })
        .map(|field| violation(field.key, reject(SETTING_REQUIRED, "is required")))
        .collect()
}
