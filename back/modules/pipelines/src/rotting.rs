use aphel_contract_crm::CrmRecordTypes;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_contract_automation::application::definition::TriggerSpec;
use aphel_contract_automation::application::time::triggers::{DueRule, TimePlan};
use aphel_contract_automation::application::triggers::{AutomationTrigger, TriggerCandidate};
use aphel_kernel::application::fields::{
    COMPUTED_RECORD_ALIAS, ComputedColumn, FieldSet, SystemField,
};
use aphel_kernel::application::query::{Filter, FilterOperator};
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const MAX_ROTTING_DAYS: i64 = 3650;
pub(crate) const ROTTING_DAYS_FIELD: &str = "rotting_days";
pub(crate) const DAYS_IN_STAGE_FIELD: &str = "days_in_stage";
pub(crate) const IS_ROTTING_FIELD: &str = "is_rotting";
pub(crate) const ROTTING_AT_FIELD: &str = "rotting_at";
pub(crate) const STAGE_ENTERED_COLUMN: &str = "stage_entered_at";

pub(crate) fn valid_rotting_days(value: i64) -> Option<u16> {
    (1..=MAX_ROTTING_DAYS)
        .contains(&value)
        .then(|| u16::try_from(value).ok())
        .flatten()
}

pub(crate) fn rotting_days_message() -> String {
    format!("must be an integer between 1 and {MAX_ROTTING_DAYS} or null")
}

pub(crate) fn parse_rotting_days(
    value: Option<i64>,
    violations: &mut Vec<FieldViolation>,
) -> Option<u16> {
    let value = value?;
    let days = valid_rotting_days(value);
    if days.is_none() {
        violations.push(FieldViolation::new(
            ROTTING_DAYS_FIELD,
            rotting_days_message(),
        ));
    }
    days
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RottingState {
    pub days_in_stage: i64,
    pub(crate) rotting_days: Option<u16>,
    pub rotting_at: Option<DateTime<Utc>>,
    pub is_rotting: bool,
}

impl RottingState {
    pub fn of(
        is_open: bool,
        stage_entered_at: DateTime<Utc>,
        rotting_days: Option<u16>,
        now: DateTime<Utc>,
    ) -> Self {
        let rotting_at = rotting_days
            .filter(|_| is_open)
            .map(|days| stage_entered_at + Duration::days(i64::from(days)));
        Self {
            days_in_stage: (now - stage_entered_at).num_days(),
            rotting_days,
            rotting_at,
            is_rotting: rotting_at.is_some_and(|at| at <= now),
        }
    }
}

fn stage_limit_source() -> String {
    format!(
        "FROM pipeline_stages rot_ps WHERE rot_ps.id = {COMPUTED_RECORD_ALIAS}.`stage_id` \
         AND rot_ps.rotting_days IS NOT NULL AND {COMPUTED_RECORD_ALIAS}.`status` = 'open'"
    )
}

fn rotting_exceeded() -> ComputedColumn {
    ComputedColumn::expression(format!(
        "EXISTS (SELECT 1 {} AND {COMPUTED_RECORD_ALIAS}.`{STAGE_ENTERED_COLUMN}` \
         <= DATE_SUB(UTC_TIMESTAMP(6), INTERVAL rot_ps.rotting_days DAY))",
        stage_limit_source()
    ))
}

fn rotting_reached_at() -> ComputedColumn {
    ComputedColumn::expression(format!(
        "SELECT DATE_ADD({COMPUTED_RECORD_ALIAS}.`{STAGE_ENTERED_COLUMN}`, INTERVAL rot_ps.rotting_days DAY) {}",
        stage_limit_source()
    ))
}

pub(crate) fn rotting_fields() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        SystemField::new(
            DAYS_IN_STAGE_FIELD,
            crate::labels("Days in stage"),
            FieldType::Number,
            DAYS_IN_STAGE_FIELD,
        )?
        .computed(ComputedColumn::days_since(STAGE_ENTERED_COLUMN)),
        SystemField::new(
            IS_ROTTING_FIELD,
            crate::labels("Stalled in stage"),
            FieldType::Checkbox,
            IS_ROTTING_FIELD,
        )?
        .computed(rotting_exceeded()),
        SystemField::new(
            ROTTING_AT_FIELD,
            crate::labels("Stalled since"),
            FieldType::DateTime,
            ROTTING_AT_FIELD,
        )?
        .computed(rotting_reached_at()),
    ])
}

pub fn stale_deal_filter(without_limit: Vec<Filter>) -> Filter {
    let mut fallback = vec![Filter::condition(
        ROTTING_AT_FIELD,
        FilterOperator::IsEmpty,
        Value::Null,
    )];
    fallback.extend(without_limit);
    Filter::or(vec![
        Filter::condition(IS_ROTTING_FIELD, FilterOperator::Eq, Value::Bool(true)),
        Filter::and(fallback),
    ])
}

pub const TIME_DEAL_ROTTING: &str = "time.deal_rotting";
pub const PIPELINE_STAGE_ROTTING_TIGHTENED: &str = "pipeline_stage.rotting_tightened";

pub(crate) fn rotting_tightened(old: Option<u16>, new: Option<u16>) -> bool {
    match (old, new) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(old), Some(new)) => new < old,
    }
}

pub fn tightened_stage(event: &DomainEvent) -> Option<Uuid> {
    if event.event_type != PIPELINE_STAGE_ROTTING_TIGHTENED {
        return None;
    }
    event
        .payload
        .get("stage_id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

pub fn rotting_catch_up_filter(stage_id: Option<Uuid>) -> Filter {
    let rotting = Filter::condition(IS_ROTTING_FIELD, FilterOperator::Eq, Value::Bool(true));
    match stage_id {
        Some(stage_id) => Filter::and(vec![
            rotting,
            Filter::condition(
                "stage_id",
                FilterOperator::Eq,
                Value::String(stage_id.to_string()),
            ),
        ]),
        None => rotting,
    }
}

pub(crate) fn rotting_plan(spec: &TriggerSpec) -> Result<TimePlan, Vec<FieldViolation>> {
    let violations: Vec<FieldViolation> = spec
        .options
        .keys()
        .map(|key| {
            FieldViolation::new(format!("trigger.{key}"), "is not an option of this trigger")
        })
        .collect();
    if !violations.is_empty() {
        return Err(violations);
    }
    Ok(TimePlan::Due(DueRule {
        field: ROTTING_AT_FIELD.to_owned(),
        fallback: None,
        offset: Duration::zero(),
        at: None,
        key_field: Some(STAGE_ENTERED_COLUMN.to_owned()),
    }))
}

pub(crate) struct DealRotting;

impl AutomationTrigger for DealRotting {
    fn key(&self) -> &'static str {
        TIME_DEAL_ROTTING
    }
    fn label(&self) -> &'static str {
        "Negócio parado na etapa"
    }
    fn description(&self) -> &'static str {
        "Runs once per stage entry when an open deal stays in a stage longer than the stage's `rotting_days`; stages without a limit never fire, and a new run only happens after the deal enters a stage again"
    }
    fn supports(&self, record_type: &str) -> bool {
        record_type == RecordType::DEAL.as_str()
    }
    fn listens_to(&self, _event_type: &str) -> bool {
        false
    }
    fn candidates(&self, _event: &DomainEvent) -> Vec<TriggerCandidate> {
        Vec::new()
    }
    fn validate(&self, spec: &TriggerSpec, fields: &FieldSet) -> Vec<FieldViolation> {
        let mut violations = rotting_plan(spec).err().unwrap_or_default();
        for key in [ROTTING_AT_FIELD, STAGE_ENTERED_COLUMN] {
            if fields.find_active(key).is_none() {
                violations.push(FieldViolation::new(
                    "trigger.type",
                    format!("is not available (it needs the field {key})"),
                ));
            }
        }
        violations
    }
    fn record_filters(&self, _spec: &TriggerSpec) -> Vec<(String, Filter)> {
        vec![(
            "trigger".to_owned(),
            Filter::condition(ROTTING_AT_FIELD, FilterOperator::IsNotEmpty, Value::Null),
        )]
    }
    fn options_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }
    fn is_time_based(&self) -> bool {
        true
    }
}
