use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::http::{LineItemRequest, LineItemsResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

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

const OP_LINE_ITEMS_LIST: OperationKey<Value, Value> = operation!("line_items.list");
const OP_LINE_ITEMS_REPLACE: OperationKey<Value, Value> = operation!("line_items.replace");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(
        OP_LINE_ITEMS_LIST,
        OperationSpec::read("Line items and totals of a record: { record_type, record_id }"),
        list,
    );
    app.operation(OP_LINE_ITEMS_REPLACE, OperationSpec::write("Replace all line items of a record: { record_type, record_id, items: [{ product_id?, name?, unit?, quantity?, unit_price?, discount_type?: none|percent|amount, discount_value?, tax_percent?, recurrence?: none|monthly|quarterly|yearly, term_months? }] }").durable(), replace);
}
