use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::{RecordTypeDefinition, SystemField};
use crate::database::transaction::MySqlTx;
use crate::domain::fields::FieldType;
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::query::computed::system_expression;
use crate::records::InputValidator;

const RECORD_ALIAS: &str = "r";
const ID_COLUMN: &str = "contributed_record_id";

pub fn contributed_fields(definition: &RecordTypeDefinition) -> Vec<&SystemField> {
    definition
        .system_fields
        .iter()
        .filter(|field| field.contributed_by.is_some() && field.is_available())
        .collect()
}

fn writable_fields(definition: &RecordTypeDefinition) -> Vec<&SystemField> {
    contributed_fields(definition)
        .into_iter()
        .filter(|field| !field.is_computed())
        .collect()
}

pub fn is_writable_contributed(definition: &RecordTypeDefinition, key: &str) -> bool {
    writable_fields(definition)
        .iter()
        .any(|field| field.key.as_str() == key)
}

pub async fn load_contributed(
    pool: &MySqlPool,
    definition: &RecordTypeDefinition,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Map<String, Value>>, AppError> {
    let fields = contributed_fields(definition);
    if fields.is_empty() || ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut builder: QueryBuilder<MySql> =
        QueryBuilder::new(format!("SELECT {RECORD_ALIAS}.id AS {ID_COLUMN}"));
    for (index, field) in fields.iter().enumerate() {
        builder.push(format!(
            ", {} AS contributed_{index}",
            system_expression(field, RECORD_ALIAS)
        ));
    }
    builder.push(format!(
        " FROM `{}` {RECORD_ALIAS} WHERE {RECORD_ALIAS}.id IN (",
        definition.table
    ));
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
    let rows = builder.build().fetch_all(pool).await?;
    let mut values = HashMap::with_capacity(rows.len());
    for row in &rows {
        let id: String = row.try_get(ID_COLUMN)?;
        let id = Uuid::parse_str(&id)
            .map_err(|_| AppError::Internal(format!("invalid record id {id}")))?;
        let mut map = Map::new();
        for (index, field) in fields.iter().enumerate() {
            map.insert(
                field.key.as_str().to_owned(),
                decode(row, &format!("contributed_{index}"), field.field_type)?,
            );
        }
        values.insert(id, map);
    }
    Ok(values)
}

pub(crate) fn decode(
    row: &MySqlRow,
    column: &str,
    field_type: FieldType,
) -> Result<Value, AppError> {
    Ok(match field_type.base_type() {
        FieldType::Number | FieldType::Currency => {
            if let Ok(value) = row.try_get::<Option<i64>, _>(column) {
                json!(value)
            } else if let Ok(value) = row.try_get::<Option<u64>, _>(column) {
                json!(value)
            } else if let Ok(value) = row.try_get::<Option<Decimal>, _>(column) {
                json!(value.and_then(|value| value.to_f64()))
            } else {
                json!(row.try_get::<Option<f64>, _>(column)?)
            }
        }
        FieldType::Checkbox => json!(row.try_get::<Option<bool>, _>(column)?),
        FieldType::Date => json!(
            row.try_get::<Option<chrono::NaiveDate>, _>(column)?
                .map(|date| date.to_string())
        ),
        FieldType::DateTime => json!(
            row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column)?
                .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        ),
        _ => json!(row.try_get::<Option<String>, _>(column)?),
    })
}

pub fn contributed_input(
    definition: &RecordTypeDefinition,
    values: &Map<String, Value>,
) -> Map<String, Value> {
    values
        .iter()
        .filter(|(key, _)| is_writable_contributed(definition, key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub fn normalize_contributed(
    definition: &RecordTypeDefinition,
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

pub async fn write_contributed(
    tx: &mut MySqlTx,
    definition: &RecordTypeDefinition,
    record_id: Uuid,
    values: &Map<String, Value>,
) -> Result<(), AppError> {
    let fields: Vec<&SystemField> = writable_fields(definition)
        .into_iter()
        .filter(|field| values.contains_key(field.key.as_str()))
        .collect();
    if fields.is_empty() {
        return Ok(());
    }
    let mut builder: QueryBuilder<MySql> =
        QueryBuilder::new(format!("UPDATE `{}` SET ", definition.table));
    let mut separated = builder.separated(", ");
    for field in &fields {
        separated.push(format!("`{}` = ", field.column));
        match values.get(field.key.as_str()).unwrap_or(&Value::Null) {
            Value::Null => separated.push_bind_unseparated(None::<String>),
            Value::Bool(flag) => separated.push_bind_unseparated(*flag),
            Value::Number(number) => match number.as_i64() {
                Some(integer) => separated.push_bind_unseparated(integer),
                None => separated.push_bind_unseparated(number.as_f64()),
            },
            Value::String(text) => separated.push_bind_unseparated(text.clone()),
            other => separated.push_bind_unseparated(other.to_string()),
        };
    }
    builder
        .push(" WHERE id = ")
        .push_bind(record_id.to_string());
    builder.build().execute(&mut **tx).await?;
    Ok(())
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
    definition: &RecordTypeDefinition,
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
