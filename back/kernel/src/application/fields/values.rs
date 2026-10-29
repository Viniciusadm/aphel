use std::str::FromStr;

use chrono::SecondsFormat;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Number, Value};

use crate::domain::fields::{
    ConfigProperty, FieldConfig, FieldDefinition, FieldValue, FormulaResultType, FormulaSettings,
    RawValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub property: String,
    pub message: String,
}

impl ConfigError {
    fn new(property: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            property: property.into(),
            message: message.into(),
        }
    }
}

pub fn raw_from_json(value: &Value) -> Option<RawValue> {
    Some(match value {
        Value::Null => RawValue::Null,
        Value::Bool(value) => RawValue::Bool(*value),
        Value::Number(number) => RawValue::Number(number.to_string()),
        Value::String(text) => RawValue::Text(text.clone()),
        Value::Array(items) => {
            RawValue::List(items.iter().map(raw_from_json).collect::<Option<_>>()?)
        }
        Value::Object(_) => return None,
    })
}

pub fn raw_to_json(raw: &RawValue) -> Value {
    match raw {
        RawValue::Null => Value::Null,
        RawValue::Bool(value) => Value::Bool(*value),
        RawValue::Number(text) => decimal_json_from_str(text),
        RawValue::Text(text) => Value::String(text.clone()),
        RawValue::List(items) => Value::Array(items.iter().map(raw_to_json).collect()),
    }
}

fn decimal_json_from_str(text: &str) -> Value {
    Decimal::from_str(text)
        .map(decimal_json)
        .unwrap_or_else(|_| Value::String(text.to_owned()))
}

pub fn decimal_json(value: Decimal) -> Value {
    let normalized = value.normalize();
    if normalized.scale() == 0
        && let Some(integer) = normalized.to_i64()
    {
        return Value::Number(Number::from(integer));
    }
    normalized
        .to_f64()
        .and_then(Number::from_f64)
        .map(Value::Number)
        .unwrap_or_else(|| Value::String(normalized.to_string()))
}

pub fn value_to_json(definition: &FieldDefinition, value: &FieldValue) -> Value {
    match value {
        FieldValue::Text(text) | FieldValue::LongText(text) => Value::String(text.clone()),
        FieldValue::Number(number) => decimal_json(*number),
        FieldValue::Date(date) => Value::String(date.format("%Y-%m-%d").to_string()),
        FieldValue::DateTime(instant) => {
            Value::String(instant.to_rfc3339_opts(SecondsFormat::AutoSi, true))
        }
        FieldValue::Bool(value) => Value::Bool(*value),
        FieldValue::Option(id) => definition
            .option_by_id(*id)
            .map(|option| Value::String(option.value.clone()))
            .unwrap_or(Value::Null),
        FieldValue::Options(ids) => {
            let mut options: Vec<_> = ids
                .iter()
                .filter_map(|id| definition.option_by_id(*id))
                .collect();
            options.sort_by_key(|option| (option.position, option.value.clone()));
            Value::Array(
                options
                    .into_iter()
                    .map(|option| Value::String(option.value.clone()))
                    .collect(),
            )
        }
        FieldValue::User(id) => Value::String(id.as_str().to_owned()),
        FieldValue::Record(id) => Value::String(id.to_string()),
    }
}

pub fn config_from_json(value: &Value) -> Result<FieldConfig, Vec<ConfigError>> {
    let object = match value {
        Value::Null => return Ok(FieldConfig::default()),
        Value::Object(object) => object,
        _ => return Err(vec![ConfigError::new("config", "must be an object")]),
    };
    let mut config = FieldConfig::default();
    let mut errors = Vec::new();
    let mut expression = None;
    let mut result_type = None;
    for (property, value) in object {
        if value.is_null() {
            continue;
        }
        match property.as_str() {
            "min" => config.min = decimal_property(property, value, &mut errors),
            "max" => config.max = decimal_property(property, value, &mut errors),
            "decimals" => config.decimals = u32_property(property, value, &mut errors),
            "max_length" => config.max_length = u32_property(property, value, &mut errors),
            "currency" => config.currency = string_property(property, value, &mut errors),
            "pattern" => config.pattern = string_property(property, value, &mut errors),
            "default_country" => {
                config.default_country = string_property(property, value, &mut errors)
            }
            "target_record_type" => {
                config.target_record_type = string_property(property, value, &mut errors)
            }
            "default_value" => match raw_from_json(value) {
                Some(raw) => config.default_value = Some(raw),
                None => errors.push(ConfigError::new(property, "must not be an object")),
            },
            "expression" => match value.as_str() {
                Some(text) => expression = Some(text.to_owned()),
                None => errors.push(ConfigError::new(property, "must be a string")),
            },
            "result_type" => match value.as_str().and_then(FormulaResultType::parse) {
                Some(parsed) => result_type = Some(parsed),
                None => errors.push(ConfigError::new(
                    property,
                    "must be one of number, currency, date, text, boolean",
                )),
            },
            other => errors.push(ConfigError::new(other, "is not a known configuration")),
        }
    }
    match (expression, result_type) {
        (Some(expression), Some(result_type)) => {
            config.formula = Some(FormulaSettings {
                expression,
                result_type,
            })
        }
        (Some(_), None) => errors.push(ConfigError::new("result_type", "is required")),
        (None, Some(_)) => errors.push(ConfigError::new("expression", "is required")),
        (None, None) => {}
    }
    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors)
    }
}

pub fn config_to_json(config: &FieldConfig) -> Value {
    let mut object = Map::new();
    for property in config.present_properties() {
        let value = match property {
            ConfigProperty::Min => config.min.map(decimal_json),
            ConfigProperty::Max => config.max.map(decimal_json),
            ConfigProperty::Decimals => config.decimals.map(Value::from),
            ConfigProperty::Currency => config.currency.clone().map(Value::String),
            ConfigProperty::MaxLength => config.max_length.map(Value::from),
            ConfigProperty::Pattern => config.pattern.clone().map(Value::String),
            ConfigProperty::DefaultCountry => config.default_country.clone().map(Value::String),
            ConfigProperty::TargetRecordType => {
                config.target_record_type.clone().map(Value::String)
            }
            ConfigProperty::DefaultValue => config.default_value.as_ref().map(raw_to_json),
            ConfigProperty::Expression => config
                .formula
                .as_ref()
                .map(|formula| Value::String(formula.expression.clone())),
            ConfigProperty::ResultType => config
                .formula
                .as_ref()
                .map(|formula| Value::String(formula.result_type.as_str().to_owned())),
        };
        if let Some(value) = value {
            object.insert(property.as_str().to_owned(), value);
        }
    }
    Value::Object(object)
}

fn decimal_property(
    property: &str,
    value: &Value,
    errors: &mut Vec<ConfigError>,
) -> Option<Decimal> {
    let parsed = match value {
        Value::Number(number) => Decimal::from_str(&number.to_string())
            .or_else(|_| Decimal::from_scientific(&number.to_string()))
            .ok(),
        Value::String(text) => Decimal::from_str(text.trim()).ok(),
        _ => None,
    };
    if parsed.is_none() {
        errors.push(ConfigError::new(property, "must be a number"));
    }
    parsed
}

fn u32_property(property: &str, value: &Value, errors: &mut Vec<ConfigError>) -> Option<u32> {
    let parsed = value.as_u64().and_then(|number| u32::try_from(number).ok());
    if parsed.is_none() {
        errors.push(ConfigError::new(property, "must be a non-negative integer"));
    }
    parsed
}

fn string_property(property: &str, value: &Value, errors: &mut Vec<ConfigError>) -> Option<String> {
    match value.as_str().map(str::trim) {
        Some(text) if !text.is_empty() => Some(text.to_owned()),
        _ => {
            errors.push(ConfigError::new(property, "must be a non-empty string"));
            None
        }
    }
}
