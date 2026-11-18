use sqlx::Row;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;

pub(crate) async fn holders(pool: &Db) -> Result<Vec<(String, String)>, AppError> {
    let rows = sqlx::query(
        "SELECT c.module_key, p.name FROM plugins_module_conflicts c \
         JOIN plugins_installed p ON p.plugin_key = c.plugin_key \
         ORDER BY c.created_at",
    )
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| Ok((row.try_get("module_key")?, row.try_get("name")?)))
        .collect()
}

pub(crate) async fn held_by(pool: &Db, plugin_key: &str) -> Result<Vec<(String, bool)>, AppError> {
    let rows = sqlx::query(
        "SELECT module_key, was_enabled FROM plugins_module_conflicts WHERE plugin_key = ?",
    )
    .bind(plugin_key)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("module_key")?,
                row.try_get::<bool, _>("was_enabled")?,
            ))
        })
        .collect()
}

pub(crate) async fn enabled_before_other_holders(
    pool: &Db,
    module: &str,
    plugin_key: &str,
) -> Result<Option<bool>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT MAX(was_enabled) FROM plugins_module_conflicts WHERE module_key = ? AND plugin_key <> ?",
    )
    .bind(module)
    .bind(plugin_key)
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn hold(
    pool: &Db,
    plugin_key: &str,
    module: &str,
    was_enabled: bool,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_module_conflicts (plugin_key, module_key, was_enabled) VALUES (?, ?, ?) \
         ON DUPLICATE KEY UPDATE was_enabled = VALUES(was_enabled)",
    )
    .bind(plugin_key)
    .bind(module)
    .bind(was_enabled)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn release(
    pool: &Db,
    plugin_key: &str,
    module: &str,
) -> Result<(Option<bool>, i64), AppError> {
    let was_enabled: Option<bool> = sqlx::query_scalar(
        "SELECT was_enabled FROM plugins_module_conflicts WHERE plugin_key = ? AND module_key = ?",
    )
    .bind(plugin_key)
    .bind(module)
    .fetch_optional(pool)
    .await?;
    sqlx::query("DELETE FROM plugins_module_conflicts WHERE plugin_key = ? AND module_key = ?")
        .bind(plugin_key)
        .bind(module)
        .execute(pool)
        .await?;
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM plugins_module_conflicts WHERE module_key = ?")
            .bind(module)
            .fetch_one(pool)
            .await?;
    Ok((was_enabled, remaining))
}
