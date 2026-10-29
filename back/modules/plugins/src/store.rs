use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};

use crate::manifest::PluginManifest;
use crate::package::PluginPackage;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper::instant;

#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub key: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub manifest: PluginManifest,
    pub checksum: String,
    pub enabled: bool,
    pub settings: Map<String, Value>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InstalledPlugin {
    pub fn effective_settings(&self) -> Map<String, Value> {
        let mut settings = self.manifest.setting_defaults();
        settings.extend(self.settings.clone());
        settings
    }
}

const SELECT: &str = "SELECT plugin_key, name, version, description, manifest, checksum, enabled, \
     settings, installed_at, updated_at FROM plugins_installed";

fn from_row(row: &MySqlRow) -> Result<InstalledPlugin, AppError> {
    let Json(manifest): Json<PluginManifest> = row.try_get("manifest")?;
    let Json(settings): Json<Map<String, Value>> = row.try_get("settings")?;
    Ok(InstalledPlugin {
        key: row.try_get("plugin_key")?,
        name: row.try_get("name")?,
        version: row.try_get("version")?,
        description: row.try_get("description")?,
        manifest,
        checksum: row.try_get("checksum")?,
        enabled: row.try_get("enabled")?,
        settings,
        installed_at: instant(row, "installed_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

pub async fn list(pool: &MySqlPool) -> Result<Vec<InstalledPlugin>, AppError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{SELECT} ORDER BY name")))
        .fetch_all(pool)
        .await?;
    rows.iter().map(from_row).collect()
}

pub async fn find(pool: &MySqlPool, key: &str) -> Result<Option<InstalledPlugin>, AppError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE plugin_key = ?"
    )))
    .bind(key)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(from_row).transpose()
}

pub async fn version(pool: &MySqlPool) -> Result<u64, AppError> {
    let version: Option<u64> = sqlx::query_scalar("SELECT version FROM plugins_state WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(version.unwrap_or_default())
}

pub async fn bump(tx: &mut MySqlTx) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_state (id, version) VALUES (1, 1) \
         ON DUPLICATE KEY UPDATE version = version + 1",
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn save(
    tx: &mut MySqlTx,
    package: &PluginPackage,
    installed_by: Option<&UserId>,
) -> Result<(), AppError> {
    let manifest = &package.manifest;
    sqlx::query(
        "INSERT INTO plugins_installed (plugin_key, name, version, description, manifest, \
         checksum, enabled, settings, installed_by) VALUES (?, ?, ?, ?, ?, ?, TRUE, JSON_OBJECT(), ?) \
         ON DUPLICATE KEY UPDATE name = VALUES(name), version = VALUES(version), \
         description = VALUES(description), manifest = VALUES(manifest), \
         checksum = VALUES(checksum), updated_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(&manifest.key)
    .bind(&manifest.name)
    .bind(&manifest.version)
    .bind(&manifest.description)
    .bind(Json(manifest))
    .bind(&package.checksum)
    .bind(installed_by.map(UserId::as_str))
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM plugins_files WHERE plugin_key = ?")
        .bind(&manifest.key)
        .execute(&mut **tx)
        .await?;
    for file in &package.files {
        sqlx::query(
            "INSERT INTO plugins_files (plugin_key, path, content_type, content) VALUES (?, ?, ?, ?)",
        )
        .bind(&manifest.key)
        .bind(&file.path)
        .bind(&file.content_type)
        .bind(&file.content)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn file(
    pool: &MySqlPool,
    key: &str,
    path: &str,
) -> Result<Option<(String, Vec<u8>)>, AppError> {
    let row = sqlx::query(
        "SELECT f.content_type, f.content FROM plugins_files f \
         JOIN plugins_installed p ON p.plugin_key = f.plugin_key \
         WHERE f.plugin_key = ? AND f.path = ? AND p.enabled = TRUE",
    )
    .bind(key)
    .bind(path)
    .fetch_optional(pool)
    .await?;
    row.map(|row| -> Result<(String, Vec<u8>), AppError> {
        Ok((row.try_get("content_type")?, row.try_get("content")?))
    })
    .transpose()
}

pub async fn backend(pool: &MySqlPool, key: &str) -> Result<Option<Vec<u8>>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT content FROM plugins_files WHERE plugin_key = ? AND path = 'backend.wasm'",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?)
}

pub async fn set_enabled(tx: &mut MySqlTx, key: &str, enabled: bool) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE plugins_installed SET enabled = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE plugin_key = ?",
    )
    .bind(enabled)
    .bind(key)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn set_settings(
    tx: &mut MySqlTx,
    key: &str,
    settings: &Map<String, Value>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE plugins_installed SET settings = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE plugin_key = ?",
    )
    .bind(Json(settings))
    .bind(key)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn delete(tx: &mut MySqlTx, key: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM plugins_installed WHERE plugin_key = ?")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn kv_get(
    pool: &MySqlPool,
    key: &str,
    collection: &str,
    item: &str,
) -> Result<Option<Value>, AppError> {
    let value: Option<Json<Value>> = sqlx::query_scalar(
        "SELECT value FROM plugins_data WHERE plugin_key = ? AND collection = ? AND item_id = ?",
    )
    .bind(key)
    .bind(collection)
    .bind(item)
    .fetch_optional(pool)
    .await?;
    Ok(value.map(|Json(value)| value))
}

pub async fn kv_set(
    pool: &MySqlPool,
    key: &str,
    collection: &str,
    item: &str,
    value: &Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_data (plugin_key, collection, item_id, value) VALUES (?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE value = VALUES(value)",
    )
    .bind(key)
    .bind(collection)
    .bind(item)
    .bind(Json(value))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn kv_delete(
    pool: &MySqlPool,
    key: &str,
    collection: &str,
    item: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM plugins_data WHERE plugin_key = ? AND collection = ? AND item_id = ?")
        .bind(key)
        .bind(collection)
        .bind(item)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn kv_list(
    pool: &MySqlPool,
    key: &str,
    collection: &str,
    limit: u32,
) -> Result<Vec<(String, Value)>, AppError> {
    let rows = sqlx::query(
        "SELECT item_id, value FROM plugins_data WHERE plugin_key = ? AND collection = ? \
         ORDER BY item_id LIMIT ?",
    )
    .bind(key)
    .bind(collection)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            let Json(value): Json<Value> = row.try_get("value")?;
            Ok((row.try_get("item_id")?, value))
        })
        .collect()
}
