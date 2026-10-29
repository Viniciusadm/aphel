use crate::modules::manifest::Migration;
use sha2::{Digest, Sha256};
use sqlx::{MySqlConnection, MySqlPool, Row};
use tracing::info;

use crate::errors::AppError;
use crate::modules::builder::{App, LoadedModule};

pub fn checksum(migration: &Migration) -> String {
    checksum_of(migration.sql)
}

pub fn checksum_of(sql: &str) -> String {
    Sha256::digest(sql.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub async fn run(pool: &MySqlPool, app: &App) -> Result<(), AppError> {
    for entry in app.modules().iter().filter(|entry| entry.is_compatible()) {
        run_module(pool, entry).await?;
    }
    sync_required(pool, app).await
}

pub async fn applied(pool: &MySqlPool, key: &str) -> Result<Vec<(u32, String)>, AppError> {
    Ok(sqlx::query(
        "SELECT version, checksum FROM module_migrations WHERE module_key = ? ORDER BY version",
    )
    .bind(key)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| Ok((row.try_get("version")?, row.try_get("checksum")?)))
    .collect::<Result<_, sqlx::Error>>()?)
}

async fn run_module(pool: &MySqlPool, entry: &LoadedModule) -> Result<(), AppError> {
    let key = entry.key();
    if entry.manifest.migrations.is_empty() {
        return Ok(());
    }
    let applied = applied(pool, key).await?;
    for migration in &entry.manifest.migrations {
        let sum = checksum(migration);
        if let Some((_, stored)) = applied
            .iter()
            .find(|(version, _)| *version == migration.version)
        {
            if *stored != sum {
                return Err(AppError::Configuration(format!(
                    "module {key}: migration {} was changed after being applied",
                    migration.version
                )));
            }
            continue;
        }
        info!(
            "applying module migration {key} {} ({})",
            migration.version, migration.description
        );
        sqlx::raw_sql(migration.sql)
            .execute(pool)
            .await
            .map_err(|error| {
                AppError::Persistence(format!(
                    "module {key} migration {} failed: {error}",
                    migration.version
                ))
            })?;
        sqlx::query(
            "INSERT INTO module_migrations (module_key, version, description, checksum) VALUES (?, ?, ?, ?)",
        )
        .bind(key)
        .bind(migration.version)
        .bind(migration.description)
        .bind(&sum)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn revert(pool: &MySqlPool, entry: &LoadedModule) -> Result<bool, AppError> {
    let key = entry.key();
    let reverted = match entry.manifest.uninstall_sql {
        Some(sql) => {
            info!("reverting the schema of module {key}");
            sqlx::raw_sql(sql).execute(pool).await?;
            true
        }
        None => false,
    };
    sqlx::query("DELETE FROM module_migrations WHERE module_key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(reverted)
}

async fn sync_required(pool: &MySqlPool, app: &App) -> Result<(), AppError> {
    for entry in app.modules().iter().filter(|entry| {
        entry.manifest.enabled_by_default && !entry.manifest.required && entry.is_compatible()
    }) {
        let mut connection = pool.acquire().await?;
        let inserted = sqlx::query(
            "INSERT IGNORE INTO installed_modules (module_key, enabled, settings, installed_version, enabled_at) \
             VALUES (?, TRUE, JSON_OBJECT(), ?, CURRENT_TIMESTAMP(6))",
        )
        .bind(entry.key())
        .bind(entry.manifest.version)
        .execute(&mut *connection)
        .await?
        .rows_affected()
            > 0;
        if inserted {
            apply_grants(&mut connection, app, entry.key()).await?;
        }
    }
    for entry in app
        .modules()
        .iter()
        .filter(|entry| entry.manifest.required && entry.is_compatible())
    {
        let mut connection = pool.acquire().await?;
        let inserted = sqlx::query(
            "INSERT IGNORE INTO installed_modules (module_key, enabled, settings, installed_version, enabled_at) \
             VALUES (?, TRUE, JSON_OBJECT(), ?, CURRENT_TIMESTAMP(6))",
        )
        .bind(entry.key())
        .bind(entry.manifest.version)
        .execute(&mut *connection)
        .await?
        .rows_affected()
            > 0;
        if inserted {
            apply_grants(&mut connection, app, entry.key()).await?;
            continue;
        }
        sqlx::query(
            "UPDATE installed_modules SET enabled = TRUE, installed_version = ? WHERE module_key = ?",
        )
        .bind(entry.manifest.version)
        .bind(entry.key())
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

pub async fn apply_grants(
    connection: &mut MySqlConnection,
    app: &App,
    module: &str,
) -> Result<u64, AppError> {
    let mut granted = 0;
    for grant in app.catalog().grants_of(module) {
        let record_scoped = app
            .catalog()
            .permissions
            .iter()
            .any(|entry| entry.item.permission == grant.permission && entry.item.record_scoped);
        for role in &grant.roles {
            let scope = match (*role, record_scoped && !grant.organization_wide) {
                (_, false) | ("admin", true) => "organization",
                ("manager", true) => "team",
                _ => "own",
            };
            granted += sqlx::query(
                "INSERT IGNORE INTO role_permissions (role_id, permission, scope) \
                 SELECT id, ?, ? FROM roles WHERE `key` = ?",
            )
            .bind(grant.permission.as_str())
            .bind(scope)
            .bind(*role)
            .execute(&mut *connection)
            .await?
            .rows_affected();
        }
    }
    Ok(granted)
}

pub async fn apply_enabled_grants(
    connection: &mut MySqlConnection,
    app: &App,
) -> Result<u64, AppError> {
    let mut granted = 0;
    for entry in app.modules() {
        if app.is_enabled(entry.key()) {
            granted += apply_grants(connection, app, entry.key()).await?;
        }
    }
    Ok(granted)
}
