use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::application::system::{APHEL_VERSION, WORKER_ROLE};
use crate::errors::AppError;
use crate::infrastructure::jobs::Shutdown;
use crate::infrastructure::repositories::mapper;

const STALE_ROW_RETENTION_HOURS: i64 = 24;

#[derive(Clone)]
pub struct HeartbeatRecorder {
    pool: MySqlPool,
    instance_id: String,
    role: &'static str,
    hostname: Option<String>,
    started_at: DateTime<Utc>,
}

impl HeartbeatRecorder {
    pub(crate) fn new(pool: MySqlPool, instance_id: impl Into<String>, role: &'static str) -> Self {
        Self {
            pool,
            instance_id: instance_id.into(),
            role,
            hostname: hostname(),
            started_at: Utc::now(),
        }
    }

    pub(crate) async fn beat(&self, now: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO system_heartbeats (instance_id, role, hostname, version, started_at, last_seen_at) \
             VALUES (?, ?, ?, ?, ?, ?) AS incoming \
             ON DUPLICATE KEY UPDATE last_seen_at = incoming.last_seen_at, version = incoming.version",
        )
        .bind(&self.instance_id)
        .bind(self.role)
        .bind(&self.hostname)
        .bind(APHEL_VERSION)
        .bind(mapper::naive(self.started_at))
        .bind(mapper::naive(now))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn prune(&self, now: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("DELETE FROM system_heartbeats WHERE last_seen_at < ?")
            .bind(mapper::naive(
                now - chrono::Duration::hours(STALE_ROW_RETENTION_HOURS),
            ))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn remove(&self) -> Result<(), AppError> {
        sqlx::query("DELETE FROM system_heartbeats WHERE instance_id = ?")
            .bind(&self.instance_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn run(self, interval: Duration, mut shutdown: Shutdown) {
        if let Err(error) = self.prune(Utc::now()).await {
            warn!("could not prune old worker heartbeats: {error}");
        }
        loop {
            if let Err(error) = self.beat(Utc::now()).await {
                warn!("could not record the worker heartbeat: {error}");
            }
            tokio::select! {
                () = shutdown.wait() => break,
                () = tokio::time::sleep(interval) => {}
            }
        }
        if let Err(error) = self.remove().await {
            warn!("could not remove the worker heartbeat: {error}");
        }
    }
}

pub fn spawn_heartbeat(
    pool: MySqlPool,
    instance_id: &str,
    interval: Duration,
    shutdown: Shutdown,
) -> JoinHandle<()> {
    let recorder = HeartbeatRecorder::new(pool, instance_id, WORKER_ROLE);
    crate::tenancy::spawn(recorder.run(interval, shutdown))
}

pub fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/proc/sys/kernel/hostname").ok())
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|value| value.trim().chars().take(255).collect::<String>())
        .filter(|value| !value.is_empty())
}
