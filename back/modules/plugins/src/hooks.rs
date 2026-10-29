use serde_json::{Map, Value};

use crate::manifest::{HOOK_DELETING, HOOK_SAVED, HOOK_SAVING, HOOK_VALIDATING};
use crate::registry;
use crate::runtime::{on_hook, scope_actor, wit};
use aphel_kernel::application::fields::WriteMode;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::{ChangedRecord, RemovedRecord, SavingRecord};

const EMPTY_OBJECT: &str = "{}";

fn mode(mode: WriteMode) -> &'static str {
    match mode {
        WriteMode::Create => "create",
        WriteMode::Update => "update",
    }
}

fn changed_call(hook: &str, actor: wit::Actor, changed: &ChangedRecord) -> wit::HookCall {
    wit::HookCall {
        hook: hook.to_owned(),
        record_type: changed.record_type.as_str().to_owned(),
        record_id: changed.record.record_id.to_string(),
        mode: Some(mode(changed.mode).to_owned()),
        actor,
        changes: changed
            .changes
            .iter()
            .map(|change| wit::Change {
                field: change.field.clone(),
                old_json: change.old.to_string(),
                new_json: change.new.to_string(),
            })
            .collect(),
        values_json: changed.values.to_string(),
        custom_fields_json: EMPTY_OBJECT.to_owned(),
    }
}

async fn notify(
    ctx: &mut HookCtx<'_>,
    record_type: &str,
    call: wit::HookCall,
) -> Result<(), AppError> {
    let snapshot = registry::current(ctx.pool).await?;
    for plugin in snapshot.hook_subscribers(&call.hook, record_type) {
        on_hook(&plugin, ctx, &call).await?;
    }
    Ok(())
}

fn merge(target: &mut Map<String, Value>, patch: &str) {
    if let Ok(Value::Object(patch)) = serde_json::from_str::<Value>(patch) {
        for (key, value) in patch {
            target.insert(key, value);
        }
    }
}

pub(crate) async fn saving(
    ctx: &mut HookCtx<'_>,
    mut saving: SavingRecord,
) -> Result<SavingRecord, AppError> {
    let snapshot = registry::current(ctx.pool).await?;
    for plugin in snapshot.hook_subscribers(HOOK_SAVING, saving.record_type.as_str()) {
        let call = wit::HookCall {
            hook: HOOK_SAVING.to_owned(),
            record_type: saving.record_type.as_str().to_owned(),
            record_id: saving.draft.record.record_id.to_string(),
            mode: Some(mode(saving.draft.mode).to_owned()),
            actor: scope_actor(ctx.scope),
            changes: Vec::new(),
            values_json: Value::Object(saving.draft.values.clone()).to_string(),
            custom_fields_json: Value::Object(saving.draft.custom_fields.clone()).to_string(),
        };
        let outcome = on_hook(&plugin, ctx, &call).await?;
        merge(&mut saving.draft.values, &outcome.values_json);
        merge(&mut saving.draft.custom_fields, &outcome.custom_fields_json);
    }
    Ok(saving)
}

pub(crate) async fn validating(
    ctx: &mut HookCtx<'_>,
    changed: &ChangedRecord,
) -> Result<(), AppError> {
    notify(
        ctx,
        changed.record_type.as_str(),
        changed_call(HOOK_VALIDATING, scope_actor(ctx.scope), changed),
    )
    .await
}

pub(crate) async fn saved(ctx: &mut HookCtx<'_>, changed: &ChangedRecord) -> Result<(), AppError> {
    notify(
        ctx,
        changed.record_type.as_str(),
        changed_call(HOOK_SAVED, scope_actor(ctx.scope), changed),
    )
    .await?;
    crate::committed::enqueue(ctx, changed).await
}

pub(crate) async fn deleting(
    ctx: &mut HookCtx<'_>,
    record: &RemovedRecord,
) -> Result<(), AppError> {
    removed(ctx, HOOK_DELETING, record).await
}

pub(crate) async fn removed(
    ctx: &mut HookCtx<'_>,
    hook: &str,
    removed: &RemovedRecord,
) -> Result<(), AppError> {
    let call = wit::HookCall {
        hook: hook.to_owned(),
        record_type: removed.record_type.as_str().to_owned(),
        record_id: removed.record.record_id.to_string(),
        mode: None,
        actor: scope_actor(ctx.scope),
        changes: Vec::new(),
        values_json: EMPTY_OBJECT.to_owned(),
        custom_fields_json: EMPTY_OBJECT.to_owned(),
    };
    notify(ctx, removed.record_type.as_str(), call).await
}
