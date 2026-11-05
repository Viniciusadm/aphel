use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::MySqlPool;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};

pub const NOTIFICATIONS_PURGE_JOB: &str = "notifications.purge";
pub const NOTIFICATIONS_PURGE_CRON: &str = "0 30 4 * * *";
pub const READ_NOTIFICATION_RETENTION_DAYS: i64 = 90;
pub const UNREAD_NOTIFICATION_RETENTION_DAYS: i64 = 365;
const PURGE_BATCH: u64 = 5_000;

pub fn notifications_purge_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(NOTIFICATIONS_PURGE_JOB, NOTIFICATIONS_PURGE_CRON)
        .expect("the notifications purge cron expression is valid")
        .with_max_attempts(1)
}

pub fn notifications_purge_handler(pool: MySqlPool) -> Arc<dyn JobHandler> {
    Arc::new(NotificationPurgeHandler { pool })
}

struct NotificationPurgeHandler {
    pool: MySqlPool,
}

impl NotificationPurgeHandler {
    async fn purge(
        &self,
        sql: &'static str,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, AppError> {
        let mut removed = 0;
        loop {
            let done = sqlx::query(sql)
                .bind(before)
                .bind(PURGE_BATCH)
                .execute(&self.pool)
                .await?
                .rows_affected();
            removed += done;
            if done < PURGE_BATCH {
                return Ok(removed);
            }
        }
    }
}

#[async_trait]
impl JobHandler for NotificationPurgeHandler {
    fn kind(&self) -> &str {
        NOTIFICATIONS_PURGE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let now = Utc::now();
        let read = self
            .purge(
                "DELETE FROM notifications WHERE read_at IS NOT NULL AND read_at < ? LIMIT ?",
                (now - Duration::days(READ_NOTIFICATION_RETENTION_DAYS)).naive_utc(),
            )
            .await?;
        let unread = self
            .purge(
                "DELETE FROM notifications WHERE created_at < ? LIMIT ?",
                (now - Duration::days(UNREAD_NOTIFICATION_RETENTION_DAYS)).naive_utc(),
            )
            .await?;
        tracing::debug!(read, unread, "old notifications purged");
        Ok(())
    }
}
