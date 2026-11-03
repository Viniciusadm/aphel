use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::api::dto::common::timestamp;
use crate::api::dto::fields::FieldSchemaResponse;
use crate::application::custom_objects::{
    CustomRecordDetails, RelationInfo, SaveCustomRecordCommand,
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
    #[schema(example = "custom.contract")]
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
    pub module_key: Option<String>,
    pub is_internal: bool,
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
            module_key: object.module_key.clone(),
            is_internal: object.is_internal,
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
pub struct RelationshipResponse {
    pub id: String,
    #[schema(example = "contracts")]
    pub key: String,
    #[schema(example = "organization_profile.organization")]
    pub inverse_key: String,
    #[schema(example = "organization_profile.organization")]
    pub from_type: String,
    #[schema(example = "custom.contract")]
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
    pub fn new(relationship: RelationshipDefinition, link_count: Option<u64>) -> Self {
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
    #[schema(example = "custom.contract")]
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
    #[schema(example = "organization_profile.organization")]
    pub key: String,
    #[schema(example = "Company")]
    pub label: String,
    #[schema(example = "organization_profile.organization")]
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
    #[schema(example = json!(["custom.contract.created", "custom.contract.updated", "custom.contract.deleted", "custom.contract.archived", "custom.contract.unarchived"]))]
    pub events: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RelationInfoResponse {
    #[schema(example = "contracts")]
    pub key: String,
    #[schema(example = "Contracts")]
    pub label: String,
    #[schema(example = "custom.contract")]
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
    #[schema(example = "custom.contract")]
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
