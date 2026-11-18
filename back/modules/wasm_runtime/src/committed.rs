use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    effects::Authority,
    registry,
    runtime::{self, wit},
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::ChangedRecord;
use aphel_sdk::platform::application::shared::{
    Actor, ExecutionContext, Job, JobContext, JobHandler, JobQueue,
};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue;

pub const RUN_JOB: &str = "plugin.committed";

pub async fn enqueue(ctx: &mut HookCtx<'_>, changed: &ChangedRecord) -> Result<(), AppError> {
    let snapshot = registry::current(ctx.pool).await?;
    let parent = ExecutionContext::current();
    for plugin in snapshot.hook_subscribers("record.committed", changed.record_type.as_str()) {
        if parent
            .as_ref()
            .is_some_and(|parent| parent.chain.contains(&plugin.key))
        {
            continue;
        }
        let actor = Actor::from_scope(ctx.scope);
        let execution = ExecutionContext::plugin(
            &plugin.key,
            "committed",
            Some(&actor),
            parent.as_ref(),
            None,
            Uuid::now_v7(),
        )?;
        let scope = if ctx.scope.is_system() {
            runtime::plugin_scope(&plugin).await?
        } else {
            runtime::capability_scope(&plugin, ctx.scope)
        };
        let authority = Authority::capture(&scope);
        let mut state = ctx.tx.record_states.get(&(changed.record_type.as_str().to_owned(), changed.record.record_id)).cloned()
            .unwrap_or_else(|| json!({"operation_id": ctx.tx.operation_id, "record_type": changed.record_type.as_str(),
                "record_id": changed.record.record_id, "before": null, "before_available": false, "proposed": null}));
        state["phase"] = json!("record.committed");
        state["persisted"] = state.get("proposed").cloned().unwrap_or(Value::Null);
        let source = format!(
            "committed:{}:{}:{}",
            ctx.tx.operation_id,
            changed.record_type.as_str(),
            changed.record.record_id
        );
        let input = json!({"record_type": changed.record_type.as_str(), "record_id": changed.record.record_id,
            "mode": match changed.mode { aphel_sdk::platform::application::fields::WriteMode::Create => "create", aphel_sdk::platform::application::fields::WriteMode::Update => "update" },
            "actor_type": actor.kind(), "actor_id": actor.id(), "values": changed.values,
            "changes": changed.changes.iter().map(|change| json!({"field": change.field, "old": change.old, "new": change.new})).collect::<Vec<_>>(),
            "state": state, "authority": authority});
        let id = crate::store::executions::record_once(
            ctx.tx,
            &execution.id.to_string(),
            &plugin.key,
            &source,
            &input,
            &execution,
        )
        .await?;
        MySqlJobQueue::new(ctx.pool.clone())
            .enqueue(
                ctx.tx,
                Job::new(RUN_JOB, json!({"id": id}))
                    .with_unique_key(format!("plugin.committed:{id}")),
            )
            .await?;
    }
    Ok(())
}

pub struct Run {
    pool: Db,
}

impl Run {
    pub fn shared(pool: Db) -> Arc<dyn JobHandler> {
        Arc::new(Self { pool })
    }
}

#[async_trait]
impl JobHandler for Run {
    fn kind(&self) -> &str {
        RUN_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let id = payload
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("plugin.committed requires an id".to_owned()))?;
        let mut tx = Tx::from(self.pool.begin().await?);
        let row = crate::store::executions::lock(&mut tx, id).await?;
        if row.status == "completed" {
            return Ok(());
        }
        let key = row.plugin_key;
        let snapshot = registry::current(&self.pool).await?;
        let plugin = snapshot.plugin(&key).ok_or_else(|| {
            AppError::unavailable_code("plugin_disabled", "the plugin is disabled or uninstalled")
        })?;
        let input = row.input;
        let execution: ExecutionContext = serde_json::from_value(row.execution)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let authority: Authority = serde_json::from_value(input["authority"].clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let scope = authority
            .resolve(&self.pool)
            .await?
            .restricted(|permission| plugin.manifest.allows(permission))
            .acting_as("plugin", &plugin.key);
        let text = |key: &str| {
            input
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let record_type = text("record_type");
        let record_id = Uuid::parse_str(&text("record_id"))
            .map_err(|_| AppError::Internal("invalid committed record id".to_owned()))?;
        tx.record_states
            .insert((record_type.clone(), record_id), input["state"].clone());
        let actor = Actor::from_parts(
            &text("actor_type"),
            input
                .get("actor_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )?;
        let call = wit::HookCall {
            hook: "record.committed".to_owned(),
            record_type,
            record_id: record_id.to_string(),
            mode: Some(text("mode")),
            actor: runtime::event_actor(&actor),
            values_json: input["values"].to_string(),
            custom_fields_json: "{}".to_owned(),
            changes: input["changes"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|change| wit::Change {
                    field: change["field"].as_str().unwrap_or_default().to_owned(),
                    old_json: change["old"].to_string(),
                    new_json: change["new"].to_string(),
                })
                .collect(),
        };
        let app = aphel_sdk::platform::modules::app();
        let mut ctx = HookCtx::new(&mut tx, &self.pool, &scope, app.services(), crate::KEY);
        execution
            .scope(runtime::on_hook(plugin, &mut ctx, &call))
            .await?;
        crate::store::executions::complete(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }
}
