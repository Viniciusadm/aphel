use serde_json::Value;
use sqlx::Row;
use sqlx::types::Json;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;

pub(crate) struct QueryKey<'a> {
    pub plugin_key: &'a str,
    pub execution_id: &'a str,
    pub operation: &'a str,
    pub input_hash: &'a str,
}

pub(crate) struct StoredQuery {
    pub status: String,
    pub output: Option<Value>,
    pub authority_hash: String,
}

pub(crate) async fn start(
    pool: &Db,
    key: &QueryKey<'_>,
    authority_hash: &str,
) -> Result<bool, AppError> {
    Ok(sqlx::query("INSERT IGNORE INTO plugins_external_queries (plugin_key, execution_id, operation, input_hash, authority_hash) VALUES (?, ?, ?, ?, ?)")
        .bind(key.plugin_key)
        .bind(key.execution_id)
        .bind(key.operation)
        .bind(key.input_hash)
        .bind(authority_hash)
        .execute(pool)
        .await?
        .rows_affected()
        != 0)
}

pub(crate) async fn find(pool: &Db, key: &QueryKey<'_>) -> Result<StoredQuery, AppError> {
    let row = sqlx::query("SELECT status, output, authority_hash FROM plugins_external_queries WHERE plugin_key = ? AND execution_id = ? AND operation = ? AND input_hash = ?")
        .bind(key.plugin_key)
        .bind(key.execution_id)
        .bind(key.operation)
        .bind(key.input_hash)
        .fetch_one(pool)
        .await?;
    let output: Option<Json<Value>> = row.try_get("output")?;
    Ok(StoredQuery {
        status: row.try_get("status")?,
        output: output.map(|Json(value)| value),
        authority_hash: row.try_get("authority_hash")?,
    })
}

pub(crate) async fn complete(
    pool: &Db,
    key: &QueryKey<'_>,
    output: &Value,
) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_external_queries SET status = 'completed', output = ?, completed_at = UTC_TIMESTAMP(6) WHERE plugin_key = ? AND execution_id = ? AND operation = ? AND input_hash = ?")
        .bind(Json(output))
        .bind(key.plugin_key)
        .bind(key.execution_id)
        .bind(key.operation)
        .bind(key.input_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn fail(pool: &Db, key: &QueryKey<'_>) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_external_queries SET status = 'failed', completed_at = UTC_TIMESTAMP(6) WHERE plugin_key = ? AND execution_id = ? AND operation = ? AND input_hash = ?")
        .bind(key.plugin_key)
        .bind(key.execution_id)
        .bind(key.operation)
        .bind(key.input_hash)
        .execute(pool)
        .await?;
    Ok(())
}
