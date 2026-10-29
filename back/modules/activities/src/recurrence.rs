use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::domain::{ActivityData, ActivityPriority, ActivityStatus};
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::FieldViolation;

pub(crate) const MAX_RECURRENCE_INTERVAL: u32 = 365;
pub(crate) const MAX_RECURRENCE_COUNT: u32 = 1000;
const MAX_SCANNED_PERIODS: i64 = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFrequency {
    pub(crate) const ALL: [Self; 4] = [Self::Daily, Self::Weekly, Self::Monthly, Self::Yearly];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|frequency| frequency.as_str() == value)
    }

    fn rrule_name(&self) -> &'static str {
        match self {
            Self::Daily => "DAILY",
            Self::Weekly => "WEEKLY",
            Self::Monthly => "MONTHLY",
            Self::Yearly => "YEARLY",
        }
    }
}

pub(crate) fn weekday_code(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "mo",
        Weekday::Tue => "tu",
        Weekday::Wed => "we",
        Weekday::Thu => "th",
        Weekday::Fri => "fr",
        Weekday::Sat => "sa",
        Weekday::Sun => "su",
    }
}

pub(crate) fn parse_weekday_code(value: &str) -> Option<Weekday> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mo" => Some(Weekday::Mon),
        "tu" => Some(Weekday::Tue),
        "we" => Some(Weekday::Wed),
        "th" => Some(Weekday::Thu),
        "fr" => Some(Weekday::Fri),
        "sa" => Some(Weekday::Sat),
        "su" => Some(Weekday::Sun),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecurrenceRuleDraft {
    pub(crate) frequency: RecurrenceFrequency,
    pub(crate) interval: Option<i64>,
    pub(crate) weekdays: Vec<Weekday>,
    pub(crate) month_day: Option<i64>,
    pub(crate) until: Option<NaiveDate>,
    pub(crate) count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecurrenceViolation {
    pub(crate) field: &'static str,
    pub(crate) message: &'static str,
}

impl RecurrenceViolation {
    fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecurrenceRule {
    pub(crate) frequency: RecurrenceFrequency,
    pub(crate) interval: u32,
    pub(crate) weekdays: Vec<Weekday>,
    pub(crate) month_day: Option<u32>,
    pub(crate) until: Option<NaiveDate>,
    pub(crate) count: Option<u32>,
}

impl RecurrenceRule {
    pub(crate) fn build(
        draft: RecurrenceRuleDraft,
        anchor: NaiveDateTime,
    ) -> Result<Self, Vec<RecurrenceViolation>> {
        let mut violations = Vec::new();
        let interval = match draft.interval {
            None => 1,
            Some(value) => match u32::try_from(value) {
                Ok(value) if (1..=MAX_RECURRENCE_INTERVAL).contains(&value) => value,
                _ => {
                    violations.push(RecurrenceViolation::new(
                        "interval",
                        "must be an integer between 1 and 365",
                    ));
                    1
                }
            },
        };
        let mut weekdays = Vec::new();
        if draft.frequency == RecurrenceFrequency::Weekly {
            weekdays = draft.weekdays.clone();
            if weekdays.is_empty() {
                weekdays.push(anchor.weekday());
            }
            weekdays.sort_by_key(Weekday::num_days_from_monday);
            weekdays.dedup();
        } else if !draft.weekdays.is_empty() {
            violations.push(RecurrenceViolation::new(
                "by_weekday",
                "is only allowed for weekly recurrences",
            ));
        }
        let month_day = match (draft.frequency, draft.month_day) {
            (RecurrenceFrequency::Monthly, None) => Some(anchor.day()),
            (RecurrenceFrequency::Monthly, Some(value)) => match u32::try_from(value) {
                Ok(value) if (1..=31).contains(&value) => Some(value),
                _ => {
                    violations.push(RecurrenceViolation::new(
                        "month_day",
                        "must be an integer between 1 and 31",
                    ));
                    None
                }
            },
            (_, Some(_)) => {
                violations.push(RecurrenceViolation::new(
                    "month_day",
                    "is only allowed for monthly recurrences",
                ));
                None
            }
            (_, None) => None,
        };
        let count = match draft.count {
            None => None,
            Some(value) => match u32::try_from(value) {
                Ok(value) if (1..=MAX_RECURRENCE_COUNT).contains(&value) => Some(value),
                _ => {
                    violations.push(RecurrenceViolation::new(
                        "count",
                        "must be an integer between 1 and 1000",
                    ));
                    None
                }
            },
        };
        if draft.until.is_some() && draft.count.is_some() {
            violations.push(RecurrenceViolation::new(
                "until",
                "must not be sent together with count",
            ));
        }
        if let Some(until) = draft.until
            && until < anchor.date()
        {
            violations.push(RecurrenceViolation::new(
                "until",
                "must not be before the due date",
            ));
        }
        if !violations.is_empty() {
            return Err(violations);
        }
        Ok(Self {
            frequency: draft.frequency,
            interval,
            weekdays,
            month_day,
            until: draft.until,
            count,
        })
    }

    pub(crate) fn allows_index(&self, index: u32) -> bool {
        self.count.is_none_or(|count| index <= count)
    }

    pub(crate) fn next_after(
        &self,
        anchor: NaiveDateTime,
        after: NaiveDateTime,
        not_before: Option<NaiveDateTime>,
    ) -> Option<NaiveDateTime> {
        let threshold = match not_before {
            Some(limit) if limit > after => limit,
            _ => after,
        };
        let start = (self.estimated_period(anchor, threshold) - 1).max(0);
        for period in start..start + MAX_SCANNED_PERIODS {
            for slot in self.period_slots(anchor, period) {
                if slot < anchor || slot <= after {
                    continue;
                }
                if not_before.is_some_and(|limit| slot < limit) {
                    continue;
                }
                if self.until.is_some_and(|until| slot.date() > until) {
                    return None;
                }
                return Some(slot);
            }
            if let Some(until) = self.until
                && self
                    .period_slots(anchor, period)
                    .first()
                    .is_some_and(|slot| slot.date() > until)
            {
                return None;
            }
        }
        None
    }

    pub(crate) fn to_rrule(&self) -> String {
        let mut parts = vec![
            format!("FREQ={}", self.frequency.rrule_name()),
            format!("INTERVAL={}", self.interval),
        ];
        if !self.weekdays.is_empty() {
            parts.push(format!(
                "BYDAY={}",
                self.weekdays
                    .iter()
                    .map(|weekday| weekday_code(*weekday).to_ascii_uppercase())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(day) = self.month_day {
            parts.push(format!("BYMONTHDAY={day}"));
        }
        if let Some(until) = self.until {
            parts.push(format!("UNTIL={}", until.format("%Y%m%d")));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={count}"));
        }
        parts.join(";")
    }

    fn estimated_period(&self, anchor: NaiveDateTime, threshold: NaiveDateTime) -> i64 {
        let interval = i64::from(self.interval);
        let days = (threshold.date() - anchor.date()).num_days();
        if days <= 0 {
            return 0;
        }
        match self.frequency {
            RecurrenceFrequency::Daily => days / interval,
            RecurrenceFrequency::Weekly => days / (7 * interval),
            RecurrenceFrequency::Monthly => {
                months_between(anchor.date(), threshold.date()) / interval
            }
            RecurrenceFrequency::Yearly => i64::from(threshold.year() - anchor.year()) / interval,
        }
    }

    fn period_slots(&self, anchor: NaiveDateTime, period: i64) -> Vec<NaiveDateTime> {
        let time = anchor.time();
        let step = period * i64::from(self.interval);
        match self.frequency {
            RecurrenceFrequency::Daily => anchor
                .date()
                .checked_add_signed(Duration::days(step))
                .map(|date| vec![date.and_time(time)])
                .unwrap_or_default(),
            RecurrenceFrequency::Weekly => {
                let week_start = anchor.date()
                    - Duration::days(i64::from(anchor.weekday().num_days_from_monday()));
                let Some(base) = week_start.checked_add_signed(Duration::weeks(step)) else {
                    return Vec::new();
                };
                self.weekdays
                    .iter()
                    .map(|weekday| {
                        (base + Duration::days(i64::from(weekday.num_days_from_monday())))
                            .and_time(time)
                    })
                    .collect()
            }
            RecurrenceFrequency::Monthly => {
                let total = i64::from(anchor.year()) * 12 + i64::from(anchor.month0()) + step;
                let day = self.month_day.unwrap_or(anchor.day());
                clamped_date(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, day)
                    .map(|date| vec![date.and_time(time)])
                    .unwrap_or_default()
            }
            RecurrenceFrequency::Yearly => clamped_date(
                i64::from(anchor.year()) + step,
                anchor.month(),
                anchor.day(),
            )
            .map(|date| vec![date.and_time(time)])
            .unwrap_or_default(),
        }
    }
}

fn months_between(from: NaiveDate, to: NaiveDate) -> i64 {
    (i64::from(to.year()) - i64::from(from.year())) * 12 + i64::from(to.month0())
        - i64::from(from.month0())
}

pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|first| first.pred_opt())
        .map(|last| last.day())
        .unwrap_or(28)
}

fn clamped_date(year: i64, month: u32, day: u32) -> Option<NaiveDate> {
    let year = i32::try_from(year).ok()?;
    NaiveDate::from_ymd_opt(year, month, day.min(days_in_month(year, month)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OccurrenceRef {
    pub(crate) series_id: Uuid,
    pub(crate) index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityRecurrenceView {
    pub(crate) series_id: Uuid,
    pub(crate) index: u32,
    pub(crate) rule: RecurrenceRule,
    pub(crate) ended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRuleInput {
    pub(crate) frequency: RecurrenceFrequency,
    pub(crate) interval: Option<i64>,
    pub(crate) by_weekday: Vec<Weekday>,
    pub(crate) month_day: Option<i64>,
    pub(crate) until: Option<String>,
    pub(crate) count: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApplyTo {
    #[default]
    This,
    Following,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityTemplate {
    pub(crate) title: String,
    pub(crate) activity_type_id: Uuid,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) priority: ActivityPriority,
    pub(crate) description: Option<String>,
    pub(crate) duration_minutes: Option<u32>,
    pub(crate) reminder_minutes_before: Option<u32>,
    pub(crate) record: Option<RecordRef>,
    pub(crate) links: Vec<RecordRef>,
}

impl ActivityTemplate {
    pub(crate) fn from_data(data: &ActivityData) -> Self {
        Self {
            title: data.title.clone(),
            activity_type_id: data.activity_type_id,
            owner_id: data.owner_id.clone(),
            priority: data.priority,
            description: data.description.clone(),
            duration_minutes: data.duration_minutes,
            reminder_minutes_before: data.reminder_minutes_before,
            record: data.record.clone(),
            links: data.links.clone(),
        }
    }

    pub(crate) fn occurrence(
        &self,
        due_at: DateTime<Utc>,
        occurrence: OccurrenceRef,
    ) -> ActivityData {
        ActivityData {
            title: self.title.clone(),
            activity_type_id: self.activity_type_id,
            status: ActivityStatus::Open,
            priority: self.priority,
            due_at: Some(due_at),
            completed_at: None,
            owner_id: self.owner_id.clone(),
            record: self.record.clone(),
            links: self.links.clone(),
            duration_minutes: self.duration_minutes,
            description: self.description.clone(),
            location: None,
            reminder_minutes_before: self.reminder_minutes_before,
            occurrence: Some(occurrence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecurrenceSeries {
    pub(crate) id: Uuid,
    pub(crate) rule: RecurrenceRule,
    pub(crate) anchor: NaiveDateTime,
    pub(crate) template: ActivityTemplate,
    pub(crate) ended_at: Option<DateTime<Utc>>,
}

impl RecurrenceSeries {
    pub(crate) fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }

    pub(crate) fn next_due(
        &self,
        timezone: Tz,
        current_due: DateTime<Utc>,
        next_index: u32,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        if !self.is_active() || !self.rule.allows_index(next_index) {
            return None;
        }
        let after = to_local(timezone, current_due);
        let today = to_local(timezone, now).date().and_hms_opt(0, 0, 0);
        self.rule
            .next_after(self.anchor, after, today)
            .map(|slot| from_local(timezone, slot))
    }
}

pub(crate) fn to_local(timezone: Tz, instant: DateTime<Utc>) -> NaiveDateTime {
    instant.with_timezone(&timezone).naive_local()
}

pub(crate) fn from_local(timezone: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    (0..4)
        .filter_map(|hours| {
            timezone
                .from_local_datetime(&(local + Duration::hours(hours)))
                .earliest()
        })
        .map(|instant| instant.with_timezone(&Utc))
        .next()
        .unwrap_or_else(|| Utc.from_utc_datetime(&local))
}

pub(crate) fn parse_rule(
    input: &RecurrenceRuleInput,
    anchor: NaiveDateTime,
    violations: &mut Vec<FieldViolation>,
) -> Option<RecurrenceRule> {
    let before = violations.len();
    let until = match input.until.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => match NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            Ok(date) => Some(date),
            Err(_) => {
                violations.push(FieldViolation::new(
                    "recurrence.until",
                    "must be a date such as 2026-12-31",
                ));
                None
            }
        },
    };
    if violations.len() > before {
        return None;
    }
    let draft = RecurrenceRuleDraft {
        frequency: input.frequency,
        interval: input.interval,
        weekdays: input.by_weekday.clone(),
        month_day: input.month_day,
        until,
        count: input.count,
    };
    match RecurrenceRule::build(draft, anchor) {
        Ok(rule) => Some(rule),
        Err(errors) => {
            violations.extend(errors.into_iter().map(|error| {
                FieldViolation::new(format!("recurrence.{}", error.field), error.message)
            }));
            None
        }
    }
}
