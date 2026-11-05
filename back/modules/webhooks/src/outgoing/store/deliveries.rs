use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::outgoing::{
    DeliveryAttempt, DeliveryListFilter, DeliveryStatus, DeliveryTrigger, WebhookDelivery,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const DELIVERY_COLUMNS: &str = "id, endpoint_id, event_id, event_type, trigger_kind, \
     dedupe_key, redelivery_of, body, status, attempts, max_attempts, next_attempt_at, last_attempt_at, \
     response_status, response_body, duration_ms, error, requested_by, created_at, completed_at";

#[derive(Clone)]
pub(crate) struct WebhookDeliveryStore {
    pool: MySqlPool,
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value)
        .map_err(|_| mapper::invalid_column(&format!("webhook_deliveries.{column}"), &value))
}

fn delivery_from_row(row: &MySqlRow) -> Result<WebhookDelivery, AppError> {
    let optional = |column: &str| -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(row
            .try_get::<Option<NaiveDateTime>, _>(column)?
            .map(mapper::utc))
    };
    let trigger: String = row.try_get("trigger_kind")?;
    let status: String = row.try_get("status")?;
    let redelivery_of = row
        .try_get::<Option<String>, _>("redelivery_of")?
        .map(|value| {
            Uuid::parse_str(&value)
                .map_err(|_| mapper::invalid_column("webhook_deliveries.redelivery_of", &value))
        })
        .transpose()?;
    let requested_by = row
        .try_get::<Option<String>, _>("requested_by")?
        .map(|value| mapper::typed_id(value, UserId::new))
        .transpose()?;
    Ok(WebhookDelivery {
        id: uuid_column(row, "id")?,
        endpoint_id: uuid_column(row, "endpoint_id")?,
        event_id: uuid_column(row, "event_id")?,
        event_type: row.try_get("event_type")?,
        trigger: DeliveryTrigger::parse(&trigger)
            .ok_or_else(|| mapper::invalid_column("webhook_deliveries.trigger_kind", &trigger))?,
        dedupe_key: row.try_get("dedupe_key")?,
        redelivery_of,
        body: row.try_get("body")?,
        status: DeliveryStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("webhook_deliveries.status", &status))?,
        attempts: row.try_get("attempts")?,
        max_attempts: row.try_get("max_attempts")?,
        next_attempt_at: optional("next_attempt_at")?,
        last_attempt_at: optional("last_attempt_at")?,
        response_status: row.try_get("response_status")?,
        response_body: row.try_get("response_body")?,
        duration_ms: row.try_get("duration_ms")?,
        error: row.try_get("error")?,
        requested_by,
        created_at: mapper::utc(row.try_get("created_at")?),
        completed_at: optional("completed_at")?,
    })
}

impl WebhookDeliveryStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        delivery: &WebhookDelivery,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO webhook_deliveries (id, endpoint_id, event_id, event_type, \
             trigger_kind, dedupe_key, redelivery_of, body, status, attempts, max_attempts, next_attempt_at, \
             requested_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(delivery.id.to_string())
        .bind(delivery.endpoint_id.to_string())
        .bind(delivery.event_id.to_string())
        .bind(&delivery.event_type)
        .bind(delivery.trigger.as_str())
        .bind(&delivery.dedupe_key)
        .bind(delivery.redelivery_of.map(|id| id.to_string()))
        .bind(&delivery.body)
        .bind(delivery.status.as_str())
        .bind(delivery.attempts)
        .bind(delivery.max_attempts)
        .bind(delivery.next_attempt_at.map(mapper::naive))
        .bind(delivery.requested_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(delivery.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn find(
        &self,
        endpoint_id: Uuid,
        id: Uuid,
    ) -> Result<Option<WebhookDelivery>, AppError> {
        let sql = format!(
            "SELECT {DELIVERY_COLUMNS} FROM webhook_deliveries \
             WHERE endpoint_id = ? AND id = ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(endpoint_id.to_string())
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(delivery_from_row)
            .transpose()
    }

    pub(crate) async fn list(
        &self,
        endpoint_id: Uuid,
        filter: DeliveryListFilter,
    ) -> Result<(Vec<WebhookDelivery>, u64), AppError> {
        let status = filter.status.map(|status| status.as_str());
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE endpoint_id = ? \
             AND (? IS NULL OR status = ?)",
        )
        .bind(endpoint_id.to_string())
        .bind(status)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;
        let sql = format!(
            "SELECT {DELIVERY_COLUMNS} FROM webhook_deliveries \
             WHERE endpoint_id = ? AND (? IS NULL OR status = ?) \
             ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?"
        );
        let items = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(endpoint_id.to_string())
            .bind(status)
            .bind(status)
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(delivery_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((items, total.max(0) as u64))
    }

    pub(crate) async fn record_attempt(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        attempt: &DeliveryAttempt,
    ) -> Result<(), AppError> {
        let completed_at = (attempt.status != DeliveryStatus::Pending)
            .then(|| mapper::naive(attempt.attempted_at));
        sqlx::query(
            "UPDATE webhook_deliveries SET status = ?, attempts = ?, last_attempt_at = ?, next_attempt_at = ?, \
             response_status = ?, response_body = ?, duration_ms = ?, error = ?, completed_at = ? \
             WHERE id = ?",
        )
        .bind(attempt.status.as_str())
        .bind(attempt.attempts)
        .bind(mapper::naive(attempt.attempted_at))
        .bind(attempt.next_attempt_at.map(mapper::naive))
        .bind(attempt.response_status)
        .bind(&attempt.response_body)
        .bind(attempt.duration_ms)
        .bind(&attempt.error)
        .bind(completed_at)
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn stalled(
        &self,
        older_than: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, AppError> {
        let sql = format!(
            "SELECT {DELIVERY_COLUMNS} FROM webhook_deliveries \
             WHERE status = 'pending' AND next_attempt_at < ? ORDER BY next_attempt_at LIMIT ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(mapper::naive(older_than))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(delivery_from_row)
            .collect()
    }

    pub(crate) async fn purge(&self, before: DateTime<Utc>, limit: u32) -> Result<u64, AppError> {
        let result = sqlx::query(
            "DELETE FROM webhook_deliveries WHERE created_at < ? AND status <> 'pending' LIMIT ?",
        )
        .bind(mapper::naive(before))
        .bind(limit)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
