use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::http::ActivityResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
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
const OP_TASKS_CREATE: OperationKey<Value, Value> = operation!("tasks.create");
const OP_TASKS_TYPES: OperationKey<Value, Value> = operation!("tasks.types");

#[derive(Deserialize)]
struct CreateInput {
    title: String,
    activity_type_id: String,
    due_at: Option<String>,
    owner_id: Option<String>,
    record_type: Option<String>,
    record_id: Option<String>,
    description: Option<String>,
    duration_minutes: Option<i64>,
}

fn create<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: CreateInput = operation_input(input)?;
        let writer = aphel_sdk::platform::modules::app()
            .services()
            .get::<dyn aphel_sdk::tasks::TaskWriter>()
            .ok_or_else(|| AppError::Internal("task writer is not registered".to_owned()))?;
        let id = writer
            .create(
                ctx,
                aphel_sdk::tasks::CreateActivityCommand {
                    title: input.title,
                    activity_type_id: input.activity_type_id,
                    due_at: input.due_at,
                    owner_id: input.owner_id,
                    record_type: input.record_type,
                    record_id: input.record_id,
                    description: input.description,
                    duration_minutes: input.duration_minutes,
                    ..aphel_sdk::tasks::CreateActivityCommand::default()
                },
            )
            .await?;
        Ok(serde_json::json!({ "id": id }))
    })
}

fn types<'a>(ctx: &'a Ctx<'_>, _input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let reader = aphel_sdk::platform::modules::app()
            .services()
            .get::<dyn aphel_sdk::tasks::TaskTypes>()
            .ok_or_else(|| AppError::Internal("task types are not registered".to_owned()))?;
        let listing = reader
            .list(
                ctx,
                false,
                aphel_sdk::platform::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 200,
                },
            )
            .await?;
        Ok(serde_json::json!({
            "items": listing
                .items
                .iter()
                .map(|item| serde_json::json!({ "id": item.id, "key": item.key, "name": item.name }))
                .collect::<Vec<_>>(),
        }))
    })
}

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(
        OP_TASKS_COMPLETE,
        OperationSpec::write("Mark an open activity as done now: { id }").durable(),
        complete,
    );
    app.operation(
        OP_TASKS_CREATE,
        OperationSpec::write(
            "Create a task: { title, activity_type_id, due_at?, owner_id?, record_type?, record_id?, description?, duration_minutes? } -> { id }",
        ),
        create,
    );
    app.operation(
        OP_TASKS_TYPES,
        OperationSpec::read("Active task types: {} -> { items: [{ id, key, name }] }"),
        types,
    );
}
