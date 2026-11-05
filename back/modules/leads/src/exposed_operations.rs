use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::http::{ConvertLeadRequest, DisqualifyLeadRequest, LeadActionResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

#[derive(Deserialize)]
struct ContactId {
    contact_id: String,
}

#[derive(Deserialize)]
struct DisqualifyInput {
    contact_id: String,
    #[serde(flatten)]
    request: DisqualifyLeadRequest,
}

#[derive(Deserialize)]
struct ConvertInput {
    contact_id: String,
    #[serde(flatten)]
    request: ConvertLeadRequest,
}

fn qualify<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ContactId = operation_input(input)?;
        let result = service::qualify_lead(ctx, &input.contact_id, None).await?;
        operation_output(LeadActionResponse::from(result))
    })
}

fn disqualify<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: DisqualifyInput = operation_input(input)?;
        let result =
            service::disqualify_lead(ctx, &input.contact_id, input.request.into_command(None))
                .await?;
        operation_output(LeadActionResponse::from(result))
    })
}

fn convert<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ConvertInput = operation_input(input)?;
        let result =
            service::convert_lead(ctx, &input.contact_id, input.request.into_command(None)).await?;
        operation_output(LeadActionResponse::from(result))
    })
}

const OP_LEADS_QUALIFY: OperationKey<Value, Value> = operation!("leads.qualify");
const OP_LEADS_DISQUALIFY: OperationKey<Value, Value> = operation!("leads.disqualify");
const OP_LEADS_CONVERT: OperationKey<Value, Value> = operation!("leads.convert");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_LEADS_QUALIFY, OperationSpec::write("Qualify a contact in the lead stage: { contact_id } -> { contact_id, lifecycle_stage, deal_id }").durable(), qualify);
    app.operation(OP_LEADS_DISQUALIFY, OperationSpec::write("Disqualify an open lead with a reason: { contact_id, reason_id, note? } -> { contact_id, lifecycle_stage, deal_id }").durable(), disqualify);
    app.operation(OP_LEADS_CONVERT, OperationSpec::write("Convert an open lead into a deal: { contact_id, pipeline_id, stage_id?, title?, amount?, currency? } -> { contact_id, lifecycle_stage, deal_id }").durable(), convert);
}
