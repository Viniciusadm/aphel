use std::sync::Arc;

use serde_json::{Value, json};

use crate::hooks::RunningAction;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{HookCtx, PluginHook, PluginHookKind, forward_plugin_hook};

pub(crate) const ACTION_RUNNING: &str = "automation.action_running";

fn running_json(running: &RunningAction) -> Value {
    json!({
        "automation_id": running.automation_id,
        "run_id": running.run_id,
        "step_id": running.step_id,
        "action": running.action,
        "record": {
            "record_type": running.record.record_type.as_str(),
            "record_id": running.record.record_id,
        },
        "config": running.config,
        "dry_run": running.dry_run,
    })
}

pub(crate) async fn action_running(
    ctx: &mut HookCtx<'_>,
    running: &RunningAction,
) -> Result<(), AppError> {
    forward_plugin_hook(
        ctx,
        ACTION_RUNNING,
        running.record.record_type.as_str(),
        running.record.record_id,
        || running_json(running),
    )
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![Arc::new(PluginHook::new(
        ACTION_RUNNING,
        "*",
        PluginHookKind::Veto,
        "Before an automation action runs on a record (dry run and execution), for the record's type; values: { automation_id, run_id, step_id, action, record: { record_type, record_id }, config, dry_run }. An error skips the action",
    ))]
}
