use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::query::{Filter, FilterOperator};
use crate::errors::AppError;

pub const SEGMENT_FIELD: &str = "segment";
pub const TAGS_FIELD: &str = "tags";
pub const ID_FIELD: &str = "id";
pub const OWNER_TEAM_FIELD: &str = "owner_team";

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentSource {
    pub record_type: String,
    pub filter: Filter,
}

#[async_trait]
pub trait SegmentSourceResolver: Send + Sync {
    async fn source(
        &self,
        pool: &MySqlPool,
        segment_id: Uuid,
    ) -> Result<Option<SegmentSource>, AppError>;
}

pub type SegmentFilters = HashMap<Uuid, Filter>;

#[derive(Debug, Clone, Copy)]
pub enum SegmentResolution<'a> {
    Reject,
    Accept,
    Expand(&'a SegmentFilters),
}

pub fn segment_ids(filter: &Filter) -> Vec<Uuid> {
    let mut ids = Vec::new();
    collect_segment_ids(filter, &mut ids);
    ids
}

fn collect_segment_ids(filter: &Filter, ids: &mut Vec<Uuid>) {
    match filter {
        Filter::Group(group) => {
            for child in &group.conditions {
                collect_segment_ids(child, ids);
            }
        }
        Filter::Related(_) => {}
        Filter::Condition(condition) => {
            if condition.operator != FilterOperator::InSegment {
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

pub fn in_segment(segment_id: Uuid) -> Filter {
    Filter::condition(
        SEGMENT_FIELD,
        FilterOperator::InSegment,
        serde_json::Value::String(segment_id.to_string()),
    )
}
