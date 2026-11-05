use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde_json::Value;

use crate::http::{LogCallRequest, activity_json};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input};

fn log<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: LogCallRequest = operation_input(input)?;
        let record = service::log(ctx, request.into_command(None)).await?;
        activity_json(&ctx.runtime(), ctx.scope, record).await
    })
}

const OP_TELEPHONY_LOG: OperationKey<Value, Value> = operation!("telephony.log");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_TELEPHONY_LOG, OperationSpec::write("Log a call as an activity: { outcome_key, record_type?, record_id?, direction?: inbound|outbound, duration_seconds?, phone_number?, recording_url?, title?, notes?, occurred_at?, provider?, provider_call_id?, custom_fields? } -> the activity").durable(), log);
}
