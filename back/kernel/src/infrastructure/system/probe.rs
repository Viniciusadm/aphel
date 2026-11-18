use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::shared::SharedFileStorage;
use crate::application::system::ports::HealthProbe;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct SystemHealthProbe {
    pool: Option<MySqlPool>,
    storage: SharedFileStorage,
}

impl SystemHealthProbe {
    pub(crate) fn new(pool: Option<MySqlPool>, storage: SharedFileStorage) -> Self {
        Self { pool, storage }
    }

    fn pool(&self) -> Result<&MySqlPool, AppError> {
        self.pool
            .as_ref()
            .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))
    }
}

/// True when the database answers a trivial query.
pub async fn database_up(pool: &MySqlPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

pub async fn probe_storage(storage: &SharedFileStorage) -> Result<(), AppError> {
    let key = format!("system/health/probe-{}", Uuid::now_v7().simple());
    storage.put(&key, Bytes::from_static(b"ok")).await?;
    storage.delete(&key).await
}

pub async fn oldest_ready_job(
    pool: &MySqlPool,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let oldest: Option<NaiveDateTime> = sqlx::query_scalar(
        "SELECT MIN(run_at) FROM jobs \
         WHERE finished_at IS NULL AND failed_at IS NULL AND locked_at IS NULL AND run_at <= ?",
    )
    .bind(mapper::naive(now))
    .fetch_one(pool)
    .await?;
    Ok(oldest.map(mapper::utc))
}

pub async fn oldest_due_schedule(pool: &MySqlPool) -> Result<Option<DateTime<Utc>>, AppError> {
    let oldest: Option<NaiveDateTime> =
        sqlx::query_scalar("SELECT MIN(next_run_at) FROM scheduled_jobs WHERE enabled = 1")
            .fetch_one(pool)
            .await?;
    Ok(oldest.map(mapper::utc))
}

pub async fn latest_heartbeat(
    pool: &MySqlPool,
    role: &str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let latest: Option<NaiveDateTime> =
        sqlx::query_scalar("SELECT MAX(last_seen_at) FROM system_heartbeats WHERE role = ?")
            .bind(role)
            .fetch_one(pool)
            .await?;
    Ok(latest.map(mapper::utc))
}

pub async fn latest_heartbeat_on_host(
    pool: &MySqlPool,
    role: &str,
    hostname: &str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let latest: Option<NaiveDateTime> = sqlx::query_scalar(
        "SELECT MAX(last_seen_at) FROM system_heartbeats WHERE role = ? AND hostname = ?",
    )
    .bind(role)
    .bind(hostname)
    .fetch_one(pool)
    .await?;
    Ok(latest.map(mapper::utc))
}

#[async_trait]
impl HealthProbe for SystemHealthProbe {
    async fn database(&self) -> Result<(), AppError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(self.pool()?)
            .await?;
        Ok(())
    }

    async fn storage(&self) -> Result<(), AppError> {
        probe_storage(&self.storage).await
    }

    async fn latest_heartbeat(&self, role: &str) -> Result<Option<DateTime<Utc>>, AppError> {
        latest_heartbeat(self.pool()?, role).await
    }

    async fn oldest_due_schedule(&self) -> Result<Option<DateTime<Utc>>, AppError> {
        oldest_due_schedule(self.pool()?).await
    }

    async fn oldest_ready_job(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        oldest_ready_job(self.pool()?, now).await
    }
}
