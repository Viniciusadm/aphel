use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::application::automations::definition::TriggerSpec;
use aphel_kernel::application::automations::time::triggers::*;
use aphel_kernel::application::automations::triggers::{
    AutomationTrigger, TriggerCandidate, is_automatable,
};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::query::{Filter, FilterOperator, in_segment};
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::errors::FieldViolation;

struct DateRelative;

impl AutomationTrigger for DateRelative {
    fn key(&self) -> &'static str {
        TIME_DATE_RELATIVE
    }
    fn label(&self) -> &'static str {
        "Data de um campo"
    }
    fn description(&self) -> &'static str {
        "Runs once per record and field value when the date of `field` plus `offset` (for example -3d for three days before) is reached; `at` (HH:MM, organization time zone) sets the time of day for date fields"
    }
    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "field": { "type": "string", "description": "Date or date and time field", "x-field-types": ["date", "datetime"] },
                "offset": { "type": "string", "pattern": "^[+-]?\\d+(m|h|d|w)$", "default": "0d", "description": "Signed offset from the field value, such as -3d or +2h" },
                "at": { "type": "string", "pattern": "^\\d{2}:\\d{2}$", "description": "Time of day for date fields (HH:MM)" },
            },
            "required": ["field"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, _event_type: &str) -> bool {
        false
    }
    fn candidates(&self, _event: &DomainEvent) -> Vec<TriggerCandidate> {
        Vec::new()
    }
    fn validate(&self, spec: &TriggerSpec, fields: &FieldSet) -> Vec<FieldViolation> {
        plan_violations(spec, fields)
    }
    fn record_filters(&self, spec: &TriggerSpec) -> Vec<(String, Filter)> {
        match spec.text("field") {
            Some(field) => vec![(
                "trigger.field".to_owned(),
                Filter::condition(field, FilterOperator::IsNotEmpty, Value::Null),
            )],
            None => Vec::new(),
        }
    }
    fn is_time_based(&self) -> bool {
        true
    }
}

struct RecordStale;

impl AutomationTrigger for RecordStale {
    fn key(&self) -> &'static str {
        TIME_RECORD_STALE
    }
    fn label(&self) -> &'static str {
        "Registro parado"
    }
    fn description(&self) -> &'static str {
        "Runs once per stop when a record goes `days` days without updates (`no_update`), without activities (`no_activity`, deals) or in the same stage (`same_stage`, deals); a new stop only counts after the record changes again"
    }
    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "mode": { "type": "string", "enum": ["no_update", "no_activity", "same_stage"] },
                "days": { "type": "integer", "minimum": 1, "maximum": MAX_STALE_DAYS },
            },
            "required": ["mode", "days"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, _event_type: &str) -> bool {
        false
    }
    fn candidates(&self, _event: &DomainEvent) -> Vec<TriggerCandidate> {
        Vec::new()
    }
    fn validate(&self, spec: &TriggerSpec, fields: &FieldSet) -> Vec<FieldViolation> {
        if spec.text("mode") == Some(StaleMode::SameStage.as_str())
            && fields.record_type.record_type != RecordType::DEAL
        {
            let mut violations = vec![violation("mode", "same_stage can only be used on deals")];
            violations.extend(
                parse_plan(spec)
                    .err()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|violation| violation.field != "trigger.mode"),
            );
            return violations;
        }
        plan_violations(spec, fields)
    }
    fn record_filters(&self, spec: &TriggerSpec) -> Vec<(String, Filter)> {
        match spec.text("mode").and_then(StaleMode::parse) {
            Some(mode) => vec![(
                "trigger.mode".to_owned(),
                Filter::condition(mode.field(), FilterOperator::IsNotEmpty, Value::Null),
            )],
            None => Vec::new(),
        }
    }
    fn is_time_based(&self) -> bool {
        true
    }
}

struct Schedule;

impl AutomationTrigger for Schedule {
    fn key(&self) -> &'static str {
        TIME_SCHEDULE
    }
    fn label(&self) -> &'static str {
        "Agendamento"
    }
    fn description(&self) -> &'static str {
        "Runs on every occurrence of `cron` (seconds minutes hours day month weekday, organization time zone, at least 15 minutes apart) for each record that matches `filter` and the segment `segment_id`, once per record and occurrence"
    }
    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "cron": { "type": "string", "description": "Cron with seconds, such as 0 0 9 * * MON" },
                "filter": schema_filter(),
                "segment_id": { "type": "string", "format": "uuid" },
            },
            "required": ["cron"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, _event_type: &str) -> bool {
        false
    }
    fn candidates(&self, _event: &DomainEvent) -> Vec<TriggerCandidate> {
        Vec::new()
    }
    fn validate(&self, spec: &TriggerSpec, _fields: &FieldSet) -> Vec<FieldViolation> {
        parse_plan(spec).err().unwrap_or_default()
    }
    fn record_filters(&self, spec: &TriggerSpec) -> Vec<(String, Filter)> {
        let mut filters = Vec::new();
        if let Some(filter) = spec
            .options
            .get("filter")
            .filter(|value| !value.is_null())
            .and_then(|value| serde_json::from_value::<Filter>(value.clone()).ok())
        {
            filters.push(("trigger.filter".to_owned(), filter));
        }
        if let Some(segment) = spec
            .text("segment_id")
            .and_then(|id| Uuid::parse_str(id).ok())
        {
            filters.push(("trigger.segment_id".to_owned(), in_segment(segment)));
        }
        filters
    }
    fn is_time_based(&self) -> bool {
        true
    }
}

pub(crate) fn time_triggers() -> Vec<std::sync::Arc<dyn AutomationTrigger>> {
    vec![
        std::sync::Arc::new(DateRelative),
        std::sync::Arc::new(RecordStale),
        std::sync::Arc::new(Schedule),
    ]
}
