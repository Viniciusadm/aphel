use aphel_sdk::calendar::{
    CalendarBatch, CalendarCategory, CalendarOwner, CalendarQuery, CalendarRecord, CalendarSource,
};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};

use crate::model::{BookingFilter, BookingStatus};
use crate::store::BookingStore;
use aphel_contract_crm::CrmPermissions;

const KEY: &str = "booking";
const LIMIT: u32 = 500;

pub(crate) struct BookingCalendar;

fn category() -> CalendarCategory {
    CalendarCategory {
        source: KEY.into(),
        id: KEY.into(),
        name: "Booking".into(),
        icon: None,
        color: Some("#0f766e".into()),
    }
}

fn instant(text: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(text)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::Validation("invalid calendar range".into()))
}

#[async_trait]
impl CalendarSource for BookingCalendar {
    fn key(&self) -> &'static str {
        KEY
    }

    async fn categories(&self, ctx: &Ctx<'_>) -> Result<Vec<CalendarCategory>, AppError> {
        if ctx
            .scope
            .record_filter(&Permission::BOOKING_MANAGE)
            .is_err()
        {
            return Ok(Vec::new());
        }
        Ok(vec![category()])
    }

    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        query: &CalendarQuery,
    ) -> Result<CalendarBatch, AppError> {
        let Ok(hosts) = ctx.scope.record_filter(&Permission::BOOKING_MANAGE) else {
            return Ok(CalendarBatch::default());
        };
        if query.team_id.is_some() || query.category_id.as_deref().is_some_and(|id| id != KEY) {
            return Ok(CalendarBatch::default());
        }
        if query
            .status
            .as_deref()
            .is_some_and(|status| status != BookingStatus::Confirmed.as_str())
        {
            return Ok(CalendarBatch::default());
        }
        let listing = BookingStore::of(ctx)
            .list_bookings(&BookingFilter {
                hosts,
                status: Some(BookingStatus::Confirmed),
                meeting_type_id: None,
                after: None,
                between: Some((instant(&query.from)?, instant(&query.to)?)),
                without_activity: true,
                window: aphel_sdk::platform::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: u64::from(LIMIT),
                },
            })
            .await?;
        let truncated = listing.total > u64::from(LIMIT);
        let total = listing.total;
        let items = listing
            .items
            .into_iter()
            .filter(|booking| {
                query
                    .owner_id
                    .as_deref()
                    .is_none_or(|owner| booking.host_user_id.to_string() == owner)
            })
            .map(|booking| {
                let id = booking.id.to_string();
                CalendarRecord {
                    id: format!("{KEY}:{id}"),
                    source: KEY.into(),
                    record_id: id,
                    title: format!("{} · {}", booking.meeting_type_name, booking.guest_name),
                    due_at: booking.starts_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                    duration_minutes: Some((booking.ends_at - booking.starts_at).num_minutes()),
                    status: booking.status.as_str().into(),
                    finished: false,
                    category: category(),
                    owner: Some(CalendarOwner {
                        id: booking.host_user_id.to_string(),
                        display_name: booking.host_name,
                    }),
                    href: "/booking".into(),
                    updated_at: booking
                        .updated_at
                        .to_rfc3339_opts(SecondsFormat::Secs, true),
                    can_update: false,
                }
            })
            .collect();
        Ok(CalendarBatch {
            items,
            total,
            truncated,
        })
    }

    async fn reschedule(
        &self,
        _ctx: &Ctx<'_>,
        _id: &str,
        _due_at: &str,
        _expected_updated_at: Option<DateTime<Utc>>,
        _ip: Option<String>,
    ) -> Result<(), AppError> {
        Err(AppError::Validation(
            "bookings cannot be rescheduled from the calendar".into(),
        ))
    }
}
