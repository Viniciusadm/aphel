use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::MySqlPool;

use crate::application::files::GeneratedFileKind;
use crate::application::integrations::IntegrationError;
use crate::application::notifications::DueTask;
use crate::application::system::FailureEntry;
use crate::domain::UserId;
use crate::errors::AppError;

#[async_trait]
pub trait FileReferenceSource: Send + Sync {
    async fn reference_kind(
        &self,
        pool: &MySqlPool,
        file_id: &str,
    ) -> Result<Option<GeneratedFileKind>, AppError>;
}

#[async_trait]
pub trait HealthSource: Send + Sync {
    async fn recent_failures(
        &self,
        pool: &MySqlPool,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError>;
}

#[async_trait]
pub trait DigestSource: Send + Sync {
    async fn open_tasks_due(
        &self,
        pool: &MySqlPool,
        owner: Option<&UserId>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<DueTask>, AppError>;
}

#[async_trait]
pub trait ReferenceProvider: Send + Sync {
    fn kind(&self) -> aphel_contract::ReferenceKind;

    async fn exists(&self, ctx: &crate::Ctx<'_>, id: &str) -> Result<bool, AppError>;

    async fn stable_key(&self, ctx: &crate::Ctx<'_>, id: &str) -> Result<Option<String>, AppError> {
        if !self.exists(ctx, id).await? {
            return Ok(None);
        }
        Ok(sqlx::query_scalar(
            "SELECT `key` FROM config_item_keys WHERE item_type = ? AND item_id = ?",
        )
        .bind(self.kind().key())
        .bind(id)
        .fetch_optional(ctx.pool())
        .await?)
    }

    async fn resolve_key(
        &self,
        ctx: &crate::Ctx<'_>,
        key: &str,
    ) -> Result<Option<String>, AppError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT item_id FROM config_item_keys WHERE item_type = ? AND `key` = ?",
        )
        .bind(self.kind().key())
        .bind(key)
        .fetch_optional(ctx.pool())
        .await?;
        match id {
            Some(id) if self.exists(ctx, &id).await? => Ok(Some(id)),
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveIntegration {
    pub id: String,
    pub status: String,
    pub updated_at: NaiveDateTime,
    pub config: Value,
}

#[async_trait]
pub trait IntegrationStore: Send + Sync {
    async fn active(
        &self,
        pool: &MySqlPool,
        provider_key: &str,
    ) -> Result<Option<ActiveIntegration>, AppError>;

    async fn record_failure(
        &self,
        pool: &MySqlPool,
        integration_id: &str,
        error: &IntegrationError,
    ) -> Result<(), AppError>;

    async fn record_recovery(&self, pool: &MySqlPool, integration_id: &str)
    -> Result<(), AppError>;
}

pub fn lookup_exists<T>(result: Result<T, AppError>) -> Result<bool, AppError> {
    use crate::errors::CodedKind;
    match result {
        Ok(_) => Ok(true),
        Err(
            AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            },
        ) => Ok(false),
        Err(error) => Err(error),
    }
}
