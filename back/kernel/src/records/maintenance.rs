use crate::database::Db;
use crate::{
    application::{
        fields::{EntityDefinition, RecordTypeRegistry, is_sql_identifier},
        shared::RecordRef,
    },
    database::transaction::Tx,
    errors::AppError,
    infrastructure::query::compiler::{push_partition, quoted},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder, Row};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAssignment {
    pub column: String,
    pub value: Option<String>,
}
pub enum StoredTextField<'a> {
    Column(&'a str),
    Custom(Uuid),
}
pub enum TextComparison {
    EqualFolded(String),
    ContainsFolded(Vec<String>),
    DigitsContain(Vec<String>),
}
fn pattern(value: &str) -> String {
    let mut result = String::from("%");
    for character in value.chars() {
        if matches!(character, '%' | '_' | '\\') {
            result.push('\\');
        }
        result.push(character);
    }
    result.push('%');
    result
}
fn predicate(builder: &mut QueryBuilder<MySql>, expression: &str, comparison: &TextComparison) {
    match comparison {
        TextComparison::EqualFolded(value) => {
            builder
                .push(format!("LOWER({expression}) = "))
                .push_bind(value.to_lowercase());
        }
        TextComparison::ContainsFolded(values) | TextComparison::DigitsContain(values) => {
            builder.push("(FALSE");
            for value in values {
                builder.push(" OR ");
                if matches!(comparison, TextComparison::DigitsContain(_)) {
                    builder
                        .push(format!("REGEXP_REPLACE({expression}, '[^0-9]', '') LIKE "))
                        .push_bind(pattern(value));
                } else {
                    builder
                        .push(format!("LOWER({expression}) LIKE "))
                        .push_bind(pattern(&value.to_lowercase()));
                }
            }
            builder.push(")");
        }
    }
}
pub async fn matching_entity_text_ids(
    pool: &Db,
    definition: &EntityDefinition,
    field: StoredTextField<'_>,
    comparison: &TextComparison,
    limit: u32,
) -> Result<Vec<Uuid>, AppError> {
    let mut builder = match field {
        StoredTextField::Column(column) => {
            if !is_sql_identifier(column)
                || !definition
                    .system_fields
                    .iter()
                    .any(|field| field.column == column)
            {
                return Err(AppError::Configuration(
                    "field column is not declared by the entity".to_owned(),
                ));
            }
            let mut builder = QueryBuilder::<MySql>::new(format!(
                "SELECT r.id FROM {} r WHERE ",
                quoted(&definition.table)
            ));
            predicate(&mut builder, &format!("r.{}", quoted(column)), comparison);
            push_partition(&mut builder, definition, "r");
            builder
        }
        StoredTextField::Custom(id) => {
            let mut builder = QueryBuilder::<MySql>::new(
                "SELECT v.record_id AS id FROM custom_field_values v JOIN field_definitions f ON f.id = v.field_id AND f.record_type = v.record_type WHERE v.record_type = ",
            );
            builder
                .push_bind(definition.record_type.as_str().to_owned())
                .push(" AND v.field_id = ")
                .push_bind(id.to_string())
                .push(" AND ");
            predicate(
                &mut builder,
                "COALESCE(v.value_text, v.value_long_text)",
                comparison,
            );
            builder
        }
    };
    builder.push(" ORDER BY id LIMIT ").push_bind(limit);
    let ids: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    ids.into_iter()
        .map(|id| {
            Uuid::parse_str(&id)
                .map_err(|_| AppError::Internal("invalid stored record id".to_owned()))
        })
        .collect()
}
pub async fn inspect_entity(
    pool: &Db,
    registry: &Arc<RecordTypeRegistry>,
    record: &RecordRef,
) -> Result<Option<Value>, AppError> {
    let definition = registry.require(record.record_type.as_str())?;
    let mut tx = Tx::from(pool.begin().await?);
    let Some(mut snapshot) = super::snapshot::capture(&mut tx, pool, registry, record).await?
    else {
        return Ok(None);
    };
    let archived = definition
        .archived_column
        .as_deref()
        .map(quoted)
        .unwrap_or_else(|| "NULL".into());
    let deleted = definition
        .deleted_column
        .as_deref()
        .map(quoted)
        .unwrap_or_else(|| "NULL".into());
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "SELECT {archived} AS archived_at, {deleted} AS deleted_at FROM {} r WHERE r.id = ",
        quoted(&definition.table)
    ));
    builder.push_bind(record.record_id.to_string());
    push_partition(&mut builder, definition, "r");
    if let Some(row) = builder.build().fetch_optional(&mut *tx).await? {
        let archived: Option<NaiveDateTime> = row.try_get("archived_at")?;
        let deleted: Option<NaiveDateTime> = row.try_get("deleted_at")?;
        snapshot["archived"] = json!(archived.is_some());
        snapshot["deleted"] = json!(deleted.is_some());
        if let Some(values) = snapshot.get_mut("values").and_then(Value::as_object_mut) {
            if let Some(column) = &definition.archived_column {
                values.insert(column.clone(), json!(archived.map(|value| value.and_utc())));
            }
            if let Some(column) = &definition.deleted_column {
                values.insert(column.clone(), json!(deleted.map(|value| value.and_utc())));
            }
        }
    }
    Ok(Some(snapshot))
}
pub async fn existing_entity_ids(
    pool: &Db,
    definition: &EntityDefinition,
    ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "SELECT r.id FROM {} r WHERE r.id IN (",
        quoted(&definition.table)
    ));
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
    push_partition(&mut builder, definition, "r");
    let values: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    values
        .into_iter()
        .map(|id| {
            Uuid::parse_str(&id)
                .map_err(|_| AppError::Internal("invalid stored record id".to_owned()))
        })
        .collect()
}
pub async fn update_entity_text(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
    assignments: &[TextAssignment],
) -> Result<bool, AppError> {
    if assignments.is_empty() {
        return Ok(false);
    }
    for assignment in assignments {
        if !is_sql_identifier(&assignment.column)
            || !definition.system_fields.iter().any(|field| {
                field.writable && field.stored_column() == Some(assignment.column.as_str())
            })
        {
            return Err(AppError::Configuration(
                "stored text column is not declared by the entity".to_owned(),
            ));
        }
    }
    let mut builder =
        QueryBuilder::<MySql>::new(format!("UPDATE {} r SET ", quoted(&definition.table)));
    let mut separated = builder.separated(", ");
    for assignment in assignments {
        separated
            .push(format!("{} = ", quoted(&assignment.column)))
            .push_bind_unseparated(assignment.value.clone());
    }
    builder.push(" WHERE r.id = ").push_bind(id.to_string());
    push_partition(&mut builder, definition, "r");
    Ok(builder.build().execute(&mut **tx).await?.rows_affected() > 0)
}
pub async fn mark_entity_deleted(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
    at: DateTime<Utc>,
) -> Result<bool, AppError> {
    let Some(column) = &definition.deleted_column else {
        return Ok(false);
    };
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "UPDATE {} r SET {} = COALESCE({}, ",
        quoted(&definition.table),
        quoted(column),
        quoted(column)
    ));
    builder
        .push_bind(at.naive_utc())
        .push(") WHERE r.id = ")
        .push_bind(id.to_string());
    push_partition(&mut builder, definition, "r");
    Ok(builder.build().execute(&mut **tx).await?.rows_affected() > 0)
}
pub async fn archive_entity(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
    at: DateTime<Utc>,
) -> Result<bool, AppError> {
    let (Some(deleted), Some(archived)) = (&definition.deleted_column, &definition.archived_column)
    else {
        return Ok(false);
    };
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "UPDATE {} r SET {} = ",
        quoted(&definition.table),
        quoted(archived)
    ));
    builder
        .push_bind(at.naive_utc())
        .push(" WHERE r.id = ")
        .push_bind(id.to_string())
        .push(format!(
            " AND {} IS NULL AND {} IS NULL",
            quoted(deleted),
            quoted(archived)
        ));
    push_partition(&mut builder, definition, "r");
    Ok(builder.build().execute(&mut **tx).await?.rows_affected() > 0)
}
pub async fn purge_entity(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
) -> Result<bool, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "DELETE r FROM {} r WHERE r.id = ",
        quoted(&definition.table)
    ));
    builder.push_bind(id.to_string());
    if let Some(column) = &definition.deleted_column {
        builder.push(format!(" AND r.{} IS NOT NULL", quoted(column)));
    }
    push_partition(&mut builder, definition, "r");
    Ok(builder.build().execute(&mut **tx).await?.rows_affected() > 0)
}

pub async fn lock_entity_version(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    let column = definition
        .system_field_by_key("updated_at")
        .and_then(|field| field.stored_column())
        .ok_or_else(|| AppError::Configuration("the entity has no stored record version".into()))?;
    let mut query = QueryBuilder::<MySql>::new(format!(
        "SELECT r.{} FROM {} r WHERE r.id = ",
        quoted(column),
        quoted(&definition.table)
    ));
    query.push_bind(id.to_string());
    push_partition(&mut query, definition, "r");
    if let Some(column) = &definition.deleted_column {
        query.push(format!(" AND r.{} IS NULL", quoted(column)));
    }
    query.push(" FOR UPDATE");
    let version: Option<NaiveDateTime> =
        query.build_query_scalar().fetch_optional(&mut **tx).await?;
    Ok(version.map(|value| value.and_utc()))
}
