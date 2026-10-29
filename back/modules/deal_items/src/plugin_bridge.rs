use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::http::{DealItemRequest, DealItemsResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
struct DealInput {
    deal_id: String,
}

#[derive(Deserialize)]
struct ReplaceInput {
    deal_id: String,
    items: Vec<DealItemRequest>,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: DealInput = operation_input(input)?;
        operation_output(DealItemsResponse::from(
            service::summary(ctx, &input.deal_id).await?,
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
        operation_output(DealItemsResponse::from(
            service::replace(ctx, &input.deal_id, inputs, None).await?,
        ))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "deal_items.list",
            "Line items and totals of a deal: { deal_id }",
            list,
        )),
        Arc::new(PluginOperation::write(
            "deal_items.replace",
            "Replace all line items of a deal: { deal_id, items: [{ product_id?, name?, unit?, quantity?, unit_price?, discount_type?: none|percent|amount, discount_value?, recurrence?: none|monthly|quarterly|yearly, term_months? }] }",
            replace,
        )
        .with_durable_mutations()),
    ]
}
