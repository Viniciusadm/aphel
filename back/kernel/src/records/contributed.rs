use serde_json::{Map, Value};
use uuid::Uuid;

use crate::application::fields::{EntityDefinition, SystemField};
use crate::domain::fields::FieldType;
use crate::errors::FieldViolation;
use crate::records::InputValidator;

pub fn contributed_fields(definition: &EntityDefinition) -> Vec<&SystemField> {
    definition
        .system_fields
        .iter()
        .filter(|field| field.contributed_by.is_some() && field.is_available())
        .collect()
}

pub(crate) fn writable_fields(definition: &EntityDefinition) -> Vec<&SystemField> {
    contributed_fields(definition)
        .into_iter()
        .filter(|field| !field.is_computed())
        .collect()
}

pub fn is_writable_contributed(definition: &EntityDefinition, key: &str) -> bool {
    writable_fields(definition)
        .iter()
        .any(|field| field.key.as_str() == key)
}

pub fn contributed_input(
    definition: &EntityDefinition,
    values: &Map<String, Value>,
) -> Map<String, Value> {
    values
        .iter()
        .filter(|(key, _)| is_writable_contributed(definition, key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub fn normalize_contributed(
    definition: &EntityDefinition,
    input: &Map<String, Value>,
    violations: &mut Vec<FieldViolation>,
) -> Map<String, Value> {
    let mut normalized = Map::new();
    for field in writable_fields(definition) {
        let key = field.key.as_str();
        let Some(value) = input.get(key) else {
            continue;
        };
        match normalize(field.field_type, value) {
            Ok(value) => {
                normalized.insert(key.to_owned(), value);
            }
            Err(message) => violations.push(FieldViolation::new(key, message)),
        }
    }
    normalized
}

fn normalize(field_type: FieldType, value: &Value) -> Result<Value, &'static str> {
    match (field_type.base_type(), value) {
        (_, Value::Null) => Ok(Value::Null),
        (FieldType::Number | FieldType::Currency, Value::Number(_)) => Ok(value.clone()),
        (FieldType::Number | FieldType::Currency, _) => Err("must be a number"),
        (FieldType::Checkbox, Value::Bool(_)) => Ok(value.clone()),
        (FieldType::Checkbox, _) => Err("must be true or false"),
        (FieldType::Record | FieldType::User, Value::String(text)) => {
            let text = text.trim();
            if text.is_empty() {
                Ok(Value::Null)
            } else if Uuid::parse_str(text).is_ok() {
                Ok(Value::String(text.to_owned()))
            } else {
                Err("must be a record id")
            }
        }
        (FieldType::Record | FieldType::User, _) => Err("must be a record id"),
        (_, Value::String(text)) => {
            let text = text.trim();
            Ok(if text.is_empty() {
                Value::Null
            } else {
                Value::String(text.to_owned())
            })
        }
        (_, Value::Number(number)) => Ok(Value::String(number.to_string())),
        _ => Err("must be a text value"),
    }
}

pub fn with_contributed(snapshot: Value, contributed: &Map<String, Value>) -> Value {
    let mut snapshot = snapshot;
    if let Value::Object(map) = &mut snapshot {
        for (key, value) in contributed {
            map.insert(key.clone(), value.clone());
        }
    }
    snapshot
}

pub fn merge_contributed(
    definition: &EntityDefinition,
    current: &Map<String, Value>,
    input: &Map<String, Value>,
    validator: &mut InputValidator,
) -> Map<String, Value> {
    let mut merged = contributed_input(definition, current);
    let mut violations = Vec::new();
    let changes = normalize_contributed(
        definition,
        &contributed_input(definition, input),
        &mut violations,
    );
    for violation in violations {
        validator.violation(&violation.field, violation.message);
    }
    merged.extend(changes);
    merged
}
