use crate::engine::IntegrationError;
use crate::{ActiveIntegration, IntegrationStore};
use aphel_sdk::AppError;
use aphel_sdk::points::{FailureEntry, FailureSource, HealthSource};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::types::Json;
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

pub(crate) struct Integrations;

#[async_trait]
impl IntegrationStore for Integrations {
    async fn active(
        &self,
        pool: &MySqlPool,
        provider_key: &str,
    ) -> Result<Option<ActiveIntegration>, AppError> {
        let row = sqlx::query(
            "SELECT id, config, status, updated_at FROM integrations \
             WHERE provider_key = ? AND status IN ('active', 'error')",
        )
        .bind(provider_key)
        .fetch_optional(pool)
        .await?;
        row.map(|row| {
            let Json(config): Json<serde_json::Value> = row.try_get("config")?;
            Ok(ActiveIntegration {
                id: row.try_get("id")?,
                status: row.try_get("status")?,
                updated_at: row.try_get("updated_at")?,
                config,
            })
        })
        .transpose()
    }

    async fn record_failure(
        &self,
        pool: &MySqlPool,
        integration_id: &str,
        error: &IntegrationError,
    ) -> Result<(), AppError> {
        let entry = serde_json::to_value(error).map_err(|failure| {
            AppError::Internal(format!("invalid integration error: {failure}"))
        })?;
        sqlx::query(
            "UPDATE integrations SET status = 'error', last_error = ?, last_error_at = ?, \
             recent_errors = JSON_REMOVE(JSON_ARRAY_INSERT(recent_errors, '$[0]', CAST(? AS JSON)), '$[5]') \
             WHERE id = ? AND status IN ('active', 'error')",
        )
        .bind(&error.message)
        .bind(error.at.naive_utc())
        .bind(Json(entry))
        .bind(integration_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn record_recovery(
        &self,
        pool: &MySqlPool,
        integration_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE integrations SET status = 'active' WHERE id = ? AND status = 'error'")
            .bind(integration_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
