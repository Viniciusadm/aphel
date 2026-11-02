use crate::application::fields::{RecordTypeDefinition, SystemField};
use crate::domain::fields::{FieldConfig, FieldType};
use crate::errors::AppError;

pub fn field(
    key: &str,
    label: impl Into<crate::application::fields::LocalizedLabels>,
    field_type: FieldType,
) -> Result<SystemField, AppError> {
    SystemField::new(key, label, field_type, key)
}

pub fn record_field(
    key: &str,
    label: impl Into<crate::application::fields::LocalizedLabels>,
    target: &str,
) -> Result<SystemField, AppError> {
    Ok(
        field(key, label, FieldType::Record)?.with_config(FieldConfig {
            target_record_type: Some(target.to_owned()),
            ..FieldConfig::default()
        }),
    )
}

pub fn timestamps() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        field(
            "created_at",
            crate::labels("Created at"),
            FieldType::DateTime,
        )?,
        field(
            "updated_at",
            crate::labels("Updated at"),
            FieldType::DateTime,
        )?,
    ])
}

pub fn with_fields(
    definition: RecordTypeDefinition,
    fields: Vec<SystemField>,
) -> RecordTypeDefinition {
    fields
        .into_iter()
        .fold(definition, RecordTypeDefinition::system_field)
}
