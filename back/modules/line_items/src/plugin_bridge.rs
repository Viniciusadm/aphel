use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::http::{LineItemRequest, LineItemsResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
struct RecordInput {
    record_type: String,
    record_id: String,
}

#[derive(Deserialize)]
struct ReplaceInput {
    record_type: String,
    record_id: String,
    items: Vec<LineItemRequest>,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: RecordInput = operation_input(input)?;
        operation_output(LineItemsResponse::from(
            service::summary(
                ctx,
                &aphel_kernel::application::shared::RecordRef::parse(
                    &input.record_type,
                    &input.record_id,
                )?,
            )
            .await?,
        ))
    })
}

fn replace<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ReplaceInput = operation_input(input)?;
        let inputs = input
            .items
            .into_iter()
            .map(|item| item.into_command(None, None).input)
            .collect();
        operation_output(LineItemsResponse::from(
            service::replace(
                ctx,
                &aphel_kernel::application::shared::RecordRef::parse(
                    &input.record_type,
                    &input.record_id,
                )?,
                inputs,
                None,
            )
            .await?,
        ))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "line_items.list",
            "Line items and totals of a record: { record_type, record_id }",
            list,
        )),
        Arc::new(PluginOperation::write(
            "line_items.replace",
            "Replace all line items of a record: { record_type, record_id, items: [{ product_id?, name?, unit?, quantity?, unit_price?, discount_type?: none|percent|amount, discount_value?, tax_percent?, recurrence?: none|monthly|quarterly|yearly, term_months? }] }",
            replace,
        )
        .with_durable_mutations()),
    ]
}
