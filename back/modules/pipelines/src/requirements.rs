use serde_json::{Map, Value};

use crate::domain::PipelineStage;
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const STAGE_REQUIREMENTS_MISSING: &str = "stage_requirements_missing";
pub(crate) const STAGE_REQUIREMENTS_INACCESSIBLE: &str = "stage_requirements_inaccessible";

pub(crate) const REQUIRABLE_SYSTEM_FIELDS: [&str; 8] = [
    "amount",
    "currency",
    "probability",
    "expected_close_date",
    "contact_id",
    "company_id",
    "owner_id",
    "source",
];

pub(crate) fn is_requirable(fields: &FieldSet, key: &str) -> bool {
    match fields.find_active(key) {
        Some(field) if field.is_system() => REQUIRABLE_SYSTEM_FIELDS.contains(&key),
        Some(_) => true,
        None => false,
    }
}

pub(crate) fn is_filled(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(text)) => !text.trim().is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(object)) => !object.is_empty(),
        Some(_) => true,
    }
}

pub(crate) fn required_keys<'a>(
    from: Option<&'a PipelineStage>,
    to: &'a PipelineStage,
) -> Vec<(&'a str, &'a PipelineStage, bool)> {
    let mut keys: Vec<(&str, &PipelineStage, bool)> = Vec::new();
    if let Some(from) = from {
        for key in &from.exit_required_fields {
            if !keys.iter().any(|(existing, _, _)| existing == key) {
                keys.push((key, from, false));
            }
        }
    }
    for key in &to.entry_required_fields {
        if !keys.iter().any(|(existing, _, _)| existing == key) {
            keys.push((key, to, true));
        }
    }
    keys
}

pub(crate) fn needs_custom_values(from: Option<&PipelineStage>, to: &PipelineStage) -> bool {
    required_keys(from, to)
        .iter()
        .any(|(key, _, _)| !REQUIRABLE_SYSTEM_FIELDS.contains(key))
}

pub(crate) fn missing_stage_fields(
    from: Option<&PipelineStage>,
    to: &PipelineStage,
    record: &Value,
    custom: &Map<String, Value>,
) -> Vec<FieldViolation> {
    required_keys(from, to)
        .into_iter()
        .filter(|(key, _, _)| {
            let value = if REQUIRABLE_SYSTEM_FIELDS.contains(key) {
                record.get(*key)
            } else {
                custom.get(*key)
            };
            !is_filled(value)
        })
        .map(|(key, stage, entering)| {
            let message = if entering {
                format!("is required to enter the stage {}", stage.name)
            } else {
                format!("is required to leave the stage {}", stage.name)
            };
            FieldViolation::new(key, message)
        })
        .collect()
}

pub(crate) fn ensure_stage_requirements(
    from: Option<&PipelineStage>,
    to: &PipelineStage,
    record: &Value,
    custom: &Map<String, Value>,
    fields: Option<&FieldSet>,
) -> Result<(), AppError> {
    let missing = missing_stage_fields(from, to, record, custom);
    if missing.is_empty() {
        return Ok(());
    }
    let inaccessible = |key: &str| {
        fields.is_some_and(|fields| {
            fields
                .find(key)
                .is_some_and(|field| field.is_hidden() || !field.is_editable())
        })
    };
    if missing
        .iter()
        .any(|violation| inaccessible(&violation.field))
    {
        let violations = missing
            .into_iter()
            .map(|violation| {
                if inaccessible(&violation.field) {
                    let message = format!(
                        "{} but the field is hidden or read-only for you",
                        violation.message
                    );
                    FieldViolation::new(violation.field, message)
                } else {
                    violation
                }
            })
            .collect();
        return Err(AppError::invalid_fields_code(
            STAGE_REQUIREMENTS_INACCESSIBLE,
            "the stage requires fields that are empty and that you cannot fill",
            violations,
        ));
    }
    Err(AppError::invalid_fields_code(
        STAGE_REQUIREMENTS_MISSING,
        "the stage requires fields that are empty",
        missing,
    ))
}
