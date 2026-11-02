use crate::application::fields::{EntityDefinition, SystemField};
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
        )?
        .read_only(),
        field(
            "updated_at",
            crate::labels("Updated at"),
            FieldType::DateTime,
        )?
        .read_only(),
    ])
}

pub fn with_fields(definition: EntityDefinition, fields: Vec<SystemField>) -> EntityDefinition {
    fields
        .into_iter()
        .fold(definition, EntityDefinition::system_field)
}
