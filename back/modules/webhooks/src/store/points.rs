use aphel_sdk::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::points::{FailureEntry, FailureSource, HealthSource};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Row;

pub(crate) struct WebhookFailures;

#[async_trait]
impl HealthSource for WebhookFailures {
    async fn recent_failures(
        &self,
        pool: &Db,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT d.endpoint_id AS reference_id, \
               d.event_type AS kind, d.error, d.response_status, \
               COALESCE(d.completed_at, d.last_attempt_at, d.created_at) AS occurred_at \
             FROM webhook_deliveries d \
             WHERE d.status = 'failed' AND d.created_at >= ? \
             ORDER BY d.created_at DESC LIMIT ?",
        )
        .bind(since.naive_utc())
        .bind(limit)
        .fetch_all(pool)
        .await?;
        rows.iter()
            .map(|row| {
                let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
                Ok(FailureEntry::new(
                    FailureSource::Webhook,
                    row.try_get("reference_id")?,
                    row.try_get("kind")?,
                    row.try_get("error")?,
                    row.try_get("response_status")?,
                    occurred_at.and_utc(),
                ))
            })
            .collect()
    }
}
