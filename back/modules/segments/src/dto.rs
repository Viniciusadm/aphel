use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::{CreateSegmentCommand, SegmentMember, SegmentView, UpdateSegmentCommand};
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::query::FilterNodeSchema;
use aphel_kernel::application::query::Filter;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SegmentListParams {
    #[param(example = "contacts.contact")]
    pub(crate) record_type: Option<String>,
    pub(crate) counts: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateSegmentRequest {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    #[schema(example = "Hot leads in São Paulo")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[schema(value_type = FilterNodeSchema)]
    pub(crate) filter: Option<Filter>,
}

impl CreateSegmentRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateSegmentCommand {
        CreateSegmentCommand {
            record_type: self.record_type,
            name: self.name,
            description: self.description,
            filter: self.filter,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateSegmentRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub(crate) filter: Option<Filter>,
}

impl UpdateSegmentRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateSegmentCommand {
        UpdateSegmentCommand {
            name: self.name,
            description: self.description,
            filter: self.filter,
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SegmentResponse {
    pub(crate) id: String,
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[schema(value_type = FilterNodeSchema)]
    pub(crate) filter: Filter,
    pub(crate) member_count: Option<u64>,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<SegmentView> for SegmentResponse {
    fn from(view: SegmentView) -> Self {
        let segment = view.segment;
        Self {
            id: segment.id.to_string(),
            record_type: segment.record_type.as_str().to_owned(),
            name: segment.name,
            description: segment.description,
            filter: segment.filter,
            member_count: view.member_count,
            created_by: segment.created_by.map(|id| id.into_inner()),
            created_at: timestamp(segment.created_at),
            updated_at: timestamp(segment.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SegmentMemberResponse {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
}

impl From<SegmentMember> for SegmentMemberResponse {
    fn from(member: SegmentMember) -> Self {
        Self {
            id: member.id.to_string(),
            label: member.label,
        }
    }
}
