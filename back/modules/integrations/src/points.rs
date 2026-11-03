use aphel_sdk::AppError;
use aphel_sdk::points::{FailureEntry, FailureSource, HealthSource};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySqlPool, Row};

pub(crate) struct IntegrationFailures;

#[async_trait]
impl HealthSource for IntegrationFailures {
    async fn recent_failures(
        &self,
        pool: &MySqlPool,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT i.id AS reference_id, \
               i.provider_key AS kind, i.last_error AS error, i.last_error_at AS occurred_at \
             FROM integrations i \
             WHERE i.status = 'error' AND i.last_error_at IS NOT NULL AND i.last_error_at >= ? \
             ORDER BY i.last_error_at DESC LIMIT ?",
        )
        .bind(since.naive_utc())
        .bind(limit)
        .fetch_all(pool)
        .await?;
        rows.iter()
            .map(|row| {
                let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
                Ok(FailureEntry::new(
                    FailureSource::Integration,
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
