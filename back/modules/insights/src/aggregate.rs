use serde_json::{Value, json};

use ::dashboard::engine::{DashboardPorts, WidgetContext};
use aphel_kernel::api::dto::aggregates::{AggregateQueryRequest, decimal};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::query::{AggregateEngine, RecordQueryPorts};
use aphel_kernel::errors::AppError;

pub(crate) const FIELD_NOT_ACCESSIBLE: &str = "field_not_accessible";

pub(crate) fn condition(field: &str, operator: &str, value: Value) -> Value {
    json!({ "field": field, "operator": operator, "value": value })
}

pub(crate) fn combine(operator: &str, mut conditions: Vec<Value>) -> Option<Value> {
    match conditions.len() {
        0 => None,
        1 => conditions.pop(),
        _ => Some(json!({ "op": operator, "conditions": conditions })),
    }
}

pub(crate) fn with_subject<P: DashboardPorts>(
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

pub(crate) fn period<P: DashboardPorts>(context: &WidgetContext<'_, P>, field: &str) -> Value {
    json!({
        "field": field,
        "preset": "custom",
        "from": context.period.from.format("%Y-%m-%d").to_string(),
        "to": context.period.to.format("%Y-%m-%d").to_string(),
    })
}

pub(crate) fn error_code(error: &AppError) -> Option<&'static str> {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => Some(code),
        _ => None,
    }
}

pub(crate) fn is_field_not_accessible(error: &AppError) -> bool {
    error_code(error) == Some(FIELD_NOT_ACCESSIBLE)
}

pub(crate) struct VisibleField {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field_type: String,
}

pub(crate) async fn visible_fields<P: DashboardPorts>(
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

pub(crate) fn has_field(fields: &[VisibleField], key: &str) -> bool {
    fields.iter().any(|field| field.key == key)
}

pub(crate) struct GroupValue {
    pub(crate) value: Option<String>,
    pub(crate) label: Option<String>,
}

pub(crate) struct AggregateRow {
    pub(crate) groups: Vec<GroupValue>,
    pub(crate) values: Vec<Option<String>>,
}

pub(crate) struct AggregateOutput {
    pub(crate) rows: Vec<AggregateRow>,
    pub(crate) totals: Vec<Option<String>>,
    pub(crate) truncated: bool,
}

pub(crate) async fn run<P: DashboardPorts>(
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

pub(crate) fn count_at(values: &[Option<String>], index: usize) -> u64 {
    values
        .get(index)
        .and_then(Option::as_deref)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.max(0.0).round() as u64)
        .unwrap_or(0)
}

pub(crate) fn text_at(values: &[Option<String>], index: usize) -> Option<String> {
    values.get(index).cloned().flatten()
}
