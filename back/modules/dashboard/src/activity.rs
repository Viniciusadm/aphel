use chrono::{DateTime, Duration, Utc};

use crate::queries::{self, EventCursor, RecentEvent};
use crate::repository::MySqlDashboardPorts;
use crate::widgets::recent_events_request;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::dashboard::{ActivityScope, DashboardPorts};
use aphel_sdk::platform::application::shared::RecordRef;
use std::collections::HashSet;

pub(crate) const DEFAULT_ACTIVITY_PAGE_SIZE: u32 = 25;
pub(crate) const MAX_ACTIVITY_PAGE_SIZE: u32 = 50;
const MAX_DAYS: u32 = 365;
const BATCH: u32 = 100;
const MAX_BATCHES: usize = 8;

#[derive(Debug, Clone, Default)]
pub(crate) struct ActivityQuery {
    pub(crate) cursor: Option<String>,
    pub(crate) per_page: Option<u32>,
    pub(crate) days: Option<u32>,
    pub(crate) record_type: Option<String>,
    pub(crate) actor_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivityPage {
    pub(crate) items: Vec<RecentEvent>,
    pub(crate) next_cursor: Option<String>,
}

fn encode(cursor: &EventCursor) -> String {
    format!("{}:{}", cursor.occurred_at.timestamp_micros(), cursor.id)
}

fn decode(value: &str) -> Option<EventCursor> {
    let (micros, id) = value.split_once(':')?;
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }
    Some(EventCursor {
        occurred_at: DateTime::<Utc>::from_timestamp_micros(micros.parse().ok()?)?,
        id: id.to_owned(),
    })
}

fn cursor_of(event: &RecentEvent) -> EventCursor {
    EventCursor {
        occurred_at: event.occurred_at,
        id: event.id.to_string(),
    }
}

pub(crate) async fn activity(
    ctx: &Ctx<'_>,
    query: ActivityQuery,
) -> Result<ActivityPage, AppError> {
    let mut violations = Vec::new();
    let per_page = query.per_page.unwrap_or(DEFAULT_ACTIVITY_PAGE_SIZE);
    if !(1..=MAX_ACTIVITY_PAGE_SIZE).contains(&per_page) {
        violations.push(FieldViolation::new("per_page", "must be between 1 and 50"));
    }
    if query
        .days
        .is_some_and(|days| !(1..=MAX_DAYS).contains(&days))
    {
        violations.push(FieldViolation::new("days", "must be between 1 and 365"));
    }
    let cursor = match query.cursor.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => {
            let cursor = decode(value);
            if cursor.is_none() {
                violations.push(FieldViolation::new("cursor", "is not a valid cursor"));
            }
            cursor
        }
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid activity query",
            violations,
        ));
    }
    let scopes = ctx.app().contributions::<dyn ActivityScope>();
    let empty = ActivityPage {
        items: Vec::new(),
        next_cursor: None,
    };
    if scopes.is_empty() {
        return Ok(empty);
    }
    let ports = MySqlDashboardPorts::new(
        ctx.app().shared_services(),
        ctx.pool().clone(),
        ctx.query(),
        std::sync::Arc::new(crate::repository::MySqlDashboardRepository::new(
            ctx.pool().clone(),
        )),
    );
    let mut request = recent_events_request(ports.query(), ctx.scope, BATCH);
    if request.records.is_empty() {
        return Ok(empty);
    }
    request.since = query
        .days
        .map(|days| Utc::now() - Duration::days(i64::from(days)));
    request.record_types = query.record_type.map(|record_type| vec![record_type]);
    request.actor_ids = query.actor_id.map(|id| vec![id]);
    request.before = cursor;
    let mut items: Vec<RecentEvent> = Vec::new();
    let mut next = None;
    for _ in 0..MAX_BATCHES {
        let page =
            queries::recent_events_page(ports.pool(), ctx.scope, ports.query(), &request).await?;
        let refs: Vec<RecordRef> = page
            .events
            .iter()
            .filter_map(|event| {
                let record = event.record.as_ref()?;
                RecordRef::parse(&record.record_type, &record.record_id.to_string()).ok()
            })
            .collect();
        let mut followed = HashSet::new();
        for scope in &scopes {
            followed.extend(
                scope
                    .followed(ports.pool(), ctx.scope.user_id(), &refs)
                    .await?,
            );
        }
        for event in page.events {
            let keep = event.record.as_ref().is_some_and(|record| {
                RecordRef::parse(&record.record_type, &record.record_id.to_string())
                    .is_ok_and(|reference| followed.contains(&reference))
            });
            if keep {
                items.push(event);
            }
        }
        next = page.next.clone();
        if items.len() > per_page as usize {
            break;
        }
        if page.exhausted {
            next = None;
            break;
        }
        request.before = page.next;
    }
    if items.len() > per_page as usize {
        items.truncate(per_page as usize);
        next = items.last().map(cursor_of);
    }
    Ok(ActivityPage {
        items,
        next_cursor: next.as_ref().map(encode),
    })
}
