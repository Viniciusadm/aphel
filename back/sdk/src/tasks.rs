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
