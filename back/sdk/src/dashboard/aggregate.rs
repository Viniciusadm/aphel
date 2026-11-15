use serde_json::{Value, json};

use crate::dashboard::{DashboardPorts, WidgetContext};
use crate::platform::api::dto::aggregates::{AggregateQueryRequest, decimal};
use crate::platform::application::fields::FieldSet;
use crate::platform::application::query::{AggregateEngine, RecordQueryPorts};
use aphel_kernel::errors::AppError;

pub const FIELD_NOT_ACCESSIBLE: &str = "field_not_accessible";

pub fn condition(field: &str, operator: &str, value: Value) -> Value {
    json!({ "field": field, "operator": operator, "value": value })
}

pub fn combine(operator: &str, mut conditions: Vec<Value>) -> Option<Value> {
    match conditions.len() {
        0 => None,
        1 => conditions.pop(),
        _ => Some(json!({ "op": operator, "conditions": conditions })),
    }
}

pub fn with_subject<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    mut conditions: Vec<Value>,
) -> Result<Option<Value>, AppError> {
    if let Some(subject) = context.subject_filter() {
        conditions.push(
            serde_json::to_value(subject).map_err(|error| AppError::Internal(error.to_string()))?,
        );
    }
    Ok(combine("and", conditions))
}

pub fn period<P: DashboardPorts>(context: &WidgetContext<'_, P>, field: &str) -> Value {
    json!({
        "field": field,
        "preset": "custom",
        "from": context.period.from.format("%Y-%m-%d").to_string(),
        "to": context.period.to.format("%Y-%m-%d").to_string(),
    })
}

pub fn error_code(error: &AppError) -> Option<&'static str> {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => Some(code),
        _ => None,
    }
}

pub fn is_field_not_accessible(error: &AppError) -> bool {
    error_code(error) == Some(FIELD_NOT_ACCESSIBLE)
}

pub struct VisibleField {
    pub key: String,
    pub label: String,
    pub field_type: String,
}

pub async fn visible_fields<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    record_type: &str,
) -> Result<Vec<VisibleField>, AppError> {
    let query = context.ports.query();
    let definition = query.registry().require(record_type)?;
    let fields = FieldSet::for_scope(query.definitions(), definition, context.scope).await?;
    Ok(fields
        .active()
        .map(|field| VisibleField {
            key: field.key().to_owned(),
            label: field.definition.label.clone(),
            field_type: field.definition.type_name().to_owned(),
        })
        .collect())
}

pub fn has_field(fields: &[VisibleField], key: &str) -> bool {
    fields.iter().any(|field| field.key == key)
}

pub struct GroupValue {
    pub value: Option<String>,
    pub label: Option<String>,
}

pub struct AggregateRow {
    pub groups: Vec<GroupValue>,
    pub values: Vec<Option<String>>,
}

pub struct AggregateOutput {
    pub rows: Vec<AggregateRow>,
    pub totals: Vec<Option<String>>,
    pub truncated: bool,
}

pub async fn run<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    body: Value,
) -> Result<AggregateOutput, AppError> {
    let request: AggregateQueryRequest = serde_json::from_value(body)
        .map_err(|error| AppError::validation_code("invalid_aggregate", error.to_string()))?;
    let result = AggregateEngine::new(context.ports.query())
        .run(context.scope, request.into_query())
        .await?;
    Ok(AggregateOutput {
        rows: result
            .rows
            .into_iter()
            .map(|row| AggregateRow {
                groups: row
                    .groups
                    .into_iter()
                    .map(|group| GroupValue {
                        value: group.value,
                        label: group.label,
                    })
                    .collect(),
                values: row.values.into_iter().map(decimal).collect(),
            })
            .collect(),
        totals: result.totals.values.into_iter().map(decimal).collect(),
        truncated: result.truncated,
    })
}

pub fn count_at(values: &[Option<String>], index: usize) -> u64 {
    values
        .get(index)
        .and_then(Option::as_deref)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.max(0.0).round() as u64)
        .unwrap_or(0)
}

pub fn text_at(values: &[Option<String>], index: usize) -> Option<String> {
    values.get(index).cloned().flatten()
}
