use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};

use crate::api::dto::common::double_option;
use crate::api::dto::common::timestamp;
use crate::api::dto::fields::FieldSchemaResponse;
use crate::application::custom_objects::{
    CreateObjectCommand, CreateRelationshipCommand, CustomRecordDetails, ObjectView, RelationInfo,
    RelationshipView, SaveCustomRecordCommand, UpdateObjectCommand, UpdateRelationshipCommand,
};
use crate::application::fields::RecordRelation;
use crate::domain::custom_objects::{CustomObject, RelationshipDefinition};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RelationshipCardinality)]
pub enum CardinalitySchema {
    OneToOne,
    OneToMany,
    ManyToMany,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RelationshipOnDelete)]
pub enum OnDeleteSchema {
    Restrict,
    Unlink,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RelationSide)]
pub enum RelationSideSchema {
    From,
    To,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CustomObjectResponse {
    pub id: String,
    #[schema(example = "contract")]
    pub key: String,
    #[schema(example = "co_contract")]
    pub record_type: String,
    #[schema(example = "Contract")]
    pub label_singular: String,
    #[schema(example = "Contracts")]
    pub label_plural: String,
    pub description: Option<String>,
    #[schema(example = "file-text")]
    pub icon: Option<String>,
    #[schema(example = "#1f6feb")]
    pub color: Option<String>,
    #[schema(example = "name")]
    pub title_field_key: String,
    pub owner_enabled: bool,
    pub is_active: bool,
    pub position: i32,
    pub permissions: Vec<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
}

impl From<&CustomObject> for CustomObjectResponse {
    fn from(object: &CustomObject) -> Self {
        Self {
            id: object.id.to_string(),
            key: object.key.clone(),
            record_type: object.record_type(),
            label_singular: object.data.label_singular.clone(),
            label_plural: object.data.label_plural.clone(),
            description: object.data.description.clone(),
            icon: object.data.icon.clone(),
            color: object.data.color.clone(),
            title_field_key: object.data.title_field_key.clone(),
            owner_enabled: object.data.owner_enabled,
            is_active: object.data.is_active,
            position: object.position,
            permissions: crate::domain::custom_objects::OBJECT_ACTIONS
                .iter()
                .filter_map(|action| object.permission(action))
                .map(|permission| permission.as_str().to_owned())
                .collect(),
            created_at: timestamp(object.created_at),
            updated_at: timestamp(object.updated_at),
        }
    }
}

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

#[derive(Debug, Serialize, ToSchema)]
pub struct RelationshipResponse {
    pub id: String,
    #[schema(example = "contracts")]
    pub key: String,
    #[schema(example = "company")]
    pub inverse_key: String,
    #[schema(example = "company")]
    pub from_type: String,
    #[schema(example = "co_contract")]
    pub to_type: String,
    #[schema(value_type = CardinalitySchema)]
    pub cardinality: String,
    #[schema(example = "Contracts")]
    pub from_label: String,
    #[schema(example = "Company")]
    pub to_label: String,
    pub required: bool,
    #[schema(value_type = OnDeleteSchema)]
    pub on_delete: String,
    pub link_count: Option<u64>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
}

impl RelationshipResponse {
    fn new(relationship: RelationshipDefinition, link_count: Option<u64>) -> Self {
        Self {
            id: relationship.id.to_string(),
            key: relationship.key,
            inverse_key: relationship.inverse_key,
            from_type: relationship.from_type,
            to_type: relationship.to_type,
            cardinality: relationship.cardinality.as_str().to_owned(),
            from_label: relationship.from_label,
            to_label: relationship.to_label,
            required: relationship.required,
            on_delete: relationship.on_delete.as_str().to_owned(),
            link_count,
            created_at: timestamp(relationship.created_at),
            updated_at: timestamp(relationship.updated_at),
        }
    }
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
    #[schema(example = "company")]
    pub inverse_key: String,
    #[schema(example = "company")]
    pub from_type: String,
    #[schema(example = "co_contract")]
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

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct CustomRecordRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub owner_id: Option<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub custom_fields: Option<Map<String, Value>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>, example = json!({"company": "0192f0c4-7d0e-7a8a-9f3e-3c2b1a0d9e8f"}))]
    pub relations: Option<Map<String, Value>>,
}

impl CustomRecordRequest {
    pub fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveCustomRecordCommand {
        SaveCustomRecordCommand {
            owner_id: self.owner_id,
            custom_fields: self.custom_fields.unwrap_or_default(),
            relations: self.relations.unwrap_or_default(),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CustomRecordResponse {
    pub id: String,
    #[schema(example = "co_contract")]
    pub record_type: String,
    #[schema(example = "Contract 2026-001")]
    pub title: String,
    pub owner_id: Option<String>,
    pub created_by: Option<String>,
    pub archived_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
    #[schema(example = "2026-10-20T13:45:00.123456Z")]
    pub updated_at: String,
    #[schema(value_type = Object)]
    pub custom_fields: Map<String, Value>,
}

impl From<CustomRecordDetails> for CustomRecordResponse {
    fn from(details: CustomRecordDetails) -> Self {
        let record = details.record;
        Self {
            id: record.id.to_string(),
            record_type: details.object.record_type(),
            title: record.title,
            owner_id: record.owner_id.map(|owner| owner.into_inner()),
            created_by: record.created_by.map(|user| user.into_inner()),
            archived_at: record.archived_at.map(timestamp),
            created_at: timestamp(record.created_at),
            updated_at: record
                .updated_at
                .to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            custom_fields: details.custom_fields,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ObjectRelationResponse {
    #[schema(example = "company")]
    pub key: String,
    #[schema(example = "Company")]
    pub label: String,
    #[schema(example = "company")]
    pub target_type: String,
    pub single: bool,
    pub relationship_key: Option<String>,
}

impl From<&RecordRelation> for ObjectRelationResponse {
    fn from(relation: &RecordRelation) -> Self {
        Self {
            key: relation.key.clone(),
            label: relation.label.clone(),
            target_type: relation.target.as_str().to_owned(),
            single: relation.is_single(),
            relationship_key: None,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ObjectSchemaResponse {
    pub object: CustomObjectResponse,
    pub fields: FieldSchemaResponse,
    pub relations: Vec<ObjectRelationResponse>,
    #[schema(example = "/api/v1/objects/contract/records")]
    pub records_path: String,
    #[schema(example = json!(["co_contract.created", "co_contract.updated", "co_contract.deleted", "co_contract.archived", "co_contract.unarchived"]))]
    pub events: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RelationInfoResponse {
    #[schema(example = "contracts")]
    pub key: String,
    #[schema(example = "Contracts")]
    pub label: String,
    #[schema(example = "co_contract")]
    pub target_type: String,
    pub relationship_key: String,
    #[schema(value_type = RelationSideSchema)]
    pub side: String,
    pub single: bool,
    pub required: bool,
    pub readable: bool,
    pub total: Option<u64>,
}

impl From<RelationInfo> for RelationInfoResponse {
    fn from(info: RelationInfo) -> Self {
        Self {
            key: info.key,
            label: info.label,
            target_type: info.target_type,
            relationship_key: info.relationship_key,
            side: info.side.as_str().to_owned(),
            single: info.single,
            required: info.required,
            readable: info.readable,
            total: info.total,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RelatedRecordResponse {
    pub id: String,
    #[schema(example = "co_contract")]
    pub record_type: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkRecordRequest {
    pub record_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkRecordResponse {
    pub linked: bool,
}
