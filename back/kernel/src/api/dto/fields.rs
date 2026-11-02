use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::api::dto::common::double_option;
use crate::api::dto::field_permissions::FieldAccessSchema;
use crate::api::dto::members::timestamp;
use crate::api::dto::query::{FilterOperatorSchema, OperandKindSchema};
use crate::application::fields::{
    CreateFieldCommand, FieldOptionInput, FieldSchema, FieldView, OperatorDescription,
    ReorderCommand, UpdateFieldCommand, UpdateOptionCommand, config_to_json,
};
use crate::domain::fields::FieldOption;

#[derive(Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(as = FieldType)]
pub struct FieldTypeSchema(pub String);

#[derive(Debug, Deserialize, ToSchema)]
pub struct FieldOptionRequest {
    #[schema(example = "hot")]
    pub value: Option<String>,
    #[schema(example = "Hot")]
    pub label: String,
    #[schema(example = "#e5534b")]
    pub color: Option<String>,
}

impl From<FieldOptionRequest> for FieldOptionInput {
    fn from(request: FieldOptionRequest) -> Self {
        Self {
            value: request.value,
            label: request.label,
            color: request.color,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFieldRequest {
    #[schema(example = "lead_source")]
    pub key: String,
    #[schema(example = "Lead source")]
    pub label: String,
    #[schema(value_type = FieldTypeSchema)]
    pub field_type: String,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default)]
    pub is_searchable: bool,
    #[serde(default)]
    pub is_personal: bool,
    #[serde(default)]
    pub is_sensitive: bool,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub config: Value,
    #[serde(default)]
    pub options: Vec<FieldOptionRequest>,
}

impl CreateFieldRequest {
    pub fn into_command(self, ip: Option<String>) -> CreateFieldCommand {
        CreateFieldCommand {
            key: self.key,
            label: self.label,
            field_type: self.field_type,
            is_required: self.is_required,
            is_unique: self.is_unique,
            is_searchable: self.is_searchable,
            is_personal: self.is_personal,
            is_sensitive: self.is_sensitive,
            config: self.config,
            options: self.options.into_iter().map(Into::into).collect(),
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateFieldRequest {
    pub label: Option<String>,
    #[schema(value_type = Option<FieldTypeSchema>)]
    pub field_type: Option<String>,
    pub is_required: Option<bool>,
    pub is_unique: Option<bool>,
    pub is_searchable: Option<bool>,
    pub is_personal: Option<bool>,
    pub is_sensitive: Option<bool>,
    #[schema(value_type = Option<Object>)]
    pub config: Option<Value>,
}

impl UpdateFieldRequest {
    pub fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> UpdateFieldCommand {
        UpdateFieldCommand {
            label: self.label,
            field_type: self.field_type,
            is_required: self.is_required,
            is_unique: self.is_unique,
            is_searchable: self.is_searchable,
            is_personal: self.is_personal,
            is_sensitive: self.is_sensitive,
            config: self.config,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReorderRequest {
    pub ids: Vec<String>,
}

impl ReorderRequest {
    pub fn into_command(self, ip: Option<String>) -> ReorderCommand {
        ReorderCommand { ids: self.ids, ip }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateOptionRequest {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub color: Option<Option<String>>,
    pub is_active: Option<bool>,
}

impl UpdateOptionRequest {
    pub fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> UpdateOptionCommand {
        UpdateOptionCommand {
            label: self.label,
            color: self.color,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FieldListParams {
    pub include_inactive: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldOptionResponse {
    pub id: String,
    pub value: String,
    pub label: String,
    pub color: Option<String>,
    pub position: i32,
    pub is_active: bool,
}

impl From<FieldOption> for FieldOptionResponse {
    fn from(option: FieldOption) -> Self {
        Self {
            id: option.id.to_string(),
            value: option.value,
            label: option.label,
            color: option.color,
            position: option.position,
            is_active: option.is_active,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldResponse {
    pub labels: std::collections::BTreeMap<String, String>,
    pub id: String,
    pub record_type: String,
    pub key: String,
    pub label: String,
    #[schema(value_type = FieldTypeSchema)]
    pub field_type: String,
    pub is_system: bool,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_searchable: bool,
    pub is_personal: bool,
    pub is_sensitive: bool,
    #[schema(example = json!(null))]
    pub module_key: Option<String>,
    pub is_active: bool,
    pub is_filterable: bool,
    pub is_sortable: bool,
    pub position: i32,
    #[schema(value_type = Object)]
    pub config: Value,
    pub options: Vec<FieldOptionResponse>,
    #[schema(value_type = Vec<FilterOperatorSchema>)]
    pub operators: Vec<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: Option<String>,
    #[schema(value_type = FieldAccessSchema)]
    pub access: &'static str,
}

impl From<FieldView> for FieldResponse {
    fn from(view: FieldView) -> Self {
        let definition = view.definition;
        let field_type = definition.type_name().to_owned();
        Self {
            id: definition.id.to_string(),
            record_type: definition.record_type,
            key: definition.key.as_str().to_owned(),
            label: definition.label,
            labels: view.labels.0,
            field_type,
            is_system: definition.is_system,
            is_required: definition.is_required,
            is_unique: definition.is_unique,
            is_searchable: definition.is_searchable,
            is_personal: definition.is_personal,
            is_sensitive: definition.is_sensitive,
            module_key: definition.module_key.clone(),
            is_active: definition.is_active,
            is_filterable: view.filterable,
            is_sortable: view.sortable,
            position: definition.position,
            config: config_to_json(&definition.config),
            options: definition.options.into_iter().map(Into::into).collect(),
            operators: view
                .operators
                .iter()
                .map(|operator| operator.as_str().to_owned())
                .collect(),
            updated_at: view.updated_at.map(timestamp),
            access: view.access.as_str(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OperatorResponse {
    #[schema(value_type = FilterOperatorSchema)]
    pub operator: String,
    #[schema(value_type = OperandKindSchema)]
    pub operand: String,
}

impl From<OperatorDescription> for OperatorResponse {
    fn from(description: OperatorDescription) -> Self {
        Self {
            operator: description.operator.as_str().to_owned(),
            operand: description.operand.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldSchemaResponse {
    pub record_type: String,
    pub fields: Vec<FieldResponse>,
    pub operators: Vec<OperatorResponse>,
}

impl From<FieldSchema> for FieldSchemaResponse {
    fn from(schema: FieldSchema) -> Self {
        Self {
            record_type: schema.record_type,
            fields: schema.fields.into_iter().map(Into::into).collect(),
            operators: schema.operators.into_iter().map(Into::into).collect(),
        }
    }
}
