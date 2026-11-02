use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use aphel_kernel::domain::{Email, Secret};
use aphel_kernel::errors::FieldViolation;

pub const MAX_TEXT_LENGTH: usize = 500;
pub const MAX_SECRET_LENGTH: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFieldType {
    Text,
    Number,
    Boolean,
    Select,
    Secret,
    Email,
    Url,
}

impl ConfigFieldType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Select => "select",
            Self::Secret => "secret",
            Self::Email => "email",
            Self::Url => "url",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigField {
    pub key: &'static str,
    pub field_type: ConfigFieldType,
    pub required: bool,
    pub options: Vec<&'static str>,
    pub default: Option<Value>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

impl ConfigField {
    pub fn new(key: &'static str, field_type: ConfigFieldType) -> Self {
        Self {
            key,
            field_type,
            required: false,
            options: Vec::new(),
            default: None,
            min: None,
            max: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_options(mut self, options: &[&'static str]) -> Self {
        self.options = options.to_vec();
        self
    }

    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_range(mut self, min: i64, max: i64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn is_secret(&self) -> bool {
        self.field_type == ConfigFieldType::Secret
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigSchema {
    pub fields: Vec<ConfigField>,
}

impl ConfigSchema {
    pub fn new(fields: Vec<ConfigField>) -> Self {
        Self { fields }
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn secret_keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.fields
            .iter()
            .filter(|field| field.is_secret())
            .map(|field| field.key)
    }

    pub fn field(&self, key: &str) -> Option<&ConfigField> {
        self.fields.iter().find(|field| field.key == key)
    }
}

#[derive(Clone, Default)]
pub struct ResolvedConfig {
    pub values: Map<String, Value>,
    pub secrets: BTreeMap<String, Secret>,
}

impl std::fmt::Debug for ResolvedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedConfig")
            .field("values", &self.values)
            .field("secrets", &self.secrets.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ResolvedConfig {
    pub fn text(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn integer(&self, key: &str) -> Option<i64> {
        self.values.get(key).and_then(Value::as_i64)
    }

    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(Value::as_bool)
    }

    pub fn secret(&self, key: &str) -> Option<&Secret> {
        self.secrets.get(key)
    }
}

#[derive(Debug, Default)]
pub struct ConfigInput {
    pub values: Map<String, Value>,
    pub clear_secrets: Vec<String>,
}

#[derive(Default)]
pub struct ValidatedConfig {
    pub values: Map<String, Value>,
    pub secrets_to_store: Vec<(String, Secret)>,
    pub secrets_to_clear: Vec<String>,
}

fn is_blank(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        _ => false,
    }
}

fn looks_like_url(value: &str) -> bool {
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"));
    rest.is_some_and(|rest| {
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        !host.is_empty() && !host.contains(char::is_whitespace)
    })
}

fn normalize_value(field: &ConfigField, value: &Value) -> Result<Value, String> {
    match field.field_type {
        ConfigFieldType::Text | ConfigFieldType::Email | ConfigFieldType::Url => {
            let text = value.as_str().ok_or("must be a string")?.trim();
            if text.chars().count() > MAX_TEXT_LENGTH {
                return Err(format!("must have at most {MAX_TEXT_LENGTH} characters"));
            }
            if field.field_type == ConfigFieldType::Email && Email::new(text).is_err() {
                return Err("must be a valid email address".to_owned());
            }
            if field.field_type == ConfigFieldType::Url && !looks_like_url(text) {
                return Err("must be an http or https URL".to_owned());
            }
            Ok(Value::String(text.to_owned()))
        }
        ConfigFieldType::Number => {
            let number = match value {
                Value::Number(number) => number.as_i64(),
                Value::String(text) => text.trim().parse::<i64>().ok(),
                _ => None,
            }
            .ok_or("must be an integer")?;
            if field.min.is_some_and(|min| number < min)
                || field.max.is_some_and(|max| number > max)
            {
                return Err(format!(
                    "must be between {} and {}",
                    field.min.unwrap_or(i64::MIN),
                    field.max.unwrap_or(i64::MAX)
                ));
            }
            Ok(Value::from(number))
        }
        ConfigFieldType::Boolean => value
            .as_bool()
            .map(Value::Bool)
            .ok_or_else(|| "must be true or false".to_owned()),
        ConfigFieldType::Select => {
            let text = value.as_str().ok_or("must be a string")?.trim();
            if field.options.contains(&text) {
                Ok(Value::String(text.to_owned()))
            } else {
                Err(format!("must be one of: {}", field.options.join(", ")))
            }
        }
        ConfigFieldType::Secret => Err("secret fields are stored separately".to_owned()),
    }
}

pub fn validate_config(
    schema: &ConfigSchema,
    input: ConfigInput,
    configured_secrets: &BTreeSet<String>,
) -> Result<ValidatedConfig, Vec<FieldViolation>> {
    let mut violations = Vec::new();
    let mut validated = ValidatedConfig::default();

    for key in input.values.keys() {
        if schema.field(key).is_none() {
            violations.push(FieldViolation::new(
                format!("config.{key}"),
                "is not a field of this integration",
            ));
        }
    }
    for key in &input.clear_secrets {
        match schema.field(key) {
            Some(field) if field.is_secret() && !field.required => {
                validated.secrets_to_clear.push(key.clone())
            }
            Some(field) if field.is_secret() => violations.push(FieldViolation::new(
                format!("config.{key}"),
                "is required and cannot be cleared",
            )),
            _ => violations.push(FieldViolation::new(
                "clear_secrets",
                format!("{key} is not a secret field of this integration"),
            )),
        }
    }

    for field in &schema.fields {
        let path = format!("config.{}", field.key);
        let provided = input.values.get(field.key).filter(|value| !is_blank(value));
        if field.is_secret() {
            match provided {
                Some(Value::String(text)) => {
                    if text.len() > MAX_SECRET_LENGTH {
                        violations.push(FieldViolation::new(
                            path,
                            format!("must have at most {MAX_SECRET_LENGTH} bytes"),
                        ));
                    } else if validated
                        .secrets_to_clear
                        .iter()
                        .any(|key| key == field.key)
                    {
                        violations.push(FieldViolation::new(
                            path,
                            "cannot be set and cleared at the same time",
                        ));
                    } else {
                        validated
                            .secrets_to_store
                            .push((field.key.to_owned(), Secret::new(text.clone())));
                    }
                }
                Some(_) => violations.push(FieldViolation::new(path, "must be a string")),
                None => {
                    if field.required && !configured_secrets.contains(field.key) {
                        violations.push(FieldViolation::new(path, "is required"));
                    }
                }
            }
            continue;
        }
        let value = provided.or(field.default.as_ref());
        match value {
            Some(value) => match normalize_value(field, value) {
                Ok(normalized) => {
                    validated.values.insert(field.key.to_owned(), normalized);
                }
                Err(message) => violations.push(FieldViolation::new(path, message)),
            },
            None if field.required => violations.push(FieldViolation::new(path, "is required")),
            None => {}
        }
    }

    if violations.is_empty() {
        Ok(validated)
    } else {
        Err(violations)
    }
}

pub fn missing_required(schema: &ConfigSchema, config: &ResolvedConfig) -> Vec<FieldViolation> {
    schema
        .fields
        .iter()
        .filter(|field| field.required)
        .filter(|field| {
            if field.is_secret() {
                config.secret(field.key).is_none()
            } else {
                config.values.get(field.key).is_none_or(is_blank)
            }
        })
        .map(|field| FieldViolation::new(format!("config.{}", field.key), "is required"))
        .collect()
}
