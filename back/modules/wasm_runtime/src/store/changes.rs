use chrono::NaiveDateTime;
use sqlx::Row;
use sqlx::types::Json;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;

pub(crate) struct NewChange<'a> {
    pub plugin_key: &'a str,
    pub event_id: String,
    pub event_type: &'a str,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub fields: Vec<String>,
    pub occurred_at: NaiveDateTime,
}

pub(crate) struct StoredChange {
    pub seq: u64,
    pub event_type: String,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub fields: Vec<String>,
    pub occurred_at: NaiveDateTime,
}

pub(crate) async fn insert(pool: &Db, change: NewChange<'_>) -> Result<(), AppError> {
    sqlx::query(
        "INSERT IGNORE INTO plugins_changes (plugin_key, event_id, event_type, record_type, record_id, fields, occurred_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(change.plugin_key)
    .bind(change.event_id)
    .bind(change.event_type)
    .bind(change.record_type)
    .bind(change.record_id)
    .bind(Json(change.fields))
    .bind(change.occurred_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn prune(pool: &Db, retention_hours: i64) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM plugins_changes WHERE created_at < UTC_TIMESTAMP(6) - INTERVAL ? HOUR LIMIT 10000",
    )
    .bind(retention_hours)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn oldest_seq(pool: &Db, plugin: &str) -> Result<Option<u64>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT MIN(seq) FROM plugins_changes WHERE plugin_key = ?")
            .bind(plugin)
            .fetch_one(pool)
            .await?,
    )
}

pub(crate) async fn after(
    pool: &Db,
    plugin: &str,
    since: u64,
    limit: u32,
) -> Result<Vec<StoredChange>, AppError> {
    let rows = sqlx::query(
        "SELECT seq, event_type, record_type, record_id, fields, occurred_at FROM plugins_changes \
         WHERE plugin_key = ? AND seq > ? ORDER BY seq LIMIT ?",
    )
    .bind(plugin)
    .bind(since)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let mut changes = Vec::with_capacity(rows.len());
    for row in &rows {
        let Json(fields): Json<Vec<String>> = row.try_get("fields")?;
        changes.push(StoredChange {
            seq: row.try_get("seq")?,
            event_type: row.try_get("event_type")?,
            record_type: row.try_get("record_type")?,
            record_id: row.try_get("record_id")?,
            fields,
            occurred_at: row.try_get("occurred_at")?,
        });
    }
    Ok(changes)
}
