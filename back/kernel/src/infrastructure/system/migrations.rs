use std::collections::HashSet;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::{MySqlPool, Row};

use crate::application::system::model::{
    CoreMigrationStatus, InstalledModuleInfo, MigrationEntry, MigrationStatus,
    ModuleMigrationStatus,
};
use crate::application::system::ports::{MigrationInspector, ModuleInventory};
use crate::database::migrations::MIGRATOR;
use crate::errors::AppError;

#[derive(Clone)]
pub struct MySqlMigrationInspector {
    pool: MySqlPool,
}

impl MySqlMigrationInspector {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

async fn table_exists(pool: &MySqlPool, table: &str) -> Result<bool, AppError> {
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ? LIMIT 1",
    )
    .bind(table)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

async fn core_status(pool: &MySqlPool) -> Result<CoreMigrationStatus, AppError> {
    let applied: Vec<(i64, String, bool)> = if table_exists(pool, "_sqlx_migrations").await? {
        sqlx::query("SELECT version, description, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.try_get("version")?,
                    row.try_get("description")?,
                    row.try_get("success")?,
                ))
            })
            .collect::<Result<_, sqlx::Error>>()?
    } else {
        Vec::new()
    };
    let known: Vec<MigrationEntry> = MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| MigrationEntry {
            version: migration.version,
            description: migration.description.to_string(),
        })
        .collect();
    let mut status = CoreMigrationStatus {
        known: known.len() as u64,
        ..CoreMigrationStatus::default()
    };
    for (version, description, success) in &applied {
        let entry = MigrationEntry {
            version: *version,
            description: description.clone(),
        };
        if !success {
            status.failed.push(entry);
            continue;
        }
        status.applied += 1;
        status.latest_applied = Some(
            status
                .latest_applied
                .map_or(*version, |latest| latest.max(*version)),
        );
        if !known.iter().any(|migration| migration.version == *version) {
            status.unknown_applied.push(entry);
        }
    }
    status.pending = known
        .into_iter()
        .filter(|migration| {
            !applied
                .iter()
                .any(|(version, _, _)| *version == migration.version)
        })
        .collect();
    Ok(status)
}

fn checksum(sql: &str) -> String {
    Sha256::digest(sql.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn module_status(pool: &MySqlPool) -> Result<Vec<ModuleMigrationStatus>, AppError> {
    let applied: Vec<(String, u32, String)> = if table_exists(pool, "module_migrations").await? {
        sqlx::query("SELECT module_key, version, checksum FROM module_migrations")
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.try_get("module_key")?,
                    row.try_get("version")?,
                    row.try_get("checksum")?,
                ))
            })
            .collect::<Result<_, sqlx::Error>>()?
    } else {
        Vec::new()
    };
    let enabled = enabled_modules(pool).await.unwrap_or_default();
    let mut statuses = Vec::new();
    for entry in crate::modules::app().modules() {
        let key = entry.key();
        let mut status = ModuleMigrationStatus {
            key: key.to_owned(),
            name: entry.manifest.name.to_owned(),
            version: entry.manifest.version.to_owned(),
            min_aphel_version: entry.manifest.min_aphel_version.to_owned(),
            compatibility: entry.compatibility.code().to_owned(),
            compatibility_detail: entry.compatibility.detail(),
            enabled: enabled.contains(key),
            applied: 0,
            pending: Vec::new(),
            changed: Vec::new(),
        };
        for migration in &entry.manifest.migrations {
            let migration_entry = MigrationEntry {
                version: i64::from(migration.version),
                description: migration.description.to_owned(),
            };
            match applied
                .iter()
                .find(|(module, version, _)| module == key && *version == migration.version)
            {
                Some((_, _, stored)) => {
                    status.applied += 1;
                    if *stored != checksum(migration.sql) {
                        status.changed.push(migration_entry);
                    }
                }
                None => status.pending.push(migration_entry),
            }
        }
        statuses.push(status);
    }
    Ok(statuses)
}

pub async fn enabled_modules(pool: &MySqlPool) -> Result<HashSet<String>, AppError> {
    if !table_exists(pool, "installed_modules").await? {
        return Ok(HashSet::new());
    }
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT module_key FROM installed_modules WHERE enabled = 1")
            .fetch_all(pool)
            .await?;
    Ok(keys.into_iter().collect())
}

pub async fn migration_status(pool: &MySqlPool) -> Result<MigrationStatus, AppError> {
    Ok(MigrationStatus {
        core: core_status(pool).await?,
        modules: module_status(pool).await?,
    })
}

#[async_trait]
impl MigrationInspector for MySqlMigrationInspector {
    async fn status(&self) -> Result<MigrationStatus, AppError> {
        migration_status(&self.pool).await
    }
}

#[derive(Clone)]
pub struct CompiledModuleInventory {
    pool: Option<MySqlPool>,
}

impl CompiledModuleInventory {
    pub(crate) fn new(pool: Option<MySqlPool>) -> Self {
        Self { pool }
    }
}

pub fn compiled_modules(enabled: &HashSet<String>) -> Vec<InstalledModuleInfo> {
    crate::modules::app()
        .modules()
        .iter()
        .map(|entry| InstalledModuleInfo {
            key: entry.key().to_owned(),
            name: entry.manifest.name.to_owned(),
            version: entry.manifest.version.to_owned(),
            compatibility: entry.compatibility.code().to_owned(),
            compatibility_detail: entry.compatibility.detail(),
            enabled: enabled.contains(entry.key()),
        })
        .collect()
}

#[async_trait]
impl ModuleInventory for CompiledModuleInventory {
    async fn installed(&self) -> Result<Vec<InstalledModuleInfo>, AppError> {
        let enabled = match &self.pool {
            Some(pool) => enabled_modules(pool).await?,
            None => HashSet::new(),
        };
        Ok(compiled_modules(&enabled))
    }
}
