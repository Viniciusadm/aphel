use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::application::fields::permissions::FieldAccess;
use crate::application::query::{FilterOperator, OperandKind};
use crate::domain::fields::FieldDefinition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldOptionInput {
    pub value: Option<String>,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateFieldCommand {
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_searchable: bool,
    pub is_personal: bool,
    pub is_sensitive: bool,
    pub config: Value,
    pub options: Vec<FieldOptionInput>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpdateFieldCommand {
    pub label: Option<String>,
    pub field_type: Option<String>,
    pub is_required: Option<bool>,
    pub is_unique: Option<bool>,
    pub is_searchable: Option<bool>,
    pub is_personal: Option<bool>,
    pub is_sensitive: Option<bool>,
    pub config: Option<Value>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReorderCommand {
    pub ids: Vec<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateOptionCommand {
    pub label: Option<String>,
    pub color: Option<Option<String>>,
    pub is_active: Option<bool>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldView {
    pub definition: FieldDefinition,
    pub access: FieldAccess,
    pub updated_at: Option<DateTime<Utc>>,
    pub filterable: bool,
    pub sortable: bool,
    pub operators: Vec<FilterOperator>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorDescription {
    pub operator: FilterOperator,
    pub operand: OperandKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSchema {
    pub record_type: String,
    pub fields: Vec<FieldView>,
    pub operators: Vec<OperatorDescription>,
}
