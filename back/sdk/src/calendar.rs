use crate::AppError;
pub use aphel_contract_calendar::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[async_trait]
pub trait CalendarSource: Send + Sync {
    fn key(&self) -> &'static str;
    async fn categories(&self, ctx: &crate::Ctx<'_>) -> Result<Vec<CalendarCategory>, AppError>;
    async fn entries(
        &self,
        ctx: &crate::Ctx<'_>,
        query: &CalendarQuery,
    ) -> Result<CalendarBatch, AppError>;
    async fn reschedule(
        &self,
        ctx: &crate::Ctx<'_>,
        id: &str,
        due_at: &str,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait CalendarAvailability: Send + Sync {
    fn key(&self) -> &str;

    async fn busy(
        &self,
        ctx: &crate::Ctx<'_>,
        user_id: &aphel_kernel::domain::UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Option<Vec<BusyInterval>>, AppError>;
}
