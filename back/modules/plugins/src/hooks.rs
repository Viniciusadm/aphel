use serde_json::{Map, Value, json};

use crate::manifest::{HOOK_DELETING, HOOK_SAVED, HOOK_SAVING, HOOK_VALIDATING};
use crate::registry;
use crate::runtime::{HOOK_TIMEOUT, invoke};
use aphel_kernel::application::fields::WriteMode;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::{ChangedRecord, RemovedRecord, SavingRecord};

fn mode(mode: WriteMode) -> &'static str {
    match mode {
        WriteMode::Create => "create",
        WriteMode::Update => "update",
    }
}

fn changed_input(hook: &str, changed: &ChangedRecord) -> Value {
    json!({
        "kind": "hook",
        "hook": hook,
        "record_type": changed.record_type.as_str(),
        "record_id": changed.record.record_id.to_string(),
        "mode": mode(changed.mode),
        "changes": changed
            .changes
            .iter()
            .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
            .collect::<Vec<_>>(),
        "values": changed.values,
    })
}

async fn notify(
    ctx: &mut HookCtx<'_>,
    hook: &str,
    record_type: &str,
    input: Value,
) -> Result<(), AppError> {
    let snapshot = registry::current(ctx.pool).await?;
    for plugin in snapshot.hook_subscribers(hook, record_type) {
        invoke(&plugin, ctx.scope, &input, HOOK_TIMEOUT).await?;
    }
    Ok(())
}

pub(crate) async fn saving(
    ctx: &mut HookCtx<'_>,
    mut saving: SavingRecord,
) -> Result<SavingRecord, AppError> {
    let snapshot = registry::current(ctx.pool).await?;
    let subscribers = snapshot.hook_subscribers(HOOK_SAVING, saving.record_type.as_str());
    for plugin in subscribers {
        let input = json!({
            "kind": "hook",
            "hook": HOOK_SAVING,
            "record_type": saving.record_type.as_str(),
            "record_id": saving.draft.record.record_id.to_string(),
            "mode": mode(saving.draft.mode),
            "values": saving.draft.values,
            "custom_fields": saving.draft.custom_fields,
        });
        let output = invoke(&plugin, ctx.scope, &input, HOOK_TIMEOUT).await?;
        merge(&mut saving.draft.values, output.get("values"));
        merge(&mut saving.draft.custom_fields, output.get("custom_fields"));
    }
    Ok(saving)
}

fn merge(target: &mut Map<String, Value>, patch: Option<&Value>) {
    if let Some(Value::Object(patch)) = patch {
        for (key, value) in patch {
            target.insert(key.clone(), value.clone());
        }
    }
}

pub(crate) async fn validating(
    ctx: &mut HookCtx<'_>,
    changed: &ChangedRecord,
) -> Result<(), AppError> {
    notify(
        ctx,
        HOOK_VALIDATING,
        changed.record_type.as_str(),
        changed_input(HOOK_VALIDATING, changed),
    )
    .await
}

pub(crate) async fn saved(ctx: &mut HookCtx<'_>, changed: &ChangedRecord) -> Result<(), AppError> {
    notify(
        ctx,
        HOOK_SAVED,
        changed.record_type.as_str(),
        changed_input(HOOK_SAVED, changed),
    )
    .await
}

pub(crate) async fn deleting(
    ctx: &mut HookCtx<'_>,
    removed: &RemovedRecord,
) -> Result<(), AppError> {
    let input = json!({
        "kind": "hook",
        "hook": HOOK_DELETING,
        "record_type": removed.record_type.as_str(),
        "record_id": removed.record.record_id.to_string(),
    });
    notify(ctx, HOOK_DELETING, removed.record_type.as_str(), input).await
}
