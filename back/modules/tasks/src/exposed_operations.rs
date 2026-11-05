use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::http::ActivityResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input};
use aphel_sdk::platform::api::routes::v1::field_permissions::redacted;
use aphel_sdk::platform::application::shared::RecordType;

#[derive(Deserialize)]
struct ActivityId {
    id: String,
}

fn complete<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ActivityId = operation_input(input)?;
        let activity = service::complete(ctx, &input.id, None).await?;
        redacted(
            &ctx.runtime(),
            ctx.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await
    })
}

const OP_TASKS_COMPLETE: OperationKey<Value, Value> = operation!("tasks.complete");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(
        OP_TASKS_COMPLETE,
        OperationSpec::write("Mark an open activity as done now: { id }").durable(),
        complete,
    );
}
