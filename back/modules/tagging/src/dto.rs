use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::Tag;
use crate::{CreateTagCommand, TagView, UpdateTagCommand};
use aphel_sdk::platform::api::dto::common::double_option;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TagListParams {
    #[param(example = "people.person")]
    pub(crate) record_type: Option<String>,
    #[param(example = "categories")]
    pub(crate) vocabulary: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct RecordTagLookupParams {
    #[param(example = "people.person")]
    pub(crate) record_type: String,
    #[param(example = "0190f1a2-0000-7000-8000-000000000001,0190f1a2-0000-7000-8000-000000000002")]
    pub(crate) ids: Option<String>,
}

impl RecordTagLookupParams {
    pub(crate) fn ids(&self) -> Vec<String> {
        self.ids
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateTagRequest {
    #[schema(example = "categories")]
    pub(crate) vocabulary: Option<String>,
    #[schema(example = "VIP customer")]
    pub(crate) name: String,
    #[schema(example = "vip-customer")]
    pub(crate) slug: Option<String>,
    pub(crate) parent_id: Option<String>,
    #[schema(example = "#1f6feb")]
    pub(crate) color: Option<String>,
    #[serde(default)]
    #[schema(example = json!(["people.person", "organizations.organization"]))]
    pub(crate) record_types: Vec<String>,
}

impl CreateTagRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateTagCommand {
        CreateTagCommand {
            vocabulary: self.vocabulary,
            name: self.name,
            slug: self.slug,
            parent_id: self.parent_id,
            color: self.color,
            record_types: self.record_types,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateTagRequest {
    pub(crate) name: Option<String>,
    pub(crate) slug: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) parent_id: Option<Option<String>>,
    pub(crate) color: Option<String>,
    pub(crate) record_types: Option<Vec<String>>,
}

impl UpdateTagRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateTagCommand {
        UpdateTagCommand {
            name: self.name,
            slug: self.slug,
            parent_id: self.parent_id,
            color: self.color,
            record_types: self.record_types,
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TagRecordRequest {
    pub(crate) tag_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TagSummary {
    pub(crate) id: String,
    #[schema(example = "VIP customer")]
    pub(crate) name: String,
    #[schema(example = "#1f6feb")]
    pub(crate) color: String,
}

impl From<Tag> for TagSummary {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id.to_string(),
            name: tag.name,
            color: tag.color,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TagResponse {
    pub(crate) id: String,
    #[schema(example = "general")]
    pub(crate) vocabulary: String,
    pub(crate) name: String,
    pub(crate) slug: Option<String>,
    pub(crate) parent_id: Option<String>,
    pub(crate) color: String,
    #[schema(example = json!(["contact"]))]
    pub(crate) record_types: Vec<String>,
    pub(crate) usage_count: u64,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<TagView> for TagResponse {
    fn from(view: TagView) -> Self {
        let tag = view.tag;
        Self {
            id: tag.id.to_string(),
            vocabulary: view.vocabulary,
            name: tag.name,
            slug: tag.slug,
            parent_id: tag.parent_id.map(|id| id.to_string()),
            color: tag.color,
            record_types: tag.record_types,
            usage_count: view.usage_count,
            created_by: tag.created_by.map(|id| id.into_inner()),
            created_at: timestamp(tag.created_at),
            updated_at: timestamp(tag.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordTagsEntry {
    pub(crate) record_id: String,
    pub(crate) tags: Vec<TagSummary>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TaggingResponse {
    pub(crate) changed: bool,
    pub(crate) tags: Vec<TagSummary>,
}
