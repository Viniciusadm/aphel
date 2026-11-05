use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::sources::CompanyData;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookupRow {
    pub(crate) id: Uuid,
    pub(crate) record_id: Uuid,
    pub(crate) cnpj: String,
    pub(crate) source: String,
    pub(crate) data: CompanyData,
    pub(crate) fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldSourceRow {
    pub(crate) field_key: String,
    pub(crate) data_key: String,
    pub(crate) source: String,
    pub(crate) source_reference: String,
    pub(crate) value_snapshot: Option<String>,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) applied_by: Option<Uuid>,
    pub(crate) applied_at: DateTime<Utc>,
}

fn uuid(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let raw: String = row.try_get(column)?;
    Uuid::parse_str(&raw).map_err(|error| AppError::Internal(error.to_string()))
}

fn utc(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    let value: NaiveDateTime = row.try_get(column)?;
    Ok(value.and_utc())
}

pub(crate) async fn insert_lookup(
    pool: &MySqlPool,
    record_type: &str,
    lookup: &LookupRow,
    fetched_by: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO enrichment_lookups (id, record_type, record_id, cnpj, source, data, fetched_by, fetched_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(lookup.id.to_string())
    .bind(record_type)
    .bind(lookup.record_id.to_string())
    .bind(&lookup.cnpj)
    .bind(&lookup.source)
    .bind(Json(serde_json::to_value(&lookup.data).map_err(|error| AppError::Internal(error.to_string()))?))
    .bind(fetched_by.to_string())
    .bind(lookup.fetched_at.naive_utc())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn find_lookup(
    pool: &MySqlPool,
    record_type: &str,
    id: Uuid,
) -> Result<Option<LookupRow>, AppError> {
    let row = sqlx::query(
        "SELECT id, record_id, cnpj, source, data, fetched_at FROM enrichment_lookups WHERE record_type = ? AND id = ?",
    )
    .bind(record_type)
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let Json(data): Json<Value> = row.try_get("data")?;
    Ok(Some(LookupRow {
        id: uuid(&row, "id")?,
        record_id: uuid(&row, "record_id")?,
        cnpj: row.try_get("cnpj")?,
        source: row.try_get("source")?,
        data: serde_json::from_value(data)
            .map_err(|error| AppError::Internal(error.to_string()))?,
        fetched_at: utc(&row, "fetched_at")?,
    }))
}

pub(crate) async fn delete_old_lookups(
    pool: &MySqlPool,
    before: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM enrichment_lookups WHERE fetched_at < ?")
        .bind(before.naive_utc())
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) struct NewFieldSource<'a> {
    pub(crate) record_type: &'a str,
    pub(crate) record_id: Uuid,
    pub(crate) field_key: &'a str,
    pub(crate) data_key: &'a str,
    pub(crate) source: &'a str,
    pub(crate) source_reference: &'a str,
    pub(crate) value_snapshot: &'a str,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) applied_by: Uuid,
    pub(crate) applied_at: DateTime<Utc>,
}

pub(crate) async fn upsert_field_source(
    tx: &mut MySqlTx,
    source: &NewFieldSource<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO enrichment_field_sources (id, record_type, record_id, field_key, data_key, source, source_reference, value_snapshot, fetched_at, applied_by, applied_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE data_key = VALUES(data_key), source = VALUES(source), source_reference = VALUES(source_reference), value_snapshot = VALUES(value_snapshot), fetched_at = VALUES(fetched_at), applied_by = VALUES(applied_by), applied_at = VALUES(applied_at)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(source.record_type)
    .bind(source.record_id.to_string())
    .bind(source.field_key)
    .bind(source.data_key)
    .bind(source.source)
    .bind(source.source_reference)
    .bind(source.value_snapshot)
    .bind(source.fetched_at.naive_utc())
    .bind(source.applied_by.to_string())
    .bind(source.applied_at.naive_utc())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn field_sources(
    pool: &MySqlPool,
    record_type: &str,
    record_id: Uuid,
) -> Result<Vec<FieldSourceRow>, AppError> {
    let rows = sqlx::query(
        "SELECT field_key, data_key, source, source_reference, value_snapshot, fetched_at, applied_by, applied_at FROM enrichment_field_sources WHERE record_type = ? AND record_id = ? ORDER BY field_key",
    )
    .bind(record_type)
    .bind(record_id.to_string())
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            let applied_by: Option<String> = row.try_get("applied_by")?;
            Ok(FieldSourceRow {
                field_key: row.try_get("field_key")?,
                data_key: row.try_get("data_key")?,
                source: row.try_get("source")?,
                source_reference: row.try_get("source_reference")?,
                value_snapshot: row.try_get("value_snapshot")?,
                fetched_at: utc(row, "fetched_at")?,
                applied_by: applied_by.and_then(|raw| Uuid::parse_str(&raw).ok()),
                applied_at: utc(row, "applied_at")?,
            })
        })
        .collect()
}

pub(crate) async fn counts(pool: &MySqlPool) -> Result<(i64, i64), AppError> {
    let lookups: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enrichment_lookups")
        .fetch_one(pool)
        .await?;
    let sources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enrichment_field_sources")
        .fetch_one(pool)
        .await?;
    Ok((lookups, sources))
}

pub(crate) async fn purge(tx: &mut MySqlTx) -> Result<(), AppError> {
    for sql in [
        "DELETE FROM enrichment_field_sources",
        "DELETE FROM enrichment_lookups",
    ] {
        sqlx::query(sql).execute(&mut **tx).await?;
    }
    Ok(())
}
