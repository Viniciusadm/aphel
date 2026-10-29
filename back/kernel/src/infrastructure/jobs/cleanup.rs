use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;
use tracing::info;

use crate::application::jobs::ScheduledJobDefinition;
use crate::application::shared::{JobContext, JobHandler};
use crate::errors::AppError;

pub(crate) const JOBS_CLEANUP_KIND: &str = "jobs.cleanup";
pub(crate) const JOBS_CLEANUP_CRON: &str = "0 17 * * * *";
pub(crate) const FINISHED_JOB_RETENTION: Duration = Duration::from_secs(7 * 24 * 3600);
pub(crate) const FAILED_JOB_RETENTION: Duration = Duration::from_secs(30 * 24 * 3600);
const DELETE_BATCH_SIZE: u32 = 5000;

pub(crate) fn jobs_cleanup_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(JOBS_CLEANUP_KIND, JOBS_CLEANUP_CRON)
        .expect("the jobs cleanup cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) struct JobCleanupHandler {
    pool: MySqlPool,
    finished_retention: Duration,
    failed_retention: Duration,
}

impl JobCleanupHandler {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            finished_retention: FINISHED_JOB_RETENTION,
            failed_retention: FAILED_JOB_RETENTION,
        }
    }

    pub(crate) async fn purge(&self) -> Result<u64, AppError> {
        let mut deleted = 0;
        for (statement, retention) in [
            (
                "DELETE FROM jobs WHERE finished_at < UTC_TIMESTAMP(6) - INTERVAL ? SECOND LIMIT ?",
                self.finished_retention,
            ),
            (
                "DELETE FROM jobs WHERE failed_at < UTC_TIMESTAMP(6) - INTERVAL ? SECOND LIMIT ?",
                self.failed_retention,
            ),
        ] {
            loop {
                let affected = sqlx::query(statement)
                    .bind(retention.as_secs())
                    .bind(DELETE_BATCH_SIZE)
                    .execute(&self.pool)
                    .await?
                    .rows_affected();
                deleted += affected;
                if affected < u64::from(DELETE_BATCH_SIZE) {
                    break;
                }
            }
        }
        Ok(deleted)
    }
}

#[async_trait]
impl JobHandler for JobCleanupHandler {
    fn kind(&self) -> &str {
        JOBS_CLEANUP_KIND
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let deleted = self.purge().await?;
        if deleted > 0 {
            info!(deleted, "deleted old finished and failed jobs");
        }
        Ok(())
    }
}
