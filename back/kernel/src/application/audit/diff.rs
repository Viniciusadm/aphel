use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::application::shared::FieldChange;
use crate::errors::AppError;

pub const DEFAULT_SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "password_hash",
    "token",
    "token_hash",
    "secret",
    "api_key",
    "access_token",
    "refresh_token",
    "client_secret",
    "private_key",
];

const SENSITIVE_SUFFIXES: &[&str] = &["_password", "_secret", "_token", "_token_hash"];

#[derive(Debug, Clone, Copy, Default)]
pub struct DiffOptions<'a> {
    pub ignored: &'a [&'a str],
    pub sensitive: &'a [&'a str],
}

impl<'a> DiffOptions<'a> {
    pub fn ignoring(mut self, ignored: &'a [&'a str]) -> Self {
        self.ignored = ignored;
        self
    }

    pub fn redacting(mut self, sensitive: &'a [&'a str]) -> Self {
        self.sensitive = sensitive;
        self
    }

    fn is_ignored(&self, field: &str) -> bool {
        self.ignored.contains(&field)
    }

    fn is_sensitive(&self, field: &str) -> bool {
        self.sensitive.contains(&field)
            || DEFAULT_SENSITIVE_FIELDS.contains(&field)
            || SENSITIVE_SUFFIXES
                .iter()
                .any(|suffix| field.ends_with(suffix))
    }
}

pub fn diff_changes<O, N>(
    old: &O,
    new: &N,
    options: &DiffOptions<'_>,
) -> Result<Vec<FieldChange>, AppError>
where
    O: Serialize + ?Sized,
    N: Serialize + ?Sized,
{
    let old = serde_json::to_value(old).map_err(serialization_error)?;
    let new = serde_json::to_value(new).map_err(serialization_error)?;
    diff_values(&old, &new, options)
}

pub fn diff_values(
    old: &Value,
    new: &Value,
    options: &DiffOptions<'_>,
) -> Result<Vec<FieldChange>, AppError> {
    let empty = Map::new();
    let old = as_object(old, &empty)?;
    let new = as_object(new, &empty)?;
    let fields: BTreeSet<&String> = old.keys().chain(new.keys()).collect();

    Ok(fields
        .into_iter()
        .filter(|field| !options.is_ignored(field))
        .filter_map(|field| {
            let before = old.get(field).unwrap_or(&Value::Null);
            let after = new.get(field).unwrap_or(&Value::Null);
            if before == after {
                return None;
            }
            Some(if options.is_sensitive(field) {
                FieldChange::redacted(field.as_str())
            } else {
                FieldChange::new(field.as_str(), before.clone(), after.clone())
            })
        })
        .collect())
}

fn as_object<'v>(
    value: &'v Value,
    empty: &'v Map<String, Value>,
) -> Result<&'v Map<String, Value>, AppError> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(empty),
        other => Err(AppError::Internal(format!(
            "audit diff expects objects, got {}",
            kind_of(other)
        ))),
    }
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn serialization_error(error: serde_json::Error) -> AppError {
    AppError::Internal(format!("could not serialize audit snapshot: {error}"))
}
