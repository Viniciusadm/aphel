pub use aphel_contract_workflow::{
    DAYS_IN_STAGE_FIELD, IS_ROTTING_FIELD, ROTTING_AT_FIELD, STAGE_ENTERED_COLUMN,
};
use aphel_kernel::application::query::{Filter, FilterOperator};
use aphel_kernel::application::shared::DomainEvent;
use serde_json::Value;
use uuid::Uuid;
pub fn stale_record_filter(without_limit: Vec<Filter>) -> Filter {
    let mut fallback = vec![Filter::condition(
        ROTTING_AT_FIELD,
        FilterOperator::IsEmpty,
        Value::Null,
    )];
    fallback.extend(without_limit);
    Filter::or(vec![
        Filter::condition(IS_ROTTING_FIELD, FilterOperator::Eq, Value::Bool(true)),
        Filter::and(fallback),
    ])
}

pub const PIPELINE_STAGE_ROTTING_TIGHTENED: &str = "pipeline_stage.rotting_tightened";

pub fn tightened_stage(event: &DomainEvent) -> Option<Uuid> {
    if event.event_type != PIPELINE_STAGE_ROTTING_TIGHTENED {
        return None;
    }
    event
        .payload
        .get("stage_id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

pub fn rotting_catch_up_filter(stage_id: Option<Uuid>) -> Filter {
    let rotting = Filter::condition(IS_ROTTING_FIELD, FilterOperator::Eq, Value::Bool(true));
    match stage_id {
        Some(stage_id) => Filter::and(vec![
            rotting,
            Filter::condition(
                "stage_id",
                FilterOperator::Eq,
                Value::String(stage_id.to_string()),
            ),
        ]),
        None => rotting,
    }
}
