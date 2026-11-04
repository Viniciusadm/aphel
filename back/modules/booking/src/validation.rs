use std::collections::HashSet;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::{
    Availability, ContactFieldOption, MAX_BUFFER_MINUTES, MAX_DESCRIPTION_CHARS,
    MAX_DURATION_MINUTES, MAX_HORIZON_DAYS, MAX_LOCATION_CHARS, MAX_MIN_NOTICE_MINUTES,
    MAX_NAME_CHARS, MAX_PER_DAY, MAX_QUESTION_LABEL, MAX_QUESTIONS, MAX_SLOT_INTERVAL_MINUTES,
    MAX_SLUG_CHARS, MAX_WEEKLY_INTERVALS, MIN_DURATION_MINUTES, MIN_SLUG_CHARS, Question,
    QuestionInput, QuestionKind, RESERVED_SLUGS, SaveAvailabilityCommand, WeeklyInterval,
    WeeklyIntervalInput, parse_clock,
};
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) fn invalid(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid booking settings", violations)
}

pub(crate) fn parse_timezone(value: &str) -> Option<Tz> {
    value.trim().parse::<Tz>().ok()
}

pub(crate) fn parse_instant(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.trim())
        .ok()
        .map(|instant| instant.with_timezone(&Utc))
}

fn bounded(
    value: u32,
    min: u32,
    max: u32,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> u32 {
    if value < min || value > max {
        violations.push(FieldViolation::new(
            field,
            format!("must be between {min} and {max}"),
        ));
    }
    value.clamp(min, max)
}

pub(crate) fn weekly_hours(
    inputs: &[WeeklyIntervalInput],
    violations: &mut Vec<FieldViolation>,
) -> Vec<WeeklyInterval> {
    if inputs.len() > MAX_WEEKLY_INTERVALS {
        violations.push(FieldViolation::new(
            "weekly_hours",
            format!("at most {MAX_WEEKLY_INTERVALS} intervals"),
        ));
        return Vec::new();
    }
    let mut intervals = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let field = format!("weekly_hours.{index}");
        if !(1..=7).contains(&input.weekday) {
            violations.push(FieldViolation::new(
                format!("{field}.weekday"),
                "must be between 1 (Monday) and 7 (Sunday)",
            ));
            continue;
        }
        let (Some(start_minute), Some(end_minute)) =
            (parse_clock(&input.start), parse_clock(&input.end))
        else {
            violations.push(FieldViolation::new(
                field,
                "start and end must be times in the HH:MM format",
            ));
            continue;
        };
        if start_minute >= end_minute {
            violations.push(FieldViolation::new(field, "end must be after start"));
            continue;
        }
        intervals.push(WeeklyInterval {
            weekday: input.weekday,
            start_minute,
            end_minute,
        });
    }
    intervals.sort_by_key(|interval| (interval.weekday, interval.start_minute));
    for pair in intervals.windows(2) {
        if pair[0].weekday == pair[1].weekday && pair[1].start_minute < pair[0].end_minute {
            violations.push(FieldViolation::new(
                "weekly_hours",
                "intervals of the same weekday cannot overlap",
            ));
            break;
        }
    }
    intervals
}

pub(crate) fn apply_availability(
    mut availability: Availability,
    command: &SaveAvailabilityCommand,
) -> Result<Availability, AppError> {
    let mut violations = Vec::new();
    if let Some(timezone) = &command.timezone {
        match parse_timezone(timezone) {
            Some(timezone) => availability.timezone = timezone,
            None => violations.push(FieldViolation::new(
                "timezone",
                "must be an IANA time zone such as America/Sao_Paulo",
            )),
        }
    }
    if let Some(inputs) = &command.weekly_hours {
        availability.weekly = weekly_hours(inputs, &mut violations);
    }
    if let Some(value) = command.min_notice_minutes {
        availability.min_notice_minutes = bounded(
            value,
            0,
            MAX_MIN_NOTICE_MINUTES,
            "min_notice_minutes",
            &mut violations,
        );
    }
    if let Some(value) = command.max_per_day {
        availability.max_per_day =
            value.map(|value| bounded(value, 1, MAX_PER_DAY, "max_per_day", &mut violations));
    }
    if let Some(value) = command.buffer_before_minutes {
        availability.buffer_before_minutes = bounded(
            value,
            0,
            MAX_BUFFER_MINUTES,
            "buffer_before_minutes",
            &mut violations,
        );
    }
    if let Some(value) = command.buffer_after_minutes {
        availability.buffer_after_minutes = bounded(
            value,
            0,
            MAX_BUFFER_MINUTES,
            "buffer_after_minutes",
            &mut violations,
        );
    }
    if let Some(value) = command.horizon_days {
        availability.horizon_days =
            bounded(value, 1, MAX_HORIZON_DAYS, "horizon_days", &mut violations);
    }
    if violations.is_empty() {
        Ok(availability)
    } else {
        Err(invalid(violations))
    }
}

pub(crate) fn text(
    value: &str,
    field: &str,
    max: usize,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        violations.push(FieldViolation::new(field, "must not be empty"));
        return None;
    }
    if value.chars().count() > max {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {max} characters"),
        ));
        return None;
    }
    Some(value.to_owned())
}

pub(crate) fn optional_text(
    value: Option<&str>,
    field: &str,
    max: usize,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    if value.chars().count() > max {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {max} characters"),
        ));
        return None;
    }
    Some(value.to_owned())
}

pub(crate) fn name(value: &str, violations: &mut Vec<FieldViolation>) -> Option<String> {
    text(value, "name", MAX_NAME_CHARS, violations)
}

pub(crate) fn description(
    value: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    optional_text(value, "description", MAX_DESCRIPTION_CHARS, violations)
}

pub(crate) fn location_value(
    value: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    optional_text(value, "location_value", MAX_LOCATION_CHARS, violations)
}

pub(crate) fn duration(value: u32, violations: &mut Vec<FieldViolation>) -> u32 {
    bounded(
        value,
        MIN_DURATION_MINUTES,
        MAX_DURATION_MINUTES,
        "duration_minutes",
        violations,
    )
}

pub(crate) fn slot_interval(value: u32, violations: &mut Vec<FieldViolation>) -> u32 {
    bounded(
        value,
        MIN_DURATION_MINUTES,
        MAX_SLOT_INTERVAL_MINUTES,
        "slot_interval_minutes",
        violations,
    )
}

pub(crate) fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for character in value.trim().to_lowercase().chars() {
        let mapped = match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        };
        if mapped.is_ascii_alphanumeric() {
            slug.push(mapped);
            dash = false;
        } else if !dash && !slug.is_empty() {
            slug.push('-');
            dash = true;
        }
    }
    let slug: String = slug
        .trim_end_matches('-')
        .chars()
        .take(MAX_SLUG_CHARS - 7)
        .collect();
    slug.trim_end_matches('-').to_owned()
}

pub(crate) fn valid_slug(slug: &str) -> bool {
    let length = slug.chars().count();
    (MIN_SLUG_CHARS..=MAX_SLUG_CHARS).contains(&length)
        && slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && !RESERVED_SLUGS.contains(&slug)
}

pub(crate) fn slug(value: &str, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let slug = value.trim().to_lowercase();
    if valid_slug(&slug) {
        Some(slug)
    } else {
        violations.push(FieldViolation::new(
            "slug",
            format!(
                "must have {MIN_SLUG_CHARS} to {MAX_SLUG_CHARS} lowercase letters, digits or single dashes"
            ),
        ));
        None
    }
}

pub(crate) fn random_suffix() -> Result<String, AppError> {
    let mut bytes = [0u8; 3];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn question_key(label: &str, index: usize) -> String {
    let base: String = slugify(label).replace('-', "_").chars().take(40).collect();
    if base.is_empty() {
        format!("question_{}", index + 1)
    } else {
        base
    }
}

pub(crate) fn questions(
    inputs: &[QuestionInput],
    fields: &[ContactFieldOption],
    violations: &mut Vec<FieldViolation>,
) -> Vec<Question> {
    if inputs.len() > MAX_QUESTIONS {
        violations.push(FieldViolation::new(
            "questions",
            format!("at most {MAX_QUESTIONS} questions"),
        ));
        return Vec::new();
    }
    let mut keys = HashSet::new();
    let mut mapped = HashSet::new();
    let mut questions = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let field = format!("questions.{index}");
        let Some(label) = text(
            &input.label,
            &format!("{field}.label"),
            MAX_QUESTION_LABEL,
            violations,
        ) else {
            continue;
        };
        let key = input
            .key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| question_key(&label, index));
        if key.len() > 64
            || !key.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
            || matches!(key.as_str(), "name" | "email" | "website")
        {
            violations.push(FieldViolation::new(
                format!("{field}.key"),
                "must use lowercase letters, digits or underscores and cannot be name, email or website",
            ));
            continue;
        }
        if !keys.insert(key.clone()) {
            violations.push(FieldViolation::new(
                format!("{field}.key"),
                "keys must be unique",
            ));
            continue;
        }
        let kind = input.kind.unwrap_or(QuestionKind::Text);
        let contact_field = input
            .contact_field
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        if let Some(target) = &contact_field {
            if !fields.iter().any(|option| &option.key == target) {
                violations.push(FieldViolation::new(
                    format!("{field}.contact_field"),
                    "is not a contact field that can receive answers",
                ));
                continue;
            }
            if !mapped.insert(target.clone()) {
                violations.push(FieldViolation::new(
                    format!("{field}.contact_field"),
                    "each contact field can receive only one answer",
                ));
                continue;
            }
        }
        questions.push(Question {
            key,
            label,
            kind,
            required: input.required,
            contact_field,
        });
    }
    questions
}
