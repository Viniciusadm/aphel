use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::{EntityDefinition, SystemField};
use crate::database::Db;
use crate::database::transaction::Tx;
use crate::domain::fields::FieldType;
use crate::errors::AppError;
use crate::infrastructure::query::computed::system_expression;
use crate::records::contributed::{contributed_fields, writable_fields};

const RECORD_ALIAS: &str = "r";
const ID_COLUMN: &str = "contributed_record_id";

pub async fn load_contributed(
    pool: &Db,
    definition: &EntityDefinition,
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
                decode_record_field(row, &format!("contributed_{index}"), field.field_type)?,
            );
        }
        values.insert(id, map);
    }
    Ok(values)
}

pub fn decode_record_field(
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
        FieldType::MultiSelect | FieldType::References => row
            .try_get::<Option<sqlx::types::Json<Value>>, _>(column)?
            .map(|json| json.0)
            .unwrap_or(Value::Null),
        FieldType::DateTime => json!(
            row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column)?
                .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        ),
        _ => json!(row.try_get::<Option<String>, _>(column)?),
    })
}

pub async fn write_contributed(
    tx: &mut Tx,
    definition: &EntityDefinition,
    record_id: Uuid,
    values: &Map<String, Value>,
) -> Result<(), AppError> {
    let fields: Vec<&SystemField> = writable_fields(definition)
        .into_iter()
        .filter(|field| values.contains_key(field.key.as_str()))
        .collect();
    let mut extensions: std::collections::BTreeMap<&str, Vec<&SystemField>> =
        std::collections::BTreeMap::new();
    for field in &fields {
        if let Some(extension) = &field.extension {
            extensions.entry(&extension.table).or_default().push(field);
        }
    }
    for (table, fields) in extensions {
        let extension = fields[0].extension.as_ref().expect("extension");
        let mut query = QueryBuilder::<MySql>::new(format!(
            "INSERT INTO `{table}` (`{}`",
            extension.record_column
        ));
        for field in &fields {
            query.push(format!(", `{}`", field.column));
        }
        query.push(") VALUES (").push_bind(record_id.to_string());
        for field in &fields {
            query.push(", ");
            push_extension_value(&mut query, &values[field.key.as_str()]);
        }
        query.push(") ON DUPLICATE KEY UPDATE ");
        let mut separated = query.separated(", ");
        for field in &fields {
            separated.push(format!("`{}` = VALUES(`{}`)", field.column, field.column));
        }
        query.build().execute(&mut **tx).await?;
    }
    let fields = fields
        .into_iter()
        .filter(|field| field.extension.is_none())
        .collect::<Vec<_>>();
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

fn push_extension_value(builder: &mut QueryBuilder<MySql>, value: &Value) {
    match value {
        Value::Null => {
            builder.push_bind(None::<String>);
        }
        Value::Bool(value) => {
            builder.push_bind(*value);
        }
        Value::String(value) => {
            builder.push_bind(value.clone());
        }
        value => {
            builder.push_bind(value.to_string());
        }
    }
}
