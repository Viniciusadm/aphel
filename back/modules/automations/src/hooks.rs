use serde_json::{Value, json};
use uuid::Uuid;

use crate::KEY;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{Hook, PluginHookArgs};
use aphel_sdk::platform::application::shared::RecordRef;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunningAction {
    pub(crate) automation_id: Uuid,
    pub(crate) run_id: Uuid,
    pub(crate) step_id: String,
    pub(crate) action: String,
    pub(crate) record: RecordRef,
    pub(crate) config: Value,
    pub(crate) dry_run: bool,
    pub(crate) output: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActionRunning;

pub(crate) const ACTION_RUNNING: &str = "automation.action_running";

impl PluginHookArgs for RunningAction {
    fn plugin_record_id(&self) -> String {
        self.record.record_id.to_string()
    }

    fn plugin_record_type(&self) -> Option<String> {
        Some(self.record.record_type.as_str().to_owned())
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "automation_id": self.automation_id,
            "run_id": self.run_id,
            "step_id": self.step_id,
            "action": self.action,
            "record": {
                "record_type": self.record.record_type.as_str(),
                "record_id": self.record.record_id,
            },
            "config": self.config,
            "dry_run": self.dry_run,
        })
    }
}

impl Hook for ActionRunning {
    type Args = RunningAction;
}

pub(crate) async fn before_action(ctx: &Ctx<'_>, running: &RunningAction) -> Result<(), AppError> {
    if !ctx
        .app()
        .hooks()
        .has_actions_except(ActionRunning, Some(KEY))
        && !aphel_sdk::platform::modules::plugin_bridge::plugin_hook_subscribed(
            ctx,
            ACTION_RUNNING,
            running.record.record_type.as_str(),
        )
    {
        return Ok(());
    }
    let mut tx = ctx.begin().await?;
    ctx.run_hook(ActionRunning, &mut tx, KEY, running).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn after_action(ctx: &Ctx<'_>, running: RunningAction) -> Result<Value, AppError> {
    if !ctx.app().hooks().has_filters_except(ActionRunning, None) {
        return Ok(running.output.unwrap_or(Value::Null));
    }
    let mut tx = ctx.begin().await?;
    let filtered = ctx.apply_hook(ActionRunning, &mut tx, KEY, running).await?;
    tx.commit().await?;
    Ok(filtered.output.unwrap_or(Value::Null))
}
