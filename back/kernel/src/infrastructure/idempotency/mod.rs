use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row};

use crate::application::idempotency::{
    BeginOutcome, IdempotencyPurgeHandler, IdempotencyStore, IdempotentRequest, StoredResponse,
};
use crate::application::shared::JobHandler;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const PURGE_BATCH_SIZE: u32 = 5000;
const BEGIN_ATTEMPTS: usize = 3;

#[derive(Clone)]
pub(crate) struct MySqlIdempotencyStore {
    pool: MySqlPool,
}

impl MySqlIdempotencyStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn insert(
        &self,
        request: &IdempotentRequest,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT INTO idempotency_keys \
             (scope_hash, idempotency_key, request_hash, started_at, expires_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&request.scope_hash)
        .bind(request.key.as_str())
        .bind(&request.request_hash)
        .bind(mapper::naive(now))
        .bind(mapper::naive(expires_at))
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(true),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    async fn reclaim(
        &self,
        request: &IdempotentRequest,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        stale_before: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let affected = sqlx::query(
            "UPDATE idempotency_keys SET request_hash = ?, response_status = NULL, \
             response_content_type = NULL, response_body = NULL, started_at = ?, \
             completed_at = NULL, expires_at = ? \
             WHERE scope_hash = ? AND idempotency_key = ? \
             AND (expires_at <= ? \
                  OR (response_status IS NULL AND started_at <= ? AND request_hash = ?))",
        )
        .bind(&request.request_hash)
        .bind(mapper::naive(now))
        .bind(mapper::naive(expires_at))
        .bind(&request.scope_hash)
        .bind(request.key.as_str())
        .bind(mapper::naive(now))
        .bind(mapper::naive(stale_before))
        .bind(&request.request_hash)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(affected == 1)
    }

    async fn existing(
        &self,
        request: &IdempotentRequest,
    ) -> Result<Option<BeginOutcome>, AppError> {
        let row = sqlx::query(
            "SELECT request_hash, response_status, response_content_type, response_body \
             FROM idempotency_keys WHERE scope_hash = ? AND idempotency_key = ?",
        )
        .bind(&request.scope_hash)
        .bind(request.key.as_str())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let request_hash: String = row.try_get("request_hash")?;
        if request_hash != request.request_hash {
            return Ok(Some(BeginOutcome::Mismatch));
        }
        let status: Option<u16> = row.try_get("response_status")?;
        Ok(Some(match status {
            None => BeginOutcome::InProgress,
            Some(status) => BeginOutcome::Replay(StoredResponse {
                status,
                content_type: row.try_get("response_content_type")?,
                body: row
                    .try_get::<Option<Vec<u8>>, _>("response_body")?
                    .unwrap_or_default(),
            }),
        }))
    }
}

#[async_trait]
impl IdempotencyStore for MySqlIdempotencyStore {
    async fn begin(
        &self,
        request: &IdempotentRequest,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        stale_before: DateTime<Utc>,
    ) -> Result<BeginOutcome, AppError> {
        for _ in 0..BEGIN_ATTEMPTS {
            if self.insert(request, now, expires_at).await? {
                return Ok(BeginOutcome::Started);
            }
            if self.reclaim(request, now, expires_at, stale_before).await? {
                return Ok(BeginOutcome::Started);
            }
            if let Some(outcome) = self.existing(request).await? {
                return Ok(outcome);
            }
        }
        Ok(BeginOutcome::InProgress)
    }

    async fn complete(
        &self,
        request: &IdempotentRequest,
        response: &StoredResponse,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE idempotency_keys SET response_status = ?, response_content_type = ?, \
             response_body = ?, completed_at = ? \
             WHERE scope_hash = ? AND idempotency_key = ? AND request_hash = ? \
             AND response_status IS NULL",
        )
        .bind(response.status)
        .bind(&response.content_type)
        .bind(&response.body)
        .bind(mapper::naive(now))
        .bind(&request.scope_hash)
        .bind(request.key.as_str())
        .bind(&request.request_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn release(&self, request: &IdempotentRequest) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM idempotency_keys \
             WHERE scope_hash = ? AND idempotency_key = ? AND request_hash = ? \
             AND response_status IS NULL",
        )
        .bind(&request.scope_hash)
        .bind(request.key.as_str())
        .bind(&request.request_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, AppError> {
        let mut purged = 0;
        loop {
            let affected =
                sqlx::query("DELETE FROM idempotency_keys WHERE expires_at <= ? LIMIT ?")
                    .bind(mapper::naive(now))
                    .bind(PURGE_BATCH_SIZE)
                    .execute(&self.pool)
                    .await?
                    .rows_affected();
            purged += affected;
            if affected < u64::from(PURGE_BATCH_SIZE) {
                return Ok(purged);
            }
        }
    }
}

pub(crate) fn idempotency_purge_handler(pool: MySqlPool) -> Arc<dyn JobHandler> {
    Arc::new(IdempotencyPurgeHandler::new(MySqlIdempotencyStore::new(
        pool,
    )))
}
