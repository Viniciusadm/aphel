use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::application::fields::{COMPUTED_RECORD_ALIAS, ComputedColumn, SystemField};
use aphel_kernel::application::query::{Filter, FilterOperator};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const MAX_ROTTING_DAYS: i64 = 3650;
pub(crate) const ROTTING_DAYS_FIELD: &str = "rotting_days";
pub(crate) const DAYS_IN_STAGE_FIELD: &str = "days_in_stage";
pub(crate) const IS_ROTTING_FIELD: &str = "is_rotting";
pub const ROTTING_AT_FIELD: &str = "rotting_at";
pub const STAGE_ENTERED_COLUMN: &str = "stage_entered_at";

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

fn stage_limit_source(mapping: &aphel_sdk::workflow::StagedMapping) -> String {
    let active = match (&mapping.state_field, &mapping.active_state) {
        (Some(field), Some(state)) => format!(
            " AND {COMPUTED_RECORD_ALIAS}.`{field}` = '{}'",
            state.replace('\'', "''")
        ),
        _ => String::new(),
    };
    format!(
        "FROM workflow_stages rot_ps WHERE rot_ps.id = {COMPUTED_RECORD_ALIAS}.`{stage_field}` \
         AND rot_ps.rotting_days IS NOT NULL{active}",
        stage_field = mapping.stage_field
    )
}

fn rotting_exceeded(mapping: &aphel_sdk::workflow::StagedMapping) -> ComputedColumn {
    ComputedColumn::expression(format!(
        "EXISTS (SELECT 1 {} AND {COMPUTED_RECORD_ALIAS}.`{entered}` \
         <= DATE_SUB(UTC_TIMESTAMP(6), INTERVAL rot_ps.rotting_days DAY))",
        stage_limit_source(mapping),
        entered = mapping.entered_at_field
    ))
}

fn rotting_reached_at(mapping: &aphel_sdk::workflow::StagedMapping) -> ComputedColumn {
    ComputedColumn::expression(format!(
        "SELECT DATE_ADD({COMPUTED_RECORD_ALIAS}.`{entered}`, INTERVAL rot_ps.rotting_days DAY) {}",
        stage_limit_source(mapping),
        entered = mapping.entered_at_field
    ))
}

pub fn rotting_fields(
    mapping: &aphel_sdk::workflow::StagedMapping,
) -> Result<Vec<SystemField>, AppError> {
    for field in [
        &mapping.stage_field,
        &mapping.pipeline_field,
        &mapping.entered_at_field,
    ]
    .into_iter()
    .chain(mapping.state_field.iter())
    {
        if !aphel_kernel::application::fields::is_sql_identifier(field) {
            return Err(AppError::validation_code(
                "invalid_workflow_mapping",
                "invalid workflow field mapping",
            ));
        }
    }
    Ok(vec![
        SystemField::new(
            DAYS_IN_STAGE_FIELD,
            crate::labels("Days in stage"),
            FieldType::Number,
            DAYS_IN_STAGE_FIELD,
        )?
        .computed(ComputedColumn::days_since(&mapping.entered_at_field)),
        SystemField::new(
            IS_ROTTING_FIELD,
            crate::labels("Stalled in stage"),
            FieldType::Checkbox,
            IS_ROTTING_FIELD,
        )?
        .computed(rotting_exceeded(mapping)),
        SystemField::new(
            ROTTING_AT_FIELD,
            crate::labels("Stalled since"),
            FieldType::DateTime,
            ROTTING_AT_FIELD,
        )?
        .computed(rotting_reached_at(mapping)),
    ])
}

pub fn stale_record_filter(without_limit: Vec<Filter>) -> Filter {
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
