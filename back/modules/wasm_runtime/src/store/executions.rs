use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use sqlx::types::Json;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;

pub(crate) struct LockedExecution {
    pub plugin_key: String,
    pub input: Value,
    pub execution: Value,
    pub status: String,
}

pub(crate) async fn record_once<E: Serialize + Sync>(
    tx: &mut Tx,
    id: &str,
    plugin_key: &str,
    source_key: &str,
    input: &Value,
    execution: &E,
) -> Result<String, AppError> {
    sqlx::query("INSERT IGNORE INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, ?, ?)")
        .bind(id)
        .bind(plugin_key)
        .bind(source_key)
        .bind(Json(input))
        .bind(Json(execution))
        .execute(&mut **tx)
        .await?;
    Ok(sqlx::query_scalar(
        "SELECT id FROM plugins_executions WHERE plugin_key = ? AND source_key = ?",
    )
    .bind(plugin_key)
    .bind(source_key)
    .fetch_one(&mut **tx)
    .await?)
}

pub(crate) async fn insert<E: Serialize + Sync>(
    tx: &mut Tx,
    id: &str,
    plugin_key: &str,
    source_key: &str,
    input: &Value,
    execution: &E,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, ?, ?)")
        .bind(id)
        .bind(plugin_key)
        .bind(source_key)
        .bind(Json(input))
        .bind(Json(execution))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn record_action_once<E: Serialize + Sync>(
    pool: &Db,
    id: &str,
    plugin_key: &str,
    source_key: &str,
    execution: &E,
) -> Result<Value, AppError> {
    sqlx::query("INSERT IGNORE INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, JSON_OBJECT(), ?)")
        .bind(id)
        .bind(plugin_key)
        .bind(source_key)
        .bind(Json(execution))
        .execute(pool)
        .await?;
    let Json(stored): Json<Value> = sqlx::query_scalar(
        "SELECT execution FROM plugins_executions WHERE plugin_key = ? AND source_key = ?",
    )
    .bind(plugin_key)
    .bind(source_key)
    .fetch_one(pool)
    .await?;
    Ok(stored)
}

pub(crate) async fn lock(tx: &mut Tx, id: &str) -> Result<LockedExecution, AppError> {
    let row = sqlx::query(
        "SELECT plugin_key, input, execution, status FROM plugins_executions WHERE id = ? FOR UPDATE",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    let Json(input): Json<Value> = row.try_get("input")?;
    let Json(execution): Json<Value> = row.try_get("execution")?;
    Ok(LockedExecution {
        plugin_key: row.try_get("plugin_key")?,
        input,
        execution,
        status: row.try_get("status")?,
    })
}

pub(crate) async fn complete(tx: &mut Tx, id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_executions SET status = 'completed', completed_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn set_last_error(tx: &mut Tx, id: &str, message: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_executions SET last_error = ? WHERE id = ?")
        .bind(message)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn insert_once<E: Serialize + Sync>(
    tx: &mut Tx,
    id: &str,
    plugin_key: &str,
    source_key: &str,
    input: &Value,
    execution: &E,
) -> Result<(), AppError> {
    sqlx::query("INSERT IGNORE INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, ?, ?)")
        .bind(id)
        .bind(plugin_key)
        .bind(source_key)
        .bind(Json(input))
        .bind(Json(execution))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn lock_by_source(
    tx: &mut Tx,
    plugin_key: &str,
    source_key: &str,
) -> Result<(String, Value, String), AppError> {
    let row = sqlx::query("SELECT id, execution, status FROM plugins_executions WHERE plugin_key = ? AND source_key = ? FOR UPDATE")
        .bind(plugin_key)
        .bind(source_key)
        .fetch_one(&mut **tx)
        .await?;
    let Json(execution): Json<Value> = row.try_get("execution")?;
    Ok((row.try_get("id")?, execution, row.try_get("status")?))
}
