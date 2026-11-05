pub use aphel_sdk::workflow::{
    DAYS_IN_STAGE_FIELD, IS_ROTTING_FIELD, ROTTING_AT_FIELD, STAGE_ENTERED_COLUMN,
};

use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::fields::{
    COMPUTED_RECORD_ALIAS, ComputedColumn, SystemField,
};
use aphel_sdk::platform::domain::fields::FieldType;

pub(crate) const MAX_ROTTING_DAYS: i64 = 3650;
pub(crate) const ROTTING_DAYS_FIELD: &str = "rotting_days";

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

pub use aphel_sdk::workflow::RottingState;

fn field_expression(
    definition: &aphel_sdk::platform::application::fields::EntityDefinition,
    key: &str,
) -> Result<String, AppError> {
    let field = definition.system_field_by_key(key).ok_or_else(|| {
        AppError::Configuration(format!(
            "workflow rotting requires a declared system field {key}"
        ))
    })?;
    Ok(
        aphel_sdk::platform::infrastructure::query::computed::system_expression(
            field,
            COMPUTED_RECORD_ALIAS,
        ),
    )
}

fn rotting_fields(
    mapping: &aphel_sdk::workflow::StagedMapping,
    definition: &aphel_sdk::platform::application::fields::EntityDefinition,
) -> Result<Vec<SystemField>, AppError> {
    let stage = field_expression(definition, &mapping.stage_field)?;
    let entered = field_expression(definition, &mapping.entered_at_field)?;
    let active = match (&mapping.state_field, &mapping.active_state) {
        (Some(field), Some(state)) => {
            let field = field_expression(definition, field)?;
            let encoded: String = state
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            format!(" AND {field} = CONVERT(X'{encoded}' USING utf8mb4) COLLATE utf8mb4_unicode_ci")
        }
        _ => String::new(),
    };
    let source = format!(
        "FROM workflow_stages rot_ps WHERE rot_ps.id = {stage} AND rot_ps.rotting_days IS NOT NULL{active}"
    );
    Ok(vec![
        SystemField::new(DAYS_IN_STAGE_FIELD, crate::labels("Days in stage"), FieldType::Number, DAYS_IN_STAGE_FIELD)?
            .computed(ComputedColumn::expression(format!("TIMESTAMPDIFF(DAY, {entered}, UTC_TIMESTAMP(6))"))),
        SystemField::new(IS_ROTTING_FIELD, crate::labels("Stalled in stage"), FieldType::Checkbox, IS_ROTTING_FIELD)?
            .computed(ComputedColumn::expression(format!("EXISTS (SELECT 1 {source} AND {entered} <= DATE_SUB(UTC_TIMESTAMP(6), INTERVAL rot_ps.rotting_days DAY))"))),
        SystemField::new(ROTTING_AT_FIELD, crate::labels("Stalled since"), FieldType::DateTime, ROTTING_AT_FIELD)?
            .computed(ComputedColumn::expression(format!("SELECT DATE_ADD({entered}, INTERVAL rot_ps.rotting_days DAY) {source}"))),
    ])
}

pub use aphel_sdk::workflow::rotting::{
    PIPELINE_STAGE_ROTTING_TIGHTENED, rotting_catch_up_filter, stale_record_filter, tightened_stage,
};

pub(crate) fn extend_staged(
    definition: &mut aphel_sdk::platform::application::fields::EntityDefinition,
) -> Result<(), AppError> {
    use aphel_sdk::workflow::WorkflowTraits;
    let Some(mapping) = definition.capabilities.stage_mapping() else {
        return Ok(());
    };
    if !mapping.rotting {
        return Ok(());
    }
    for field in rotting_fields(&mapping, definition)? {
        if definition.system_field_by_key(field.key.as_str()).is_some() {
            return Err(AppError::Configuration(format!(
                "workflow computed field {} is already declared",
                field.key
            )));
        }
        definition.system_fields.push(field);
    }
    Ok(())
}

pub(crate) fn rotting_tightened(old: Option<u16>, new: Option<u16>) -> bool {
    match (old, new) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(old), Some(new)) => new < old,
    }
}
