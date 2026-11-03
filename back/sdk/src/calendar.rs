use crate::{AppError, Ctx};
pub use aphel_contract_calendar::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[async_trait]
pub trait CalendarSource: Send + Sync {
    fn key(&self) -> &'static str;
    async fn categories(&self, ctx: &Ctx<'_>) -> Result<Vec<CalendarCategory>, AppError>;
    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        query: &CalendarQuery,
    ) -> Result<CalendarBatch, AppError>;
    async fn reschedule(
        &self,
        ctx: &Ctx<'_>,
        id: &str,
        due_at: &str,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> Result<(), AppError>;
}
