use aphel_sdk::AppError;
use aphel_sdk::points::{FailureEntry, FailureSource, HealthSource};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySqlPool, Row};

pub(crate) struct AutomationFailures;

#[async_trait]
impl HealthSource for AutomationFailures {
    async fn recent_failures(
        &self,
        pool: &MySqlPool,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT r.automation_id AS reference_id, \
               r.trigger_type AS kind, r.error, COALESCE(r.finished_at, r.started_at) AS occurred_at \
             FROM automation_runs r \
             WHERE r.status = 'failed' AND r.started_at >= ? \
             ORDER BY r.started_at DESC LIMIT ?",
        )
        .bind(since.naive_utc())
        .bind(limit)
        .fetch_all(pool)
        .await?;
        rows.iter()
            .map(|row| {
                let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
                Ok(FailureEntry::new(
                    FailureSource::Automation,
                    row.try_get("reference_id")?,
                    row.try_get("kind")?,
                    row.try_get("error")?,
                    None,
                    occurred_at.and_utc(),
                ))
            })
            .collect()
    }
}
