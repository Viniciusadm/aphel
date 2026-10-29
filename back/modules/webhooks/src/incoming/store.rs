use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::incoming::{
    ActionOutcome, IncomingRequestLog, IncomingWebhook, RequestStatus, WebhookAction,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

const WEBHOOK_COLUMNS: &str = "w.id, w.name, w.description, w.token_prefix, \
     w.is_active, w.signature_required, w.signature_tolerance_seconds, w.actions, w.created_by, \
     u.display_name AS created_by_name, w.last_received_at, w.created_at, w.updated_at";

const REQUEST_SUMMARY_COLUMNS: &str = "id, webhook_id, received_at, ip, content_type, body_bytes, \
     body_truncated, status, http_status, error_code, error_message, results, duration_ms";

const PURGE_BATCH_SIZE: u32 = 1000;

#[derive(Clone)]
pub(crate) struct IncomingWebhookStore {
    pool: MySqlPool,
}

fn uuid(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row
        .try_get::<Option<NaiveDateTime>, _>(column)?
        .map(mapper::utc))
}

fn webhook_from_row(row: &MySqlRow) -> Result<IncomingWebhook, AppError> {
    let Json(actions): Json<Value> = row.try_get("actions")?;
    let actions: Vec<WebhookAction> = serde_json::from_value(actions.clone())
        .map_err(|_| mapper::invalid_column("incoming_webhooks.actions", &actions.to_string()))?;
    let tolerance: u32 = row.try_get("signature_tolerance_seconds")?;
    Ok(IncomingWebhook {
        id: uuid(row, "id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        token_prefix: row.try_get("token_prefix")?,
        is_active: row.try_get("is_active")?,
        signature_required: row.try_get("signature_required")?,
        signature_tolerance_seconds: tolerance,
        actions,
        created_by: mapper::typed_id(row.try_get("created_by")?, UserId::new)?,
        created_by_name: row.try_get("created_by_name")?,
        last_received_at: optional_instant(row, "last_received_at")?,
        created_at: mapper::utc(row.try_get("created_at")?),
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

fn request_from_row(row: &MySqlRow, detailed: bool) -> Result<IncomingRequestLog, AppError> {
    let status: String = row.try_get("status")?;
    let Json(results): Json<Vec<ActionOutcome>> = row.try_get("results")?;
    let (headers, body) = if detailed {
        let Json(headers): Json<Map<String, Value>> = row.try_get("headers")?;
        (headers, row.try_get::<Option<String>, _>("body")?)
    } else {
        (Map::new(), None)
    };
    let body_bytes: u32 = row.try_get("body_bytes")?;
    let http_status: u16 = row.try_get("http_status")?;
    let duration_ms: u32 = row.try_get("duration_ms")?;
    Ok(IncomingRequestLog {
        id: uuid(row, "id")?,
        webhook_id: uuid(row, "webhook_id")?,
        received_at: mapper::utc(row.try_get("received_at")?),
        ip: row.try_get("ip")?,
        content_type: row.try_get("content_type")?,
        headers,
        body,
        body_bytes: u64::from(body_bytes),
        body_truncated: row.try_get("body_truncated")?,
        status: RequestStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("incoming_webhook_requests.status", &status))?,
        http_status,
        error_code: row.try_get("error_code")?,
        error_message: row.try_get("error_message")?,
        results,
        duration_ms: u64::from(duration_ms),
    })
}

fn actions_json(webhook: &IncomingWebhook) -> Result<Json<Value>, AppError> {
    serde_json::to_value(&webhook.actions)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("could not encode webhook actions: {error}")))
}

impl IncomingWebhookStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        webhook: &IncomingWebhook,
        token_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO incoming_webhooks (id, name, description, token_prefix, token_hash, \
             is_active, signature_required, signature_tolerance_seconds, actions, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(webhook.id.to_string())
        .bind(&webhook.name)
        .bind(&webhook.description)
        .bind(&webhook.token_prefix)
        .bind(token_hash)
        .bind(webhook.is_active)
        .bind(webhook.signature_required)
        .bind(webhook.signature_tolerance_seconds)
        .bind(actions_json(webhook)?)
        .bind(webhook.created_by.as_str())
        .bind(mapper::naive(webhook.created_at))
        .bind(mapper::naive(webhook.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        webhook: &IncomingWebhook,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE incoming_webhooks SET name = ?, description = ?, is_active = ?, signature_required = ?, \
             signature_tolerance_seconds = ?, actions = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&webhook.name)
        .bind(&webhook.description)
        .bind(webhook.is_active)
        .bind(webhook.signature_required)
        .bind(webhook.signature_tolerance_seconds)
        .bind(actions_json(webhook)?)
        .bind(mapper::naive(webhook.updated_at))
        .bind(webhook.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn replace_token(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        token_prefix: &str,
        token_hash: &str,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE incoming_webhooks SET token_prefix = ?, token_hash = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(token_prefix)
        .bind(token_hash)
        .bind(mapper::naive(at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM incoming_webhooks WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn list(&self) -> Result<Vec<IncomingWebhook>, AppError> {
        let sql = format!(
            "SELECT {WEBHOOK_COLUMNS} FROM incoming_webhooks w LEFT JOIN users u ON u.id = w.created_by \
             ORDER BY w.created_at DESC, w.id DESC"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(webhook_from_row)
            .collect()
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<IncomingWebhook>, AppError> {
        let sql = format!(
            "SELECT {WEBHOOK_COLUMNS} FROM incoming_webhooks w LEFT JOIN users u ON u.id = w.created_by \
             WHERE w.id = ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(webhook_from_row)
            .transpose()
    }

    pub(crate) async fn find_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<IncomingWebhook>, AppError> {
        let sql = format!(
            "SELECT {WEBHOOK_COLUMNS} FROM incoming_webhooks w LEFT JOIN users u ON u.id = w.created_by \
             WHERE w.token_hash = ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(webhook_from_row)
            .transpose()
    }

    pub(crate) async fn touch_received(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE incoming_webhooks SET last_received_at = ? WHERE id = ?")
            .bind(mapper::naive(at))
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn remember_delivery(
        &self,
        webhook_id: Uuid,
        replay_key: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO incoming_webhook_replays (webhook_id, replay_key, seen_at) \
             VALUES (?, ?, ?)",
        )
        .bind(webhook_id.to_string())
        .bind(replay_key)
        .bind(mapper::naive(at))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn forget_delivery(
        &self,
        webhook_id: Uuid,
        replay_key: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM incoming_webhook_replays WHERE webhook_id = ? AND replay_key = ?")
            .bind(webhook_id.to_string())
            .bind(replay_key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_request(&self, log: &IncomingRequestLog) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO incoming_webhook_requests (id, webhook_id, received_at, ip, content_type, \
             headers, body, body_bytes, body_truncated, status, http_status, error_code, error_message, results, duration_ms) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(log.id.to_string())
        .bind(log.webhook_id.to_string())
        .bind(mapper::naive(log.received_at))
        .bind(&log.ip)
        .bind(&log.content_type)
        .bind(Json(&log.headers))
        .bind(&log.body)
        .bind(u32::try_from(log.body_bytes).unwrap_or(u32::MAX))
        .bind(log.body_truncated)
        .bind(log.status.as_str())
        .bind(log.http_status)
        .bind(&log.error_code)
        .bind(&log.error_message)
        .bind(Json(&log.results))
        .bind(u32::try_from(log.duration_ms).unwrap_or(u32::MAX))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn list_requests(
        &self,
        webhook_id: Uuid,
        status: Option<RequestStatus>,
        window: ListWindow,
    ) -> Result<Listing<IncomingRequestLog>, AppError> {
        let status = status.map(|status| status.as_str());
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM incoming_webhook_requests WHERE webhook_id = ? \
             AND (? IS NULL OR status = ?)",
        )
        .bind(webhook_id.to_string())
        .bind(status)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;
        let sql = format!(
            "SELECT {REQUEST_SUMMARY_COLUMNS} FROM incoming_webhook_requests \
             WHERE webhook_id = ? AND (? IS NULL OR status = ?) \
             ORDER BY received_at DESC, id DESC LIMIT ? OFFSET ?"
        );
        let items = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(webhook_id.to_string())
            .bind(status)
            .bind(status)
            .bind(window.limit)
            .bind(window.offset)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| request_from_row(row, false))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Listing {
            items,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn find_request(
        &self,
        webhook_id: Uuid,
        request_id: Uuid,
    ) -> Result<Option<IncomingRequestLog>, AppError> {
        let sql = format!(
            "SELECT {REQUEST_SUMMARY_COLUMNS}, headers, body FROM incoming_webhook_requests \
             WHERE webhook_id = ? AND id = ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(webhook_id.to_string())
            .bind(request_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(|row| request_from_row(row, true))
            .transpose()
    }

    pub(crate) async fn purge_replays(&self, before: DateTime<Utc>) -> Result<u64, AppError> {
        purge_batches(
            &self.pool,
            "DELETE FROM incoming_webhook_replays WHERE seen_at < ? LIMIT ?",
            before,
        )
        .await
    }

    pub(crate) async fn purge_requests(&self, before: DateTime<Utc>) -> Result<u64, AppError> {
        purge_batches(
            &self.pool,
            "DELETE FROM incoming_webhook_requests WHERE received_at < ? LIMIT ?",
            before,
        )
        .await
    }
}

async fn purge_batches(
    pool: &MySqlPool,
    sql: &'static str,
    before: DateTime<Utc>,
) -> Result<u64, AppError> {
    let mut purged = 0;
    loop {
        let affected = sqlx::query(sql)
            .bind(mapper::naive(before))
            .bind(PURGE_BATCH_SIZE)
            .execute(pool)
            .await?
            .rows_affected();
        purged += affected;
        if affected < u64::from(PURGE_BATCH_SIZE) {
            return Ok(purged);
        }
    }
}
