pub mod remote_calendar;
use crate::AppError;
pub use aphel_contract_tasks::*;
use aphel_kernel::domain::UserId;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueTaskReminder {
    pub id: Uuid,
    pub owner_id: UserId,
    pub title: String,
    pub due_at: DateTime<Utc>,
    pub reminder_at: DateTime<Utc>,
    pub minutes_before: u32,
}

#[async_trait::async_trait]
pub trait TaskReminderSource: Send + Sync {
    async fn due(
        &self,
        pool: &MySqlPool,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DueTaskReminder>, AppError>;
    async fn mark_sent(
        &self,
        pool: &MySqlPool,
        reminder: &DueTaskReminder,
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

use serde_json::{Map, Value};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordLinkInput {
    pub record_type: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityLocationInput {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    pub captured_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreateActivityCommand {
    pub title: String,
    pub activity_type_id: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_at: Option<String>,
    pub completed_at: Option<String>,
    pub owner_id: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub links: Vec<RecordLinkInput>,
    pub duration_minutes: Option<i64>,
    pub description: Option<String>,
    pub location: Option<ActivityLocationInput>,
    pub custom_fields: Map<String, Value>,
    pub ip: Option<String>,
}

#[async_trait::async_trait]
pub trait TaskWriter: Send + Sync {
    async fn create(
        &self,
        ctx: &crate::Ctx<'_>,
        command: CreateActivityCommand,
    ) -> Result<Uuid, AppError>;
}

pub fn writer() -> Result<std::sync::Arc<dyn TaskWriter>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn TaskWriter>()
        .ok_or_else(|| {
            AppError::unavailable_code("task_writer_unavailable", "task creation is not enabled")
        })
}
