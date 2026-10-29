use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::application::files::UserReference;

use crate::api::extractors::Pagination;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    #[schema(example = "validation_error")]
    pub code: String,
    pub message: String,
    #[schema(value_type = Option<Object>)]
    pub details: Option<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub meta: PageMeta,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({ "total": 120, "page": 1, "per_page": 25 }))]
pub struct PageMeta {
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, pagination: Pagination) -> Self {
        Self {
            data: items,
            meta: PageMeta {
                total,
                page: pagination.page,
                per_page: pagination.per_page,
            },
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Data<T> {
    pub data: T,
}

impl<T> Data<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> From<T> for Data<T> {
    fn from(data: T) -> Self {
        Self { data }
    }
}

pub fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    <Option<T> as serde::Deserialize>::deserialize(deserializer).map(Some)
}

pub fn decimal_text(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text),
        Value::Number(number) => Some(number.to_string()),
        other => Some(other.to_string()),
    }
}

pub fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct QuickSearchParams {
    #[param(example = "acme")]
    pub q: Option<String>,
}

impl QuickSearchParams {
    pub fn merge_into(self, search: &mut Option<String>) {
        let empty = search.as_deref().is_none_or(|term| term.trim().is_empty());
        if empty && let Some(q) = self.q {
            *search = Some(q);
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OwnerResponse {
    pub id: String,
    pub display_name: String,
}

impl From<UserReference> for OwnerResponse {
    fn from(user: UserReference) -> Self {
        Self {
            id: user.id.into_inner(),
            display_name: user.display_name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DuplicateMatchResponse {
    pub record_id: String,
    pub title: String,
    #[schema(example = json!(["email"]))]
    pub rules: Vec<String>,
}

impl From<crate::records::PossibleDuplicate> for DuplicateMatchResponse {
    fn from(found: crate::records::PossibleDuplicate) -> Self {
        Self {
            record_id: found.record_id,
            title: found.title,
            rules: found.rules,
        }
    }
}
