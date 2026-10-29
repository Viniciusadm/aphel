use sqlx::MySqlPool;
use sqlx::types::Json;

use crate::application::system::upgrade::UpgradeRecord;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlUpgradeLog {
    pool: MySqlPool,
}

impl MySqlUpgradeLog {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn available(&self) -> Result<bool, AppError> {
        let found: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'system_upgrades' LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }

    pub async fn last_version(&self) -> Result<Option<String>, AppError> {
        if !self.available().await? {
            return Ok(None);
        }
        let version: Option<String> = sqlx::query_scalar(
            "SELECT to_version FROM system_upgrades WHERE status = 'succeeded' ORDER BY started_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(version)
    }

    pub async fn record(&self, record: &UpgradeRecord) -> Result<bool, AppError> {
        if !self.available().await? {
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO system_upgrades (id, from_version, to_version, from_migration, to_migration, status, \
               applied_migrations, error, hostname, backup_confirmed, started_at, finished_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind(&record.from_version)
        .bind(&record.to_version)
        .bind(record.from_migration)
        .bind(record.to_migration)
        .bind(record.outcome.as_str())
        .bind(Json(&record.applied_migrations))
        .bind(&record.error)
        .bind(&record.hostname)
        .bind(record.backup_confirmed)
        .bind(mapper::naive(record.started_at))
        .bind(mapper::naive(record.finished_at))
        .execute(&self.pool)
        .await?;
        Ok(true)
    }
}
