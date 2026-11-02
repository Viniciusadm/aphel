use chrono::{Duration, NaiveTime};

pub const MAX_DURATION_DAYS: i64 = 3650;

fn magnitude(text: &str) -> Option<Duration> {
    let unit = text.chars().last()?;
    let digits = &text[..text.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let amount: i64 = digits.parse().ok()?;
    let duration = match unit {
        'm' => Duration::try_minutes(amount)?,
        'h' => Duration::try_hours(amount)?,
        'd' => Duration::try_days(amount)?,
        'w' => Duration::try_weeks(amount)?,
        _ => return None,
    };
    (duration <= Duration::days(MAX_DURATION_DAYS)).then_some(duration)
}

pub fn parse_duration(text: &str) -> Option<Duration> {
    magnitude(text.trim())
}

pub fn parse_offset(text: &str) -> Option<Duration> {
    let text = text.trim();
    match text.strip_prefix('-') {
        Some(rest) => magnitude(rest).map(|duration| -duration),
        None => magnitude(text.strip_prefix('+').unwrap_or(text)),
    }
}

pub fn parse_time_of_day(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M").ok()
}

pub const DURATION_MESSAGE: &str =
    "must be a number followed by m, h, d or w (for example 30m, 2d or 1w), up to 3650 days";
pub const OFFSET_MESSAGE: &str = "must be a number followed by m, h, d or w, optionally signed (for example -3d or +2h), up to 3650 days";
