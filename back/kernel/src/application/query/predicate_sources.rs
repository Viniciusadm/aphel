use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::query::{Filter, FilterOperator};
use crate::database::Db;
use crate::errors::AppError;

pub const ID_FIELD: &str = "id";
pub const OWNER_TEAM_FIELD: &str = "owner_team";

#[derive(Debug, Clone, PartialEq)]
pub struct FilterSource {
    pub record_type: String,
    pub filter: Filter,
}

#[async_trait]
pub trait FilterSourceResolver: Send + Sync {
    fn field(&self) -> &'static str;
    async fn source(&self, pool: &Db, filter_id: Uuid) -> Result<Option<FilterSource>, AppError>;
}

pub type FilterExpansions = HashMap<Uuid, Filter>;

#[derive(Debug, Clone, Copy)]
pub enum FilterExpansion<'a> {
    Reject,
    Accept,
    Expand(&'a FilterExpansions),
}

pub fn filter_expansion_ids(filter: &Filter) -> Vec<Uuid> {
    let mut ids = Vec::new();
    collect_filter_expansion_ids(filter, &mut ids);
    ids
}

fn collect_filter_expansion_ids(filter: &Filter, ids: &mut Vec<Uuid>) {
    match filter {
        Filter::Group(group) => {
            for child in &group.conditions {
                collect_filter_expansion_ids(child, ids);
            }
        }
        Filter::Related(_) => {}
        Filter::Condition(condition) => {
            if condition.operator != FilterOperator::InSavedFilter {
                return;
            }
            if let Some(id) = condition
                .value
                .as_str()
                .and_then(|text| Uuid::parse_str(text.trim()).ok())
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
    }
}

pub fn in_saved_filter(filter_id: Uuid) -> Filter {
    Filter::condition(
        filter_source_field().unwrap_or("saved_filter"),
        FilterOperator::InSavedFilter,
        serde_json::Value::String(filter_id.to_string()),
    )
}

pub fn filter_source_field() -> Option<&'static str> {
    crate::modules::try_app()?
        .services()
        .get::<dyn FilterSourceResolver>()
        .map(|source| source.field())
}
#[derive(Debug, Clone)]
pub struct ReferenceSetField {
    pub key: &'static str,
    pub label: &'static str,
}
pub fn reference_set_field() -> Option<std::sync::Arc<ReferenceSetField>> {
    crate::modules::try_app()?
        .contributions::<ReferenceSetField>()
        .into_iter()
        .next()
}
