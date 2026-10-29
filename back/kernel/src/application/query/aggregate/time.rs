use chrono::{DateTime, Datelike, Days, Duration, Months, NaiveDate, Offset, TimeZone, Utc};
use chrono_tz::Tz;

use crate::application::query::{OffsetSegment, TimeGranularity, TimezoneShift};

pub const BUCKET_FORMAT: &str = "%Y-%m-%d";
const MAX_BUCKETS: usize = 5000;

fn offset_at(timezone: Tz, instant: DateTime<Utc>) -> i32 {
    timezone
        .offset_from_utc_datetime(&instant.naive_utc())
        .fix()
        .local_minus_utc()
}

fn transition(timezone: Tz, mut low: DateTime<Utc>, mut high: DateTime<Utc>) -> DateTime<Utc> {
    let initial = offset_at(timezone, low);
    while high - low > Duration::seconds(1) {
        let middle = low + (high - low) / 2;
        if offset_at(timezone, middle) == initial {
            low = middle;
        } else {
            high = middle;
        }
    }
    high
}

pub fn timezone_shift(timezone: Tz, from: DateTime<Utc>, until: DateTime<Utc>) -> TimezoneShift {
    let mut segments = Vec::new();
    let mut cursor = from;
    let mut current = offset_at(timezone, cursor);
    while cursor < until {
        let next = (cursor + Duration::days(1)).min(until);
        let offset = offset_at(timezone, next);
        if offset != current {
            let changed_at = transition(timezone, cursor, next);
            segments.push(OffsetSegment {
                until: Some(changed_at),
                seconds: current,
            });
            current = offset;
        }
        cursor = next;
    }
    segments.push(OffsetSegment {
        until: None,
        seconds: current,
    });
    TimezoneShift { segments }
}

pub fn bucket_start(day: NaiveDate, granularity: TimeGranularity) -> NaiveDate {
    match granularity {
        TimeGranularity::Day => day,
        TimeGranularity::Week => day
            .checked_sub_days(Days::new(u64::from(day.weekday().num_days_from_monday())))
            .unwrap_or(day),
        TimeGranularity::Month => day.with_day(1).unwrap_or(day),
        TimeGranularity::Quarter => {
            let month = ((day.month() - 1) / 3) * 3 + 1;
            NaiveDate::from_ymd_opt(day.year(), month, 1).unwrap_or(day)
        }
        TimeGranularity::Year => NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day),
    }
}

pub fn next_bucket(start: NaiveDate, granularity: TimeGranularity) -> Option<NaiveDate> {
    match granularity {
        TimeGranularity::Day => start.checked_add_days(Days::new(1)),
        TimeGranularity::Week => start.checked_add_days(Days::new(7)),
        TimeGranularity::Month => start.checked_add_months(Months::new(1)),
        TimeGranularity::Quarter => start.checked_add_months(Months::new(3)),
        TimeGranularity::Year => start.checked_add_months(Months::new(12)),
    }
}

pub fn buckets(from: NaiveDate, to: NaiveDate, granularity: TimeGranularity) -> Vec<String> {
    let mut keys = Vec::new();
    let mut cursor = bucket_start(from, granularity);
    while cursor <= to && keys.len() < MAX_BUCKETS {
        keys.push(cursor.format(BUCKET_FORMAT).to_string());
        match next_bucket(cursor, granularity) {
            Some(next) => cursor = next,
            None => break,
        }
    }
    keys
}
