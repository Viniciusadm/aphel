use chrono::{DateTime, Datelike, Days, Months, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

pub const MAX_CUSTOM_PERIOD_DAYS: i64 = 731;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodPreset {
    Today,
    Yesterday,
    Last7Days,
    Last30Days,
    Week,
    LastWeek,
    Month,
    LastMonth,
    Quarter,
    LastQuarter,
    Year,
    LastYear,
    Custom,
}

impl PeriodPreset {
    pub const ALL: [Self; 13] = [
        Self::Today,
        Self::Yesterday,
        Self::Last7Days,
        Self::Last30Days,
        Self::Week,
        Self::LastWeek,
        Self::Month,
        Self::LastMonth,
        Self::Quarter,
        Self::LastQuarter,
        Self::Year,
        Self::LastYear,
        Self::Custom,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::Last7Days => "last_7_days",
            Self::Last30Days => "last_30_days",
            Self::Week => "week",
            Self::LastWeek => "last_week",
            Self::Month => "month",
            Self::LastMonth => "last_month",
            Self::Quarter => "quarter",
            Self::LastQuarter => "last_quarter",
            Self::Year => "year",
            Self::LastYear => "last_year",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodSelection {
    pub preset: PeriodPreset,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

impl PeriodSelection {
    pub fn preset(preset: PeriodPreset) -> Self {
        Self {
            preset,
            from: None,
            to: None,
        }
    }

    pub fn custom(from: NaiveDate, to: NaiveDate) -> Self {
        Self {
            preset: PeriodPreset::Custom,
            from: Some(from),
            to: Some(to),
        }
    }
}

impl Default for PeriodSelection {
    fn default() -> Self {
        Self::preset(PeriodPreset::Month)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodError {
    MissingCustomRange,
    InvertedRange,
    RangeTooLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPeriod {
    pub preset: PeriodPreset,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

pub fn start_of_day(timezone: Tz, day: NaiveDate) -> DateTime<Utc> {
    let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    timezone
        .from_local_datetime(&midnight)
        .earliest()
        .or_else(|| {
            timezone
                .from_local_datetime(&(midnight + chrono::Duration::hours(1)))
                .earliest()
        })
        .map(|local| local.with_timezone(&Utc))
        .unwrap_or_else(|| midnight.and_utc())
}

pub fn local_today(now: DateTime<Utc>, timezone: Tz) -> NaiveDate {
    now.with_timezone(&timezone).date_naive()
}

fn first_of_month(day: NaiveDate) -> NaiveDate {
    day.with_day(1).unwrap_or(day)
}

fn first_of_quarter(day: NaiveDate) -> NaiveDate {
    let month = ((day.month() - 1) / 3) * 3 + 1;
    NaiveDate::from_ymd_opt(day.year(), month, 1).unwrap_or(day)
}

fn first_of_year(day: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day)
}

fn first_of_week(day: NaiveDate) -> NaiveDate {
    day.checked_sub_days(Days::new(u64::from(day.weekday().num_days_from_monday())))
        .unwrap_or(day)
}

fn before(day: NaiveDate) -> NaiveDate {
    day.pred_opt().unwrap_or(day)
}

fn days_before(day: NaiveDate, days: u64) -> NaiveDate {
    day.checked_sub_days(Days::new(days)).unwrap_or(day)
}

fn months_span(first: NaiveDate, months: u32) -> (NaiveDate, NaiveDate) {
    let next = first
        .checked_add_months(Months::new(months))
        .unwrap_or(first);
    (first, before(next))
}

fn months_back(first: NaiveDate, months: u32) -> NaiveDate {
    first
        .checked_sub_months(Months::new(months))
        .unwrap_or(first)
}

fn whole_months(from: NaiveDate, to: NaiveDate) -> Option<u32> {
    if from.day() != 1 || to.succ_opt().is_none_or(|next| next.day() != 1) {
        return None;
    }
    let after = to.succ_opt()?;
    let months = (after.year() - from.year()) * 12 + after.month() as i32 - from.month() as i32;
    u32::try_from(months).ok().filter(|months| *months > 0)
}

pub fn period_between(
    timezone: Tz,
    preset: PeriodPreset,
    from: NaiveDate,
    to: NaiveDate,
) -> ResolvedPeriod {
    let after = to.succ_opt().unwrap_or(to);
    ResolvedPeriod {
        preset,
        from,
        to,
        starts_at: start_of_day(timezone, from),
        ends_at: start_of_day(timezone, after),
    }
}

pub fn previous_period(period: &ResolvedPeriod, timezone: Tz) -> ResolvedPeriod {
    let (from, to) = match whole_months(period.from, period.to) {
        Some(months) => months_span(months_back(period.from, months), months),
        None => {
            let days = u64::try_from((period.to - period.from).num_days() + 1).unwrap_or(1);
            (days_before(period.from, days), before(period.from))
        }
    };
    period_between(timezone, period.preset, from, to)
}

pub fn resolve_period(
    selection: &PeriodSelection,
    now: DateTime<Utc>,
    timezone: Tz,
) -> Result<ResolvedPeriod, PeriodError> {
    let today = local_today(now, timezone);
    let (from, to) = match selection.preset {
        PeriodPreset::Today => (today, today),
        PeriodPreset::Yesterday => (before(today), before(today)),
        PeriodPreset::Last7Days => (days_before(today, 6), today),
        PeriodPreset::Last30Days => (days_before(today, 29), today),
        PeriodPreset::Week => {
            let first = first_of_week(today);
            (first, first.checked_add_days(Days::new(6)).unwrap_or(first))
        }
        PeriodPreset::LastWeek => {
            let first = days_before(first_of_week(today), 7);
            (first, first.checked_add_days(Days::new(6)).unwrap_or(first))
        }
        PeriodPreset::Month => months_span(first_of_month(today), 1),
        PeriodPreset::LastMonth => months_span(months_back(first_of_month(today), 1), 1),
        PeriodPreset::Quarter => months_span(first_of_quarter(today), 3),
        PeriodPreset::LastQuarter => months_span(months_back(first_of_quarter(today), 3), 3),
        PeriodPreset::Year => months_span(first_of_year(today), 12),
        PeriodPreset::LastYear => months_span(months_back(first_of_year(today), 12), 12),
        PeriodPreset::Custom => {
            let (Some(from), Some(to)) = (selection.from, selection.to) else {
                return Err(PeriodError::MissingCustomRange);
            };
            if to < from {
                return Err(PeriodError::InvertedRange);
            }
            if (to - from).num_days() + 1 > MAX_CUSTOM_PERIOD_DAYS {
                return Err(PeriodError::RangeTooLong);
            }
            (from, to)
        }
    };
    Ok(period_between(timezone, selection.preset, from, to))
}
