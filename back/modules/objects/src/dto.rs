use crate::application::*;
use aphel_sdk::platform::api::dto::common::double_option;
use aphel_sdk::platform::api::dto::custom_objects::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
#[derive(Debug, Serialize, ToSchema)]
pub struct CustomObjectAdminResponse {
    #[serde(flatten)]
    pub object: CustomObjectResponse,
    pub record_count: u64,
}

impl From<ObjectView> for CustomObjectAdminResponse {
    fn from(view: ObjectView) -> Self {
        Self {
            object: CustomObjectResponse::from(&view.object),
            record_count: view.record_count,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCustomObjectRequest {
    #[schema(example = "contract")]
    pub key: String,
    #[schema(example = "Contract")]
    pub label_singular: String,
    #[schema(example = "Contracts")]
    pub label_plural: String,
    pub description: Option<String>,
    #[schema(example = "file-text")]
    pub icon: Option<String>,
    #[schema(example = "#1f6feb")]
    pub color: Option<String>,
    pub owner_enabled: Option<bool>,
}

impl CreateCustomObjectRequest {
    pub fn into_command(self, ip: Option<String>) -> CreateObjectCommand {
        CreateObjectCommand {
            key: self.key,
            label_singular: self.label_singular,
            label_plural: self.label_plural,
            description: self.description,
            icon: self.icon,
            color: self.color,
            owner_enabled: self.owner_enabled,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateCustomObjectRequest {
    pub label_singular: Option<String>,
    pub label_plural: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub icon: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub color: Option<Option<String>>,
    #[schema(example = "contract_number")]
    pub title_field_key: Option<String>,
    pub owner_enabled: Option<bool>,
    #[schema(example = true)]
    pub staged: Option<bool>,
    pub is_active: Option<bool>,
}

impl UpdateCustomObjectRequest {
    pub fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateObjectCommand {
        UpdateObjectCommand {
            label_singular: self.label_singular,
            label_plural: self.label_plural,
            description: self.description,
            icon: self.icon,
            color: self.color,
            title_field_key: self.title_field_key,
            owner_enabled: self.owner_enabled,
            staged: self.staged,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReorderCustomObjectsRequest {
    pub keys: Vec<String>,
}

impl From<RelationshipView> for RelationshipResponse {
    fn from(view: RelationshipView) -> Self {
        Self::new(view.relationship, Some(view.link_count))
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RelationshipListParams {
    pub record_type: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRelationshipRequest {
    #[schema(example = "contracts")]
    pub key: String,
    #[schema(example = "organizations.organization")]
    pub inverse_key: String,
    #[schema(example = "organizations.organization")]
    pub from_type: String,
    #[schema(example = "custom.contract")]
    pub to_type: String,
    #[schema(value_type = CardinalitySchema)]
    pub cardinality: String,
    #[schema(example = "Contracts")]
    pub from_label: String,
    #[schema(example = "Company")]
    pub to_label: String,
    #[serde(default)]
    pub required: bool,
    #[schema(value_type = Option<OnDeleteSchema>)]
    pub on_delete: Option<String>,
}

impl CreateRelationshipRequest {
    pub fn into_command(self, ip: Option<String>) -> CreateRelationshipCommand {
        CreateRelationshipCommand {
            key: self.key,
            inverse_key: self.inverse_key,
            from_type: self.from_type,
            to_type: self.to_type,
            cardinality: self.cardinality,
            from_label: self.from_label,
            to_label: self.to_label,
            required: self.required,
            on_delete: self.on_delete,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateRelationshipRequest {
    pub from_label: Option<String>,
    pub to_label: Option<String>,
    pub required: Option<bool>,
    #[schema(value_type = Option<OnDeleteSchema>)]
    pub on_delete: Option<String>,
}

impl UpdateRelationshipRequest {
    pub fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateRelationshipCommand {
        UpdateRelationshipCommand {
            from_label: self.from_label,
            to_label: self.to_label,
            required: self.required,
            on_delete: self.on_delete,
            expected_updated_at,
            ip,
        }
    }
}
