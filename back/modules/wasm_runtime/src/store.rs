use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};

use crate::manifest::PluginManifest;
use crate::package::PluginPackage;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::repositories::mapper::instant;

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
    pub installed_by: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pack_key: Option<String>,
}

impl InstalledPlugin {
    pub fn effective_settings(&self) -> Map<String, Value> {
        let mut settings = self.manifest.setting_defaults();
        settings.extend(self.settings.clone());
        for setting in self.manifest.secret_settings() {
            settings.remove(&setting.key);
        }
        settings
    }
}

const SELECT: &str = "SELECT plugin_key, name, version, description, manifest, checksum, enabled, \
     settings, installed_by, installed_at, updated_at, pack_key FROM plugins_installed";

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
        installed_by: row.try_get("installed_by")?,
        installed_at: instant(row, "installed_at")?,
        updated_at: instant(row, "updated_at")?,
        pack_key: row.try_get("pack_key")?,
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

pub async fn bump(tx: &mut Tx) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_state (id, version) VALUES (1, 1) \
         ON DUPLICATE KEY UPDATE version = version + 1",
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn default_installer(pool: &MySqlPool) -> Result<UserId, AppError> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN roles r ON r.id = u.role_id \
         WHERE u.status = 'active' AND r.`key` = 'admin' ORDER BY u.joined_at, u.id LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    let id = id.ok_or_else(|| {
        AppError::conflict_code(
            "plugin_installer_missing",
            "the organization has no active administrator to run the plugin on behalf of",
        )
    })?;
    Ok(UserId::new(id)?)
}

pub async fn save(
    tx: &mut Tx,
    package: &PluginPackage,
    installed_by: Option<&UserId>,
    pack_key: Option<&str>,
) -> Result<(), AppError> {
    let manifest = &package.manifest;
    sqlx::query(
        "INSERT INTO plugins_installed (plugin_key, name, version, description, manifest, \
         checksum, enabled, settings, installed_by, pack_key) VALUES (?, ?, ?, ?, ?, ?, TRUE, JSON_OBJECT(), ?, ?) \
         ON DUPLICATE KEY UPDATE name = VALUES(name), version = VALUES(version), \
         description = VALUES(description), manifest = VALUES(manifest), \
         checksum = VALUES(checksum), installed_by = VALUES(installed_by), \
         pack_key = VALUES(pack_key), updated_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(&manifest.key)
    .bind(&manifest.name)
    .bind(&manifest.version)
    .bind(&manifest.description)
    .bind(Json(manifest))
    .bind(&package.checksum)
    .bind(installed_by.map(UserId::as_str))
    .bind(pack_key)
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

pub async fn files_under(
    pool: &MySqlPool,
    key: &str,
    directory: &str,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let rows = sqlx::query(
        "SELECT path, content FROM plugins_files WHERE plugin_key = ? AND path LIKE ? ORDER BY path",
    )
    .bind(key)
    .bind(format!("{}/%", directory.trim_end_matches('/')))
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| Ok((row.try_get("path")?, row.try_get("content")?)))
        .collect()
}

pub async fn backend(pool: &MySqlPool, key: &str) -> Result<Option<Vec<u8>>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT content FROM plugins_files WHERE plugin_key = ? AND path = 'backend.wasm'",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?)
}

pub async fn set_enabled(tx: &mut Tx, key: &str, enabled: bool) -> Result<(), AppError> {
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
    tx: &mut Tx,
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

pub async fn delete(tx: &mut Tx, key: &str) -> Result<(), AppError> {
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

pub async fn kv_increment(
    pool: &MySqlPool,
    key: &str,
    collection: &str,
    item: &str,
    delta: i64,
) -> Result<i64, AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT IGNORE INTO plugins_data (plugin_key, collection, item_id, value) VALUES (?, ?, ?, ?)",
    )
    .bind(key)
    .bind(collection)
    .bind(item)
    .bind(Json(Value::from(0)))
    .execute(&mut *tx)
    .await?;
    let Json(current): Json<Value> = sqlx::query_scalar(
        "SELECT value FROM plugins_data WHERE plugin_key = ? AND collection = ? AND item_id = ? FOR UPDATE",
    )
    .bind(key)
    .bind(collection)
    .bind(item)
    .fetch_one(&mut *tx)
    .await?;
    let current = current.as_i64().ok_or_else(|| {
        AppError::conflict_code(
            "plugin_kv_not_a_counter",
            "the stored value is not an integer, so it cannot be incremented",
        )
    })?;
    let next = current
        .checked_add(delta)
        .ok_or_else(|| AppError::Validation("the counter would overflow".to_owned()))?;
    sqlx::query(
        "UPDATE plugins_data SET value = ? WHERE plugin_key = ? AND collection = ? AND item_id = ?",
    )
    .bind(Json(Value::from(next)))
    .bind(key)
    .bind(collection)
    .bind(item)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub async fn kv_set_if_absent(
    pool: &MySqlPool,
    key: &str,
    collection: &str,
    item: &str,
    value: &Value,
) -> Result<bool, AppError> {
    Ok(sqlx::query(
        "INSERT IGNORE INTO plugins_data (plugin_key, collection, item_id, value) VALUES (?, ?, ?, ?)",
    )
    .bind(key)
    .bind(collection)
    .bind(item)
    .bind(Json(value))
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

#[derive(Debug, Clone)]
pub struct InstalledPack {
    pub key: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn save_pack(
    tx: &mut Tx,
    manifest: &crate::pack::PackManifest,
    installed_by: Option<&UserId>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_packs (pack_key, name, version, description, manifest, installed_by) \
         VALUES (?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name), \
         version = VALUES(version), description = VALUES(description), manifest = VALUES(manifest), \
         installed_by = VALUES(installed_by), updated_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(&manifest.key)
    .bind(&manifest.name)
    .bind(&manifest.version)
    .bind(&manifest.description)
    .bind(Json(manifest))
    .bind(installed_by.map(UserId::as_str))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn list_packs(pool: &MySqlPool) -> Result<Vec<InstalledPack>, AppError> {
    let rows = sqlx::query(
        "SELECT pack_key, name, version, description, installed_at, updated_at FROM plugins_packs ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(InstalledPack {
                key: row.try_get("pack_key")?,
                name: row.try_get("name")?,
                version: row.try_get("version")?,
                description: row.try_get("description")?,
                installed_at: instant(row, "installed_at")?,
                updated_at: instant(row, "updated_at")?,
            })
        })
        .collect()
}

pub async fn find_pack(pool: &MySqlPool, key: &str) -> Result<Option<InstalledPack>, AppError> {
    Ok(list_packs(pool)
        .await?
        .into_iter()
        .find(|pack| pack.key == key))
}

pub async fn set_pack(pool: &MySqlPool, plugin: &str, pack: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_installed SET pack_key = ? WHERE plugin_key = ?")
        .bind(pack)
        .bind(plugin)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PackResource {
    pub kind: String,
    pub resource_id: Option<String>,
    pub data_hash: String,
    pub applied: serde_json::Value,
    pub status: String,
}

pub async fn pack_resources(
    pool: &MySqlPool,
    pack: &str,
) -> Result<std::collections::BTreeMap<String, PackResource>, AppError> {
    let rows = sqlx::query(
        "SELECT resource_key, kind, resource_id, data_hash, applied, status \
         FROM plugins_pack_resources WHERE pack_key = ?",
    )
    .bind(pack)
    .fetch_all(pool)
    .await?;
    let mut resources = std::collections::BTreeMap::new();
    for row in rows {
        let applied: Option<Json<serde_json::Value>> = row.try_get("applied")?;
        resources.insert(
            row.try_get::<String, _>("resource_key")?,
            PackResource {
                kind: row.try_get("kind")?,
                resource_id: row.try_get("resource_id")?,
                data_hash: row.try_get("data_hash")?,
                applied: applied.map(|Json(value)| value).unwrap_or_default(),
                status: row.try_get("status")?,
            },
        );
    }
    Ok(resources)
}

pub struct SavedResource<'a> {
    pub pack: &'a str,
    pub key: &'a str,
    pub kind: &'a str,
    pub target: Option<&'a str>,
    pub resource_id: Option<&'a str>,
    pub mode: &'a str,
    pub data_hash: &'a str,
    pub applied: Option<&'a serde_json::Value>,
    pub status: &'a str,
    pub error: Option<serde_json::Value>,
}

pub async fn save_pack_resource(
    pool: &MySqlPool,
    resource: SavedResource<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_pack_resources (pack_key, resource_key, kind, target, resource_id, mode, \
         data_hash, applied, status, error) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE kind = VALUES(kind), target = VALUES(target), \
         resource_id = COALESCE(VALUES(resource_id), resource_id), mode = VALUES(mode), \
         data_hash = IF(VALUES(status) = 'failed', data_hash, VALUES(data_hash)), \
         applied = COALESCE(VALUES(applied), applied), status = VALUES(status), error = VALUES(error), \
         applied_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(resource.pack)
    .bind(resource.key)
    .bind(resource.kind)
    .bind(resource.target)
    .bind(resource.resource_id)
    .bind(resource.mode)
    .bind(resource.data_hash)
    .bind(resource.applied.map(Json))
    .bind(resource.status)
    .bind(resource.error.map(Json))
    .execute(pool)
    .await?;
    Ok(())
}
