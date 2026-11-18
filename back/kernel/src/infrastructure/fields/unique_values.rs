use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::{EntityDefinition, custom_field_path};
use crate::application::shared::RecordRef;
use crate::database::transaction::Tx;
use crate::domain::fields::{FieldDefinition, FieldValueError};
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::query::compiler::quoted;

pub const UNIQUE_KEY_EXPRESSION: &str = "CASE WHEN v.value_text IS NOT NULL \
     THEN SHA2(CONCAT('t:', LOWER(v.value_text)), 256) \
     ELSE SHA2(CONCAT('n:', CAST(v.value_number AS CHAR)), 256) END";

const DUPLICATED_VALUES: &str = "duplicated_values";

pub fn reservable(field: &FieldDefinition) -> bool {
    field.is_unique && field.field_type.supports_unique()
}

fn not_unique(field: &FieldDefinition) -> AppError {
    AppError::invalid_fields(
        "invalid custom field values",
        vec![FieldViolation::new(
            custom_field_path(field.key.as_str()),
            FieldValueError::NotUnique.message(),
        )],
    )
}

async fn lock_field(tx: &mut Tx, field_id: Uuid) -> Result<(), AppError> {
    sqlx::query("SELECT id FROM field_definitions WHERE id = ? FOR UPDATE")
        .bind(field_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
    Ok(())
}

async fn is_live(
    tx: &mut Tx,
    definition: &EntityDefinition,
    record_id: &str,
) -> Result<bool, AppError> {
    let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
        "SELECT 1 FROM {} r WHERE r.id = ",
        quoted(&definition.table)
    ));
    builder.push_bind(record_id.to_owned());
    if let Some(deleted) = &definition.deleted_column {
        builder.push(format!(" AND r.{} IS NULL", quoted(deleted)));
    }
    builder.push(" FOR SHARE");
    let live: Option<i32> = builder
        .build_query_scalar()
        .fetch_optional(&mut **tx)
        .await?;
    Ok(live.is_some())
}

async fn holds_value(
    tx: &mut Tx,
    field_id: Uuid,
    record_type: &str,
    record_id: &str,
    value_key: &str,
) -> Result<bool, AppError> {
    let held: Option<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT 1 FROM custom_field_values v WHERE v.record_type = ? \
         AND v.record_id = ? AND v.field_id = ? AND {UNIQUE_KEY_EXPRESSION} = ? FOR SHARE"
    )))
    .bind(record_type)
    .bind(record_id)
    .bind(field_id.to_string())
    .bind(value_key)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(held.is_some())
}

pub async fn release(tx: &mut Tx, record: &RecordRef, field_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM custom_field_unique_values WHERE record_type = ? AND record_id = ? AND field_id = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .bind(field_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn release_record(tx: &mut Tx, record: &RecordRef) -> Result<(), AppError> {
    sqlx::query("DELETE FROM custom_field_unique_values WHERE record_type = ? AND record_id = ?")
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn reserve(
    tx: &mut Tx,
    definition: &EntityDefinition,
    record: &RecordRef,
    field: &FieldDefinition,
) -> Result<(), AppError> {
    if !reservable(field) {
        return Ok(());
    }
    lock_field(tx, field.id).await?;
    release(tx, record, field.id).await?;
    let value_key: Option<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT {UNIQUE_KEY_EXPRESSION} FROM custom_field_values v \
         WHERE v.record_type = ? AND v.record_id = ? AND v.field_id = ? \
         AND (v.value_text IS NOT NULL OR v.value_number IS NOT NULL)"
    )))
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .bind(field.id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(value_key) = value_key else {
        return Ok(());
    };
    let holder = sqlx::query(
        "SELECT record_type, record_id FROM custom_field_unique_values \
         WHERE field_id = ? AND value_key = ? FOR UPDATE",
    )
    .bind(field.id.to_string())
    .bind(&value_key)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(holder) = holder {
        let holder_type: String = holder.try_get("record_type")?;
        let holder_id: String = holder.try_get("record_id")?;
        if holder_type == record.record_type.as_str()
            && is_live(tx, definition, &holder_id).await?
            && holds_value(tx, field.id, &holder_type, &holder_id, &value_key).await?
        {
            return Err(not_unique(field));
        }
        sqlx::query("DELETE FROM custom_field_unique_values WHERE field_id = ? AND value_key = ?")
            .bind(field.id.to_string())
            .bind(&value_key)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query(
        "INSERT INTO custom_field_unique_values (field_id, value_key, record_type, record_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(field.id.to_string())
    .bind(&value_key)
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => not_unique(field),
        _ => AppError::from(error),
    })?;
    Ok(())
}

pub async fn reserve_record(
    tx: &mut Tx,
    definition: &EntityDefinition,
    record: &RecordRef,
) -> Result<(), AppError> {
    let fields = crate::infrastructure::fields::mysql_definitions::unique_fields(
        tx,
        record.record_type.as_str(),
    )
    .await?;
    for field in fields {
        reserve(tx, definition, record, &field).await?;
    }
    Ok(())
}

pub async fn rebuild(
    tx: &mut Tx,
    definition: &EntityDefinition,
    field: &FieldDefinition,
) -> Result<(), AppError> {
    lock_field(tx, field.id).await?;
    sqlx::query("DELETE FROM custom_field_unique_values WHERE field_id = ?")
        .bind(field.id.to_string())
        .execute(&mut **tx)
        .await?;
    if !reservable(field) {
        return Ok(());
    }
    let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
        "INSERT INTO custom_field_unique_values (field_id, value_key, record_type, record_id) \
         SELECT v.field_id, {UNIQUE_KEY_EXPRESSION}, v.record_type, v.record_id \
         FROM custom_field_values v WHERE v.field_id = "
    ));
    builder
        .push_bind(field.id.to_string())
        .push(" AND (v.value_text IS NOT NULL OR v.value_number IS NOT NULL)");
    if let Some(deleted) = &definition.deleted_column {
        builder.push(format!(
            " AND NOT EXISTS (SELECT 1 FROM {} r WHERE r.id = v.record_id AND r.{} IS NOT NULL)",
            quoted(&definition.table),
            quoted(deleted)
        ));
    }
    builder
        .build()
        .execute(&mut **tx)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(database) if database.is_unique_violation() => {
                AppError::conflict_code(
                    DUPLICATED_VALUES,
                    format!(
                        "existing records share values of {}, so it cannot be unique",
                        field.key.as_str()
                    ),
                )
            }
            _ => AppError::from(error),
        })?;
    Ok(())
}

pub async fn duplicated_records(
    pool: &sqlx::MySqlPool,
    definition: &EntityDefinition,
    field: &FieldDefinition,
    limit: u32,
) -> Result<Vec<Uuid>, AppError> {
    if !field.field_type.supports_unique() {
        return Ok(Vec::new());
    }
    let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
        "SELECT v.record_id FROM custom_field_values v JOIN (SELECT {UNIQUE_KEY_EXPRESSION} AS value_key \
         FROM custom_field_values v WHERE v.field_id = "
    ));
    builder.push_bind(field.id.to_string());
    push_live(&mut builder, definition);
    builder.push(format!(
        " AND (v.value_text IS NOT NULL OR v.value_number IS NOT NULL) GROUP BY value_key HAVING COUNT(*) > 1) d \
         ON d.value_key = {UNIQUE_KEY_EXPRESSION} WHERE v.field_id = "
    ));
    builder.push_bind(field.id.to_string());
    push_live(&mut builder, definition);
    builder
        .push(" ORDER BY v.record_id LIMIT ")
        .push_bind(limit);
    let rows: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    Ok(rows
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect())
}

fn push_live(builder: &mut QueryBuilder<MySql>, definition: &EntityDefinition) {
    if let Some(deleted) = &definition.deleted_column {
        builder.push(format!(
            " AND NOT EXISTS (SELECT 1 FROM {} r WHERE r.id = v.record_id AND r.{} IS NOT NULL)",
            quoted(&definition.table),
            quoted(deleted)
        ));
    }
}
