use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::domain::custom::{ScheduleFrequency, ScheduleTiming};

const SEARCH_DAYS: u64 = 400;

fn last_day_of_month(day: NaiveDate) -> u32 {
    let (year, month) = if day.month() == 12 {
        (day.year() + 1, 1)
    } else {
        (day.year(), day.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|first| first.pred_opt())
        .map_or(28, |last| last.day())
}

fn matches(timing: &ScheduleTiming, day: NaiveDate) -> bool {
    match timing.frequency {
        ScheduleFrequency::Daily => true,
        ScheduleFrequency::Weekly => {
            u32::from(timing.weekday.unwrap_or(1)) == day.weekday().number_from_monday()
        }
        ScheduleFrequency::Monthly => {
            let wanted = u32::from(timing.day_of_month.unwrap_or(1));
            day.day() == wanted.min(last_day_of_month(day))
        }
    }
}

fn local_instant(timezone: Tz, day: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
    let local = day.and_time(time);
    timezone
        .from_local_datetime(&local)
        .earliest()
        .or_else(|| {
            timezone
                .from_local_datetime(&(local + Duration::hours(1)))
                .earliest()
        })
        .map(|instant| instant.with_timezone(&Utc))
}

pub(crate) fn next_run_after(
    timing: &ScheduleTiming,
    timezone: Tz,
    after: DateTime<Utc>,
) -> DateTime<Utc> {
    let time = NaiveTime::from_hms_opt(u32::from(timing.hour), u32::from(timing.minute), 0)
        .unwrap_or_default();
    let start = after.with_timezone(&timezone).date_naive();
    (0..=SEARCH_DAYS)
        .filter_map(|offset| start.checked_add_days(Days::new(offset)))
        .filter(|day| matches(timing, *day))
        .filter_map(|day| local_instant(timezone, day, time))
        .find(|instant| *instant > after)
        .unwrap_or(after + Duration::days(1))
}
