use chrono::NaiveDate;

use crate::domain::dashboard::{PeriodError, PeriodPreset, PeriodSelection};
use crate::errors::FieldViolation;

pub const PERIOD_PRESET_MESSAGE: &str = "must be today, yesterday, last_7_days, last_30_days, week, last_week, month, last_month, quarter, last_quarter, year, last_year or custom";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PeriodQuery {
    pub period: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub fn parse_day(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

pub fn parse_preset(value: &str) -> Option<PeriodPreset> {
    PeriodPreset::parse(value.trim())
}

pub fn period_violation(field: &str, error: PeriodError) -> FieldViolation {
    let message = match error {
        PeriodError::MissingCustomRange => "a custom period needs from and to dates",
        PeriodError::InvertedRange => "must not be before from",
        PeriodError::RangeTooLong => "a custom period spans at most 731 days",
    };
    FieldViolation::new(field, message)
}

pub fn parse_period(
    query: &PeriodQuery,
    preset_field: &str,
    from_field: &str,
    to_field: &str,
) -> Result<PeriodSelection, Vec<FieldViolation>> {
    let mut violations = Vec::new();
    let preset = match query
        .period
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        None => PeriodPreset::Month,
        Some(value) => parse_preset(value).unwrap_or_else(|| {
            violations.push(FieldViolation::new(preset_field, PERIOD_PRESET_MESSAGE));
            PeriodPreset::Month
        }),
    };
    let mut day = |value: &Option<String>, field: &str| {
        value
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| {
                let parsed = parse_day(value);
                if parsed.is_none() {
                    violations.push(FieldViolation::new(field, "must be a date (YYYY-MM-DD)"));
                }
                parsed
            })
    };
    let from = day(&query.from, from_field);
    let to = day(&query.to, to_field);
    if !violations.is_empty() {
        return Err(violations);
    }
    let selection = if preset == PeriodPreset::Custom {
        match (from, to) {
            (Some(from), Some(to)) => PeriodSelection::custom(from, to),
            (from, to) => {
                let mut missing = Vec::new();
                if from.is_none() {
                    missing.push(FieldViolation::new(
                        from_field,
                        "is required for a custom period",
                    ));
                }
                if to.is_none() {
                    missing.push(FieldViolation::new(
                        to_field,
                        "is required for a custom period",
                    ));
                }
                return Err(missing);
            }
        }
    } else {
        PeriodSelection::preset(preset)
    };
    Ok(selection)
}
