use chrono::{DateTime, Duration, NaiveDate, NaiveTime, SecondsFormat, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;

use crate::application::automations::definition::{AutomationDefinition, StepBody};
use crate::application::automations::templates::CUSTOM_FIELDS_KEY;
use crate::application::automations::time::duration::{
    DURATION_MESSAGE, OFFSET_MESSAGE, parse_duration, parse_offset,
};
use crate::application::fields::FieldSet;
use crate::domain::fields::FieldType;
use crate::errors::FieldViolation;

pub const WAIT_UNTIL_POLL_MINUTES: i64 = 10;

pub fn is_date_type(field_type: FieldType) -> bool {
    matches!(field_type, FieldType::Date | FieldType::DateTime)
}

pub fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn parse_timestamp(value: Option<&Value>) -> Option<DateTime<Utc>> {
    value
        .and_then(Value::as_str)
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|instant| instant.with_timezone(&Utc))
}

pub fn field_value<'a>(record: &'a Value, fields: &FieldSet, key: &str) -> Option<&'a Value> {
    let field = fields.find(key)?;
    let value = if field.is_system() {
        record.get(key)
    } else {
        record
            .get(CUSTOM_FIELDS_KEY)
            .and_then(|custom| custom.get(key))
    }?;
    (!value.is_null() && value.as_str().is_none_or(|text| !text.trim().is_empty())).then_some(value)
}

pub fn local_instant(day: NaiveDate, time: NaiveTime, timezone: Tz) -> DateTime<Utc> {
    let local = day.and_time(time);
    timezone
        .from_local_datetime(&local)
        .earliest()
        .map(|instant| instant.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&local))
}

pub fn instant_of(value: &Value, field_type: FieldType, timezone: Tz) -> Option<DateTime<Utc>> {
    let text = value.as_str()?.trim();
    match field_type {
        FieldType::Date => NaiveDate::parse_from_str(text.get(..10)?, "%Y-%m-%d")
            .ok()
            .map(|day| local_instant(day, NaiveTime::MIN, timezone)),
        FieldType::DateTime => DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|instant| instant.with_timezone(&Utc)),
        _ => None,
    }
}

pub struct WaitTarget<'a> {
    pub duration: Option<&'a str>,
    pub until_field: Option<&'a str>,
    pub offset: Option<&'a str>,
}

impl WaitTarget<'_> {
    pub fn resolve(
        &self,
        record: &Value,
        fields: &FieldSet,
        timezone: Tz,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, String> {
        if let Some(duration) = self.duration {
            let duration =
                parse_duration(duration).ok_or_else(|| format!("duration {DURATION_MESSAGE}"))?;
            return Ok(Some(now + duration));
        }
        let Some(key) = self.until_field else {
            return Err("the wait needs a duration or an until_field".to_owned());
        };
        let field = fields
            .find(key)
            .ok_or_else(|| format!("{key} is not a field of this record type"))?;
        let offset = match self.offset {
            Some(offset) => {
                parse_offset(offset).ok_or_else(|| format!("offset {OFFSET_MESSAGE}"))?
            }
            None => Duration::zero(),
        };
        Ok(field_value(record, fields, key)
            .and_then(|value| instant_of(value, field.definition.field_type, timezone))
            .map(|instant| instant + offset))
    }
}

pub fn wait_violations(
    definition: &AutomationDefinition,
    fields: &FieldSet,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    for (index, step) in definition.steps.iter().enumerate() {
        let path = format!("definition.steps.{index}");
        match &step.body {
            StepBody::Wait {
                duration,
                until_field,
                offset,
                ..
            } => match (duration, until_field) {
                (Some(_), Some(_)) | (None, None) => violations.push(FieldViolation::new(
                    path,
                    "must have exactly one of duration or until_field",
                )),
                (Some(duration), None) => {
                    if parse_duration(duration).is_none() {
                        violations.push(FieldViolation::new(
                            format!("{path}.duration"),
                            DURATION_MESSAGE,
                        ));
                    }
                    if offset.is_some() {
                        violations.push(FieldViolation::new(
                            format!("{path}.offset"),
                            "can only be used with until_field",
                        ));
                    }
                }
                (None, Some(key)) => {
                    match fields.find_active(key) {
                        Some(field) if is_date_type(field.definition.field_type) => {}
                        Some(_) => violations.push(FieldViolation::new(
                            format!("{path}.until_field"),
                            "must be a date or date and time field",
                        )),
                        None => violations.push(FieldViolation::new(
                            format!("{path}.until_field"),
                            "is not a field of this record type",
                        )),
                    }
                    if let Some(offset) = offset
                        && parse_offset(offset).is_none()
                    {
                        violations.push(FieldViolation::new(
                            format!("{path}.offset"),
                            OFFSET_MESSAGE,
                        ));
                    }
                }
            },
            StepBody::WaitUntil { timeout, .. } if parse_duration(timeout).is_none() => {
                violations.push(FieldViolation::new(
                    format!("{path}.timeout"),
                    DURATION_MESSAGE,
                ));
            }
            _ => {}
        }
    }
    violations
}
