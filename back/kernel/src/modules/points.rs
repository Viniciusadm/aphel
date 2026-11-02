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
