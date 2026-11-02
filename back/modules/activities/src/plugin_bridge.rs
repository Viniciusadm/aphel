use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::ActivityData;
use crate::hooks::CompletingActivity;
use crate::http::ActivityResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::routes::v1::field_permissions::redacted;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, OperationFuture, PluginHook, PluginHookKind, PluginOperation, forward_plugin_hook,
    operation_input,
};

pub(crate) const COMPLETING: &str = "activity.completing";

fn record_json(record: &RecordRef) -> Value {
    json!({
        "record_type": record.record_type.as_str(),
        "record_id": record.record_id,
    })
}

fn activity_json(data: &ActivityData) -> Value {
    json!({
        "title": data.title,
        "activity_type_id": data.activity_type_id,
        "status": data.status.as_str(),
        "priority": data.priority.as_str(),
        "due_at": data.due_at,
        "completed_at": data.completed_at,
        "owner_id": data.owner_id.as_ref().map(|owner| owner.as_str()),
        "record": data.record.as_ref().map(record_json),
        "links": data.links.iter().map(record_json).collect::<Vec<_>>(),
        "duration_minutes": data.duration_minutes,
        "description": data.description,
    })
}

fn completing_json(completing: &CompletingActivity) -> Value {
    let mut activity = activity_json(&completing.data);
    if let Value::Object(map) = &mut activity {
        map.insert("id".to_owned(), json!(completing.id));
    }
    json!({
        "activity": activity,
        "previous": activity_json(&completing.before),
        "type_key": completing.type_key,
    })
}

pub(crate) async fn completing(
    ctx: &mut HookCtx<'_>,
    completing: &CompletingActivity,
) -> Result<(), AppError> {
    forward_plugin_hook(
        ctx,
        COMPLETING,
        "activities.activity",
        completing.id,
        || completing_json(completing),
    )
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![Arc::new(PluginHook::new(
        COMPLETING,
        "activities.activity",
        PluginHookKind::Veto,
        "Before an activity is completed; values: { activity, previous, type_key }. An error blocks it",
    ))]
}

#[derive(Deserialize)]
struct ActivityId {
    id: String,
}

fn complete<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ActivityId = operation_input(input)?;
        let activity = service::complete(ctx, &input.id, None).await?;
        redacted(
            &ctx.state(),
            ctx.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(
        PluginOperation::write(
            "activities.complete",
            "Mark an open activity as done now: { id }",
            complete,
        )
        .with_durable_mutations(),
    )]
}
