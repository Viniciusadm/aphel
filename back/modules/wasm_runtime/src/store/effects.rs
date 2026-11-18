use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use sqlx::types::Json;
use uuid::Uuid;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;

pub(crate) struct StoredEffect {
    pub plugin_key: String,
    pub command: Value,
    pub authority: Value,
    pub execution: Value,
    pub status: String,
}

pub(crate) async fn locked_effect_count(
    tx: &mut Tx,
    plugin_key: &str,
    execution_id: &str,
) -> Result<u32, AppError> {
    sqlx::query(
        "INSERT IGNORE INTO plugins_effect_limits (plugin_key, execution_id) VALUES (?, ?)",
    )
    .bind(plugin_key)
    .bind(execution_id)
    .execute(&mut **tx)
    .await?;
    Ok(sqlx::query_scalar("SELECT effect_count FROM plugins_effect_limits WHERE plugin_key = ? AND execution_id = ? FOR UPDATE")
        .bind(plugin_key)
        .bind(execution_id)
        .fetch_one(&mut **tx)
        .await?)
}

pub(crate) struct NewEffect<'a, C, A, E> {
    pub id: Uuid,
    pub plugin_key: &'a str,
    pub execution_id: &'a str,
    pub effect_key: &'a str,
    pub command: &'a C,
    pub authority: &'a A,
    pub execution: &'a E,
}

pub(crate) async fn insert_once<C, A, E>(
    tx: &mut Tx,
    effect: NewEffect<'_, C, A, E>,
) -> Result<bool, AppError>
where
    C: Serialize + Sync,
    A: Serialize + Sync,
    E: Serialize + Sync,
{
    Ok(sqlx::query("INSERT IGNORE INTO plugins_effects (id, plugin_key, execution_id, effect_key, command, authority, execution) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(effect.id.to_string())
        .bind(effect.plugin_key)
        .bind(effect.execution_id)
        .bind(effect.effect_key)
        .bind(Json(effect.command))
        .bind(Json(effect.authority))
        .bind(Json(effect.execution))
        .execute(&mut **tx)
        .await?
        .rows_affected()
        != 0)
}

pub(crate) async fn count_effect(
    tx: &mut Tx,
    plugin_key: &str,
    execution_id: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_effect_limits SET effect_count = effect_count + 1 WHERE plugin_key = ? AND execution_id = ?")
        .bind(plugin_key)
        .bind(execution_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn lock_by_key(
    tx: &mut Tx,
    plugin_key: &str,
    execution_id: &str,
    effect_key: &str,
) -> Result<(String, Value), AppError> {
    let row = sqlx::query("SELECT id, command FROM plugins_effects WHERE plugin_key = ? AND execution_id = ? AND effect_key = ? FOR UPDATE")
        .bind(plugin_key)
        .bind(execution_id)
        .bind(effect_key)
        .fetch_one(&mut **tx)
        .await?;
    let Json(command): Json<Value> = row.try_get("command")?;
    Ok((row.try_get("id")?, command))
}

pub(crate) async fn find(pool: &Db, id: Uuid) -> Result<StoredEffect, AppError> {
    let row = sqlx::query("SELECT plugin_key, command, authority, execution, status FROM plugins_effects WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(pool)
        .await?;
    let Json(command): Json<Value> = row.try_get("command")?;
    let Json(authority): Json<Value> = row.try_get("authority")?;
    let Json(execution): Json<Value> = row.try_get("execution")?;
    Ok(StoredEffect {
        plugin_key: row.try_get("plugin_key")?,
        command,
        authority,
        execution,
        status: row.try_get("status")?,
    })
}

pub(crate) async fn set_last_error(pool: &Db, id: Uuid, message: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_effects SET last_error = ? WHERE id = ?")
        .bind(message)
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn complete(pool: &Db, id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_effects SET status = 'completed', completed_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn result(pool: &Db, plugin: &str, id: &str) -> Result<Option<Value>, AppError> {
    let value: Option<Option<Json<Value>>> =
        sqlx::query_scalar("SELECT result FROM plugins_effects WHERE id = ? AND plugin_key = ?")
            .bind(id)
            .bind(plugin)
            .fetch_optional(pool)
            .await?;
    Ok(value.flatten().map(|Json(value)| value))
}

pub(crate) async fn claim_sending(pool: &Db, id: Uuid) -> Result<bool, AppError> {
    Ok(sqlx::query(
        "UPDATE plugins_effects SET status = 'sending' WHERE id = ? AND status = 'pending'",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

pub(crate) async fn complete_with_result(
    tx: &mut Tx,
    id: Uuid,
    result: &Value,
) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_effects SET result = ?, status = 'completed', completed_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?")
        .bind(Json(result))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}
