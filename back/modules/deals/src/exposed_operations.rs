use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::http::{DealResponse, MoveDealPipelineRequest, MoveDealRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

#[derive(Deserialize)]
struct MoveStageInput {
    id: String,
    #[serde(flatten)]
    request: MoveDealRequest,
}

#[derive(Deserialize)]
struct MovePipelineInput {
    id: String,
    #[serde(flatten)]
    request: MoveDealPipelineRequest,
}

fn move_stage<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: MoveStageInput = operation_input(input)?;
        let deal = service::move_to_stage(ctx, &input.id, input.request.into_command(None)).await?;
        operation_output(DealResponse::from(deal))
    })
}

fn move_pipeline<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: MovePipelineInput = operation_input(input)?;
        let deal =
            service::move_to_pipeline(ctx, &input.id, input.request.into_command(None)).await?;
        operation_output(DealResponse::from(deal))
    })
}

const OP_DEALS_MOVE_STAGE: OperationKey<Value, Value> = operation!("deals.move_stage");
const OP_DEALS_MOVE_PIPELINE: OperationKey<Value, Value> = operation!("deals.move_pipeline");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_DEALS_MOVE_STAGE, OperationSpec::write("Move a deal to a stage (a won/lost stage wins/loses it): { id, stage_id, lost_reason_id?, lost_reason_note?, fields? }").durable(), move_stage);
    app.operation(OP_DEALS_MOVE_PIPELINE, OperationSpec::write("Move a deal to another pipeline: { id, pipeline_id, stage_id?, lost_reason_id?, lost_reason_note?, fields? }").durable(), move_pipeline);
}
