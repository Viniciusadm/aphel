use aphel_sdk::calendar::BusyInterval;
use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::{Availability, Slot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotRules {
    pub(crate) duration_minutes: u32,
    pub(crate) interval_minutes: u32,
}

pub(crate) struct HostCalendar<'a> {
    pub(crate) availability: &'a Availability,
    pub(crate) busy: &'a [BusyInterval],
    pub(crate) booked: &'a [BusyInterval],
}

fn minutes(value: u32) -> Duration {
    Duration::minutes(i64::from(value))
}

fn local_date(availability: &Availability, instant: DateTime<Utc>) -> NaiveDate {
    instant.with_timezone(&availability.timezone).date_naive()
}

fn local_instant(
    availability: &Availability,
    date: NaiveDate,
    minute: u16,
) -> Option<DateTime<Utc>> {
    let base = date.and_time(NaiveTime::MIN) + Duration::minutes(i64::from(minute));
    availability
        .timezone
        .from_local_datetime(&base)
        .earliest()
        .map(|instant| instant.with_timezone(&Utc))
}

fn day_counts(calendar: &HostCalendar<'_>) -> HashMap<NaiveDate, u32> {
    let mut counts = HashMap::new();
    for booked in calendar.booked {
        *counts
            .entry(local_date(calendar.availability, booked.starts_at))
            .or_insert(0) += 1;
    }
    counts
}

fn overlaps(calendar: &HostCalendar<'_>, starts_at: DateTime<Utc>, ends_at: DateTime<Utc>) -> bool {
    let availability = calendar.availability;
    let guarded_start = starts_at - minutes(availability.buffer_before_minutes);
    let guarded_end = ends_at + minutes(availability.buffer_after_minutes);
    calendar
        .busy
        .iter()
        .chain(calendar.booked.iter())
        .any(|busy| guarded_start < busy.ends_at && guarded_end > busy.starts_at)
}

pub(crate) fn free_slots(
    calendar: &HostCalendar<'_>,
    rules: SlotRules,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<Slot> {
    let availability = calendar.availability;
    if rules.duration_minutes == 0 || rules.interval_minutes == 0 || from >= to {
        return Vec::new();
    }
    let earliest = (now + minutes(availability.min_notice_minutes)).max(from);
    let latest = (now + Duration::days(i64::from(availability.horizon_days))).min(to);
    if earliest >= latest {
        return Vec::new();
    }
    let counts = day_counts(calendar);
    let duration = minutes(rules.duration_minutes);
    let step = minutes(rules.interval_minutes);
    let first_day = local_date(availability, earliest)
        .pred_opt()
        .unwrap_or(NaiveDate::MIN);
    let last_day = local_date(availability, latest)
        .succ_opt()
        .unwrap_or(NaiveDate::MAX);
    let mut slots = BTreeSet::new();
    let mut day = first_day;
    while day <= last_day {
        let weekday = u8::try_from(day.weekday().number_from_monday()).unwrap_or(0);
        let full = availability
            .max_per_day
            .is_some_and(|max| counts.get(&day).copied().unwrap_or(0) >= max);
        if !full {
            for interval in availability
                .weekly
                .iter()
                .filter(|interval| interval.weekday == weekday)
            {
                let (Some(open), Some(close)) = (
                    local_instant(availability, day, interval.start_minute),
                    local_instant(availability, day, interval.end_minute),
                ) else {
                    continue;
                };
                let mut starts_at = open;
                while starts_at + duration <= close {
                    let ends_at = starts_at + duration;
                    if starts_at >= earliest
                        && starts_at < latest
                        && local_date(availability, starts_at) == day
                        && !overlaps(calendar, starts_at, ends_at)
                    {
                        slots.insert(Slot { starts_at, ends_at });
                    }
                    starts_at += step;
                }
            }
        }
        let Some(next) = day.succ_opt() else {
            break;
        };
        day = next;
    }
    slots.into_iter().collect()
}

pub(crate) fn merge_slots(groups: impl IntoIterator<Item = Vec<Slot>>) -> Vec<Slot> {
    groups
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
