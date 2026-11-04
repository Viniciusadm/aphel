use crate::AppError;
pub use aphel_contract_calendar::*;
use async_trait::async_trait;
use chrono::TimeZone;
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

pub fn local_midnight(timezone: chrono_tz::Tz, date: chrono::NaiveDate) -> DateTime<Utc> {
    (0..4)
        .filter_map(|hour| date.and_hms_opt(hour, 0, 0))
        .find_map(|naive| timezone.from_local_datetime(&naive).earliest())
        .map(|local| local.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default()))
}

pub fn local_day(
    timezone: chrono_tz::Tz,
    now: DateTime<Utc>,
) -> (chrono::NaiveDate, DateTime<Utc>, DateTime<Utc>) {
    let date = now.with_timezone(&timezone).date_naive();
    let start = local_midnight(timezone, date);
    let end = date
        .succ_opt()
        .map(|next| local_midnight(timezone, next))
        .unwrap_or(start + chrono::Duration::days(1));
    (date, start, end)
}
