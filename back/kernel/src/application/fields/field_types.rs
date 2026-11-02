use std::sync::Arc;

use serde_json::Value;

use crate::application::query::{FilterOperator, operators_for};
use crate::domain::fields::{FieldDefinition, FieldType, FieldValueError, RawValue, ValueColumn};

pub type FieldValidator = fn(&FieldDefinition, &RawValue) -> Result<(), FieldValueError>;

#[derive(Debug, Clone)]
pub struct FieldTypeDefinition {
    pub key: &'static str,
    pub base: FieldType,
    pub config: Value,
    pub validate: FieldValidator,
    pub operators: Vec<FilterOperator>,
}

impl FieldTypeDefinition {
    pub fn field_type(&self) -> FieldType {
        if self.key == self.base.as_str() {
            self.base
        } else {
            FieldType::Custom {
                key: self.key,
                base: self.base.base_type().as_str(),
            }
        }
    }

    pub fn storage(&self) -> ValueColumn {
        self.base.value_column()
    }
}

pub fn find(key: &str) -> Option<Arc<FieldTypeDefinition>> {
    crate::modules::try_app()?
        .contributions::<FieldTypeDefinition>()
        .into_iter()
        .find(|definition| definition.key == key)
}

pub fn native_definitions() -> impl Iterator<Item = FieldTypeDefinition> {
    FieldType::ALL.into_iter().map(|base| FieldTypeDefinition {
        key: base.as_str(),
        base,
        config: serde_json::json!({"type": "object"}),
        validate: |definition, raw| {
            crate::domain::fields::validate_native_value(definition, raw).map(|_| ())
        },
        operators: operators_for(base),
    })
}

pub fn validate_registry(app: &crate::modules::App) -> Result<(), crate::errors::AppError> {
    let mut keys = std::collections::BTreeSet::new();
    for (_, definition) in app.all_contributions::<FieldTypeDefinition>() {
        let native = FieldType::ALL
            .iter()
            .any(|native| native.as_str() == definition.key);
        if !keys.insert(definition.key)
            || (!native && !aphel_contract::is_valid_key(definition.key))
            || matches!(definition.base, FieldType::Custom { .. })
            || !definition.config.is_object()
            || definition
                .operators
                .iter()
                .any(|operator| !operators_for(definition.base).contains(operator))
        {
            return Err(crate::errors::AppError::Configuration(format!(
                "invalid or duplicate field type {}",
                definition.key
            )));
        }
    }
    Ok(())
}
