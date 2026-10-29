use std::sync::Arc;

use serde_json::Value;

use crate::http::{LogCallRequest, activity_json};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input};

fn log<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: LogCallRequest = operation_input(input)?;
        let record = service::log(ctx, request.into_command(None)).await?;
        activity_json(&ctx.state(), ctx.scope, record).await
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(PluginOperation::write(
        "calls.log",
        "Log a call as an activity: { outcome_key, record_type?, record_id?, direction?: inbound|outbound, duration_seconds?, phone_number?, recording_url?, title?, notes?, occurred_at?, provider?, provider_call_id?, custom_fields? } -> the activity",
        log,
    )
        .with_durable_mutations())]
}
