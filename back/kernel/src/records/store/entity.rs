use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::application::fields::{EntityDefinition, EntityStorage, WriteMode};
use crate::database::Db;
use crate::database::transaction::Tx;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::errors::AppError;
use crate::infrastructure::query::compiler::push_record_scope;

#[derive(Clone, Copy)]
pub(crate) enum RowLock {
    Share,
    Update,
}

pub(crate) async fn live_row_in_scope(
    tx: &mut Tx,
    scope: &AccessScope,
    definition: &EntityDefinition,
    visibility: &RecordScopeFilter,
    id: Uuid,
    lock: RowLock,
) -> Result<bool, AppError> {
    let mut query = QueryBuilder::<MySql>::new(format!(
        "SELECT r.id FROM `{}` r WHERE r.id = ",
        definition.table
    ));
    query.push_bind(id.to_string());
    if let Some(column) = &definition.deleted_column {
        query.push(format!(" AND r.`{column}` IS NULL"));
    }
    push_record_scope(&mut query, scope, definition, visibility);
    query.push(match lock {
        RowLock::Share => " FOR SHARE",
        RowLock::Update => " FOR UPDATE",
    });
    Ok(query
        .build_query_scalar::<String>()
        .fetch_optional(&mut **tx)
        .await?
        .is_some())
}

pub(crate) async fn write_row(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
    values: &Map<String, Value>,
    mode: WriteMode,
    creator: Option<String>,
) -> Result<(), AppError> {
    let fields = definition
        .system_fields
        .iter()
        .filter(|field| {
            field.writable
                && !field.is_computed()
                && field.contributed_by.is_none()
                && values.contains_key(field.key.as_str())
        })
        .collect::<Vec<_>>();
    let mut query = QueryBuilder::<MySql>::new(if mode == WriteMode::Create {
        format!("INSERT INTO `{}` (`id`", definition.table)
    } else {
        format!("UPDATE `{}` SET ", definition.table)
    });
    if mode == WriteMode::Create {
        for field in &fields {
            query.push(format!(", `{}`", field.column));
        }
        if definition.storage == EntityStorage::Managed {
            query.push(", `created_by`");
        }
        query.push(") VALUES (").push_bind(id.to_string());
        for field in &fields {
            query.push(", ");
            push_value(&mut query, &values[field.key.as_str()]);
        }
        if definition.storage == EntityStorage::Managed {
            query.push(", ").push_bind(creator);
        }
        query.push(")");
    } else {
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                query.push(", ");
            }
            query.push(format!("`{}` = ", field.column));
            push_value(&mut query, &values[field.key.as_str()]);
        }
        if definition.storage == EntityStorage::Managed
            || definition.system_field_by_key("updated_at").is_some()
        {
            if !fields.is_empty() {
                query.push(", ");
            }
            query.push("updated_at = UTC_TIMESTAMP(6)");
        } else if fields.is_empty() {
            query.push("id = id");
        }

        query.push(" WHERE id = ").push_bind(id.to_string());
    }
    query.build().execute(&mut **tx).await.map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            AppError::conflict_code(
                "entity_unique_conflict",
                "record violates a unique entity index",
            )
        } else {
            AppError::from(error)
        }
    })?;
    Ok(())
}

fn push_value(query: &mut QueryBuilder<MySql>, value: &Value) {
    match value {
        Value::Null => {
            query.push_bind(None::<String>);
        }
        Value::Bool(value) => {
            query.push_bind(*value);
        }
        Value::Number(value) => {
            query.push_bind(value.to_string());
        }
        Value::String(value) => {
            query.push_bind(value.clone());
        }
        value => {
            query.push_bind(value.to_string());
        }
    }
}

pub(crate) async fn delete_row(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
) -> Result<(), AppError> {
    let mut query = QueryBuilder::<MySql>::new(match &definition.deleted_column {
        Some(column) => format!(
            "UPDATE `{}` SET `{column}` = UTC_TIMESTAMP(6) WHERE id = ",
            definition.table
        ),
        None => format!("DELETE FROM `{}` WHERE id = ", definition.table),
    });
    query
        .push_bind(id.to_string())
        .build()
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn lock_archived_at(
    tx: &mut Tx,
    table: &str,
    column: &str,
    id: Uuid,
) -> Result<Option<NaiveDateTime>, AppError> {
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT `{column}` FROM `{table}` WHERE id = ? FOR UPDATE"
    )))
    .bind(id.to_string())
    .fetch_one(&mut **tx)
    .await?)
}

pub(crate) async fn set_archived(
    tx: &mut Tx,
    table: &str,
    column: &str,
    id: Uuid,
    archived: bool,
) -> Result<(), AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE `{table}` SET `{column}` = {}, updated_at = UTC_TIMESTAMP(6) WHERE id = ?",
        if archived { "UTC_TIMESTAMP(6)" } else { "NULL" }
    )))
    .bind(id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn archived_at(
    pool: &Db,
    table: &str,
    column: &str,
    id: Uuid,
) -> Result<Option<NaiveDateTime>, AppError> {
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT `{column}` FROM `{table}` WHERE id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?
    .flatten())
}
