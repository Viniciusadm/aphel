use std::str::FromStr;

use chrono::{DateTime, Days, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::automations::definition::TriggerSpec;
use crate::application::automations::time::duration::{
    OFFSET_MESSAGE, parse_offset, parse_time_of_day,
};
use crate::application::automations::time::waits::{
    field_value, instant_of, is_date_type, local_instant,
};
use crate::application::fields::FieldSet;
use crate::application::jobs::CronSchedule;
use crate::application::query::{Filter, FilterOperator, in_segment};
use crate::domain::fields::FieldType;
use crate::errors::FieldViolation;

pub const TIME_TRIGGER_PREFIX: &str = "time.";
pub const TIME_DATE_RELATIVE: &str = "time.date_relative";
pub const TIME_RECORD_STALE: &str = "time.record_stale";
pub const TIME_SCHEDULE: &str = "time.schedule";

pub const UPDATED_AT_FIELD: &str = "updated_at";
pub const CREATED_AT_FIELD: &str = "created_at";
pub const LAST_ACTIVITY_FIELD: &str = "last_activity_at";
pub const STAGE_ENTERED_FIELD: &str = "stage_entered_at";

pub const MAX_STALE_DAYS: i64 = 3650;
pub const MIN_SCHEDULE_INTERVAL_MINUTES: i64 = 15;

pub fn is_time_trigger(kind: &str) -> bool {
    kind.starts_with(TIME_TRIGGER_PREFIX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleMode {
    NoUpdate,
    NoActivity,
    SameStage,
}

impl StaleMode {
    pub const ALL: [Self; 3] = [Self::NoUpdate, Self::NoActivity, Self::SameStage];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoUpdate => "no_update",
            Self::NoActivity => "no_activity",
            Self::SameStage => "same_stage",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }

    pub fn field(&self) -> &'static str {
        match self {
            Self::NoUpdate => UPDATED_AT_FIELD,
            Self::NoActivity => LAST_ACTIVITY_FIELD,
            Self::SameStage => STAGE_ENTERED_FIELD,
        }
    }

    pub fn fallback(&self) -> Option<&'static str> {
        match self {
            Self::NoActivity => Some(CREATED_AT_FIELD),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DueRule {
    pub field: String,
    pub fallback: Option<String>,
    pub offset: Duration,
    pub at: Option<NaiveTime>,
    pub key_field: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScheduleRule {
    pub expression: String,
    pub filter: Option<Filter>,
    pub segment_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum TimePlan {
    Due(DueRule),
    Schedule(ScheduleRule),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DueValue {
    pub field: String,
    pub value: String,
    pub instant: DateTime<Utc>,
}

pub fn field_type(fields: &FieldSet, key: &str) -> Option<FieldType> {
    fields.find(key).map(|field| field.definition.field_type)
}

pub fn date_text(day: chrono::NaiveDate) -> Value {
    Value::String(day.format("%Y-%m-%d").to_string())
}

pub fn instant_text(instant: DateTime<Utc>) -> Value {
    Value::String(instant.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
}

impl DueRule {
    fn window_for(
        &self,
        key: &str,
        kind: FieldType,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        timezone: Tz,
    ) -> Option<Filter> {
        match kind {
            FieldType::DateTime => Some(Filter::and(vec![
                Filter::condition(key, FilterOperator::Gte, instant_text(from - self.offset)),
                Filter::condition(key, FilterOperator::Lte, instant_text(to - self.offset)),
            ])),
            FieldType::Date => {
                let time = self.at.unwrap_or(NaiveTime::MIN);
                let due = |day| local_instant(day, time, timezone) + self.offset;
                let mut last = (to - self.offset).with_timezone(&timezone).date_naive();
                if due(last) > to {
                    last = last.checked_sub_days(Days::new(1))?;
                }
                let mut first = (from - self.offset).with_timezone(&timezone).date_naive();
                if due(first) < from {
                    first = first.checked_add_days(Days::new(1))?;
                }
                (first <= last).then(|| {
                    Filter::and(vec![
                        Filter::condition(key, FilterOperator::Gte, date_text(first)),
                        Filter::condition(key, FilterOperator::Lte, date_text(last)),
                    ])
                })
            }
            _ => None,
        }
    }

    pub fn window(
        &self,
        fields: &FieldSet,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        timezone: Tz,
    ) -> Option<Filter> {
        let primary = self.window_for(
            &self.field,
            field_type(fields, &self.field)?,
            from,
            to,
            timezone,
        );
        let Some(fallback) = &self.fallback else {
            return primary;
        };
        let secondary = self
            .window_for(fallback, field_type(fields, fallback)?, from, to, timezone)
            .map(|window| {
                Filter::and(vec![
                    Filter::condition(&self.field, FilterOperator::IsEmpty, Value::Null),
                    window,
                ])
            });
        match (primary, secondary) {
            (Some(primary), Some(secondary)) => Some(Filter::or(vec![primary, secondary])),
            (primary, secondary) => primary.or(secondary),
        }
    }

    fn due_value(
        &self,
        key: &str,
        record: &Value,
        fields: &FieldSet,
        timezone: Tz,
    ) -> Option<DueValue> {
        let kind = field_type(fields, key)?;
        let value = field_value(record, fields, key)?;
        let start = match (kind, self.at) {
            (FieldType::Date, Some(time)) => {
                let day = chrono::NaiveDate::parse_from_str(value.as_str()?.get(..10)?, "%Y-%m-%d")
                    .ok()?;
                local_instant(day, time, timezone)
            }
            _ => instant_of(value, kind, timezone)?,
        };
        Some(DueValue {
            field: key.to_owned(),
            value: value.as_str()?.trim().to_owned(),
            instant: start + self.offset,
        })
    }

    pub fn dedup_value(&self, record: &Value, fields: &FieldSet, due: &DueValue) -> Option<String> {
        let Some(key) = &self.key_field else {
            return Some(due.value.clone());
        };
        field_value(record, fields, key)
            .and_then(Value::as_str)
            .map(|value| value.trim().to_owned())
    }

    pub fn value(&self, record: &Value, fields: &FieldSet, timezone: Tz) -> Option<DueValue> {
        if let Some(value) = self.due_value(&self.field, record, fields, timezone) {
            return Some(value);
        }
        let fallback = self.fallback.as_deref()?;
        if field_value(record, fields, &self.field).is_some() {
            return None;
        }
        self.due_value(fallback, record, fields, timezone)
    }
}

impl ScheduleRule {
    pub fn filter(&self) -> Option<Filter> {
        let mut parts: Vec<Filter> = self.filter.iter().cloned().collect();
        if let Some(segment) = self.segment_id {
            parts.push(in_segment(segment));
        }
        match parts.len() {
            0 => None,
            1 => parts.pop(),
            _ => Some(Filter::and(parts)),
        }
    }

    pub fn latest_occurrence(
        &self,
        after: DateTime<Utc>,
        now: DateTime<Utc>,
        timezone: Tz,
    ) -> Option<DateTime<Utc>> {
        let schedule = cron::Schedule::from_str(&self.expression).ok()?;
        schedule
            .after(&after.with_timezone(&timezone))
            .take(100_000)
            .map(|occurrence| occurrence.with_timezone(&Utc))
            .take_while(|occurrence| *occurrence <= now)
            .last()
    }
}

pub fn violation(field: &str, message: impl Into<String>) -> FieldViolation {
    FieldViolation::new(format!("trigger.{field}"), message)
}

pub fn allowed_keys(spec: &TriggerSpec, allowed: &[&str]) -> Vec<FieldViolation> {
    spec.options
        .keys()
        .filter(|key| !allowed.contains(&key.as_str()))
        .map(|key| violation(key, "is not an option of this trigger"))
        .collect()
}

pub struct TimeTriggerPlanner {
    pub kind: &'static str,
    pub plan: fn(&TriggerSpec) -> Result<TimePlan, Vec<FieldViolation>>,
}

pub fn parse_plan(spec: &TriggerSpec) -> Result<TimePlan, Vec<FieldViolation>> {
    match spec.kind.as_str() {
        TIME_DATE_RELATIVE => {
            let mut violations = allowed_keys(spec, &["field", "offset", "at"]);
            let field = spec.text("field").map(str::to_owned);
            if field.is_none() {
                violations.push(violation("field", "is required"));
            }
            let offset = match spec.options.get("offset") {
                None | Some(Value::Null) => Some(Duration::zero()),
                Some(Value::String(text)) => parse_offset(text),
                Some(_) => None,
            };
            if offset.is_none() {
                violations.push(violation("offset", OFFSET_MESSAGE));
            }
            let at = match spec.options.get("at") {
                None | Some(Value::Null) => Some(None),
                Some(Value::String(text)) => parse_time_of_day(text).map(Some),
                Some(_) => None,
            };
            if at.is_none() {
                violations.push(violation("at", "must be a time of day in the format HH:MM"));
            }
            match (field, offset, at) {
                (Some(field), Some(offset), Some(at)) if violations.is_empty() => {
                    Ok(TimePlan::Due(DueRule {
                        field,
                        fallback: None,
                        offset,
                        at,
                        key_field: None,
                    }))
                }
                _ => Err(violations),
            }
        }
        TIME_RECORD_STALE => {
            let mut violations = allowed_keys(spec, &["mode", "days"]);
            let mode = spec.text("mode").and_then(StaleMode::parse);
            if mode.is_none() {
                violations.push(violation(
                    "mode",
                    "must be no_update, no_activity or same_stage",
                ));
            }
            let days = spec
                .options
                .get("days")
                .and_then(Value::as_i64)
                .filter(|days| (1..=MAX_STALE_DAYS).contains(days));
            if days.is_none() {
                violations.push(violation(
                    "days",
                    format!("must be an integer between 1 and {MAX_STALE_DAYS}"),
                ));
            }
            match (mode, days) {
                (Some(mode), Some(days)) if violations.is_empty() => Ok(TimePlan::Due(DueRule {
                    field: mode.field().to_owned(),
                    fallback: mode.fallback().map(str::to_owned),
                    offset: Duration::days(days),
                    at: None,
                    key_field: None,
                })),
                _ => Err(violations),
            }
        }
        TIME_SCHEDULE => {
            let mut violations = allowed_keys(spec, &["cron", "filter", "segment_id"]);
            let expression = spec.text("cron").map(str::to_owned);
            match expression.as_deref() {
                None => violations.push(violation("cron", "is required")),
                Some(expression) => match CronSchedule::parse(expression) {
                    Err(error) => violations.push(violation("cron", error.client_message())),
                    Ok(schedule) => {
                        let now = Utc::now();
                        let mut previous = schedule.next_after(now);
                        for _ in 0..8 {
                            let Some(current) = previous else { break };
                            let next = schedule.next_after(current);
                            if let Some(next) = next
                                && next - current < Duration::minutes(MIN_SCHEDULE_INTERVAL_MINUTES)
                            {
                                violations.push(violation(
                                    "cron",
                                    format!(
                                        "occurrences must be at least {MIN_SCHEDULE_INTERVAL_MINUTES} minutes apart"
                                    ),
                                ));
                                break;
                            }
                            previous = next;
                        }
                    }
                },
            }
            let filter = match spec.options.get("filter") {
                None | Some(Value::Null) => None,
                Some(value) => match serde_json::from_value::<Filter>(value.clone()) {
                    Ok(filter) => Some(filter),
                    Err(error) => {
                        violations.push(violation("filter", format!("is invalid: {error}")));
                        None
                    }
                },
            };
            let segment_id = match spec.options.get("segment_id") {
                None | Some(Value::Null) => None,
                Some(value) => match value
                    .as_str()
                    .and_then(|id| Uuid::parse_str(id.trim()).ok())
                {
                    Some(id) => Some(id),
                    None => {
                        violations.push(violation("segment_id", "must be a UUID"));
                        None
                    }
                },
            };
            match expression {
                Some(expression) if violations.is_empty() => Ok(TimePlan::Schedule(ScheduleRule {
                    expression,
                    filter,
                    segment_id,
                })),
                _ => Err(violations),
            }
        }
        kind => crate::modules::app()
            .all_contributions::<TimeTriggerPlanner>()
            .into_iter()
            .find(|(_, planner)| planner.kind == kind)
            .map(|(_, planner)| (planner.plan)(spec))
            .unwrap_or_else(|| Err(vec![violation("type", "is not a time trigger")])),
    }
}

pub fn plan_violations(spec: &TriggerSpec, fields: &FieldSet) -> Vec<FieldViolation> {
    let plan = match parse_plan(spec) {
        Ok(plan) => plan,
        Err(violations) => return violations,
    };
    let TimePlan::Due(rule) = plan else {
        return Vec::new();
    };
    let mut violations = Vec::new();
    let path = if spec.kind == TIME_DATE_RELATIVE {
        "field"
    } else {
        "mode"
    };
    match fields.find_active(&rule.field) {
        Some(field) if is_date_type(field.definition.field_type) => {
            if rule.at.is_some() && field.definition.field_type != FieldType::Date {
                violations.push(violation("at", "can only be used with date fields"));
            }
        }
        Some(_) => violations.push(violation(path, "must be a date or date and time field")),
        None if spec.kind == TIME_DATE_RELATIVE => {
            violations.push(violation(path, "is not a field of this record type"))
        }
        None => violations.push(violation(
            path,
            format!(
                "is not available for this record type (it needs the field {})",
                rule.field
            ),
        )),
    }
    violations
}

pub fn schema_filter() -> Value {
    json!({ "type": "object", "description": "Filter AST of the listings" })
}
