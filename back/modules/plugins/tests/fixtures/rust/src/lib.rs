use aphel_plugin_sdk::{
    Ctx, Event, HookCall, HookOutcome, Plugin, PluginError, Request, Response, Result, Value, json,
};

struct ContractPlugin;

impl Plugin for ContractPlugin {
    fn on_hook(&self, ctx: &Ctx, call: &HookCall) -> Result<HookOutcome> {
        let state = ctx.hook_state()?;
        if ctx.setting::<bool>("reject")?.unwrap_or(false) {
            ctx.defer_effect(
                "discarded",
                json!({"kind": "kv_set", "collection": "hooks", "id": "discarded", "value": true}),
            )?;
            return Err(PluginError::new(
                "fixture_rejected",
                "the fixture rejected the write",
            ));
        }
        if state.as_ref().is_some_and(|state| state.effects_allowed) {
            ctx.defer_effect(
                &format!("snapshot:{}", call.hook),
                json!({"kind": "kv_set", "collection": "hooks", "id": call.hook, "value": state}),
            )?;
        }
        if call.hook == "record.saving" && ctx.setting::<bool>("patch")?.unwrap_or(false) {
            return Ok(HookOutcome::default().set("first_name", json!("Filtered")));
        }
        Ok(HookOutcome::default())
    }

    fn on_event(&self, ctx: &Ctx, event: &Event) -> Result<()> {
        ctx.defer_effect("event-write", json!({"kind": "kv_set", "collection": "events", "id": event.id, "value": ctx.execution_context()?}))?;
        if ctx.setting::<bool>("fail")?.unwrap_or(false) {
            return Err(PluginError::new(
                "fixture_failed",
                "the fixture failed after deferring an effect",
            ));
        }
        Ok(())
    }

    fn on_route(&self, ctx: &Ctx, request: &Request) -> Result<Response> {
        if let Some(command) = request.body.get("command") {
            let id = ctx.defer_effect("route-write", command.clone())?;
            return Ok(Response::with_status(
                202,
                json!({"id": id, "execution": ctx.execution_context()?}),
            ));
        }
        Ok(Response::ok(ctx.execution_context()?))
    }

    fn on_schedule(&self, ctx: &Ctx, name: &str, _slot: &str) -> Result<()> {
        ctx.defer_effect("schedule-write", json!({"kind": "kv_set", "collection": "schedules", "id": name, "value": ctx.execution_context()?}))?;
        Ok(())
    }

    fn on_action(&self, ctx: &Ctx, _call: &aphel_plugin_sdk::ActionCall) -> Result<Value> {
        ctx.defer_effect("action-write", json!({"kind": "kv_set", "collection": "actions", "id": "run", "value": ctx.execution_context()?}))?;
        Ok(json!(ctx.execution_context()?))
    }
}

aphel_plugin_sdk::export_plugin!(ContractPlugin);
