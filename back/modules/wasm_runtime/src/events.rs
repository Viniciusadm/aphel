use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::registry;
use crate::runtime::{event_actor, on_event, wit};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{Actor, DomainEvent, EventConsumer};
use aphel_sdk::platform::application::shared::{ExecutionContext, JobContext, JobHandler};
use sqlx::{Row, types::Json};
use std::sync::Arc;

pub(crate) const EVENTS_CONSUMER: &str = "wasm_runtime.events";

pub(crate) struct PluginEventsConsumer {
    pool: MySqlPool,
}

impl PluginEventsConsumer {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        registry::spawn_refresh(pool.clone());
        Self { pool }
    }
}

#[async_trait]
impl EventConsumer for PluginEventsConsumer {
    fn name(&self) -> &str {
        EVENTS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        registry::has_event_consumers(event_type).unwrap_or(true)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let snapshot = registry::current(&self.pool).await?;
        let consumers = snapshot.event_consumers(&event.event_type);
        if consumers.is_empty() {
            return Ok(());
        }
        let mut tx =
            aphel_sdk::platform::database::transaction::MySqlTx::from(self.pool.begin().await?);
        for plugin in consumers {
            if matches!(&event.actor, Actor::Service { kind, id } if kind == Actor::PLUGIN && *id == plugin.key)
                || event
                    .execution
                    .as_ref()
                    .is_some_and(|execution| execution.chain.contains(&plugin.key))
            {
                continue;
            }
            let origin = event
                .execution
                .clone()
                .unwrap_or_else(|| ExecutionContext::event_origin(event.id, &event.actor));
            let execution = ExecutionContext::plugin(
                &plugin.key,
                "event",
                Some(&event.actor),
                Some(&origin),
                Some(event.id),
                uuid::Uuid::now_v7(),
            )?;
            let input = serde_json::json!({
                "id": event.id, "event_type": event.event_type,
                "record_type": event.record.as_ref().map(|record| record.record_type.as_str()),
                "record_id": event.record.as_ref().map(|record| record.record_id.to_string()),
                "actor_type": event.actor.kind(), "actor_id": event.actor.id(),
                "occurred_at": event.occurred_at.to_rfc3339(), "payload": event.payload,
            });
            sqlx::query("INSERT IGNORE INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, ?, ?)")
                .bind(execution.id.to_string()).bind(&plugin.key).bind(format!("event:{}", event.id))
                .bind(sqlx::types::Json(input)).bind(sqlx::types::Json(&execution)).execute(&mut *tx).await?;
            let id: String = sqlx::query_scalar(
                "SELECT id FROM plugins_executions WHERE plugin_key = ? AND source_key = ?",
            )
            .bind(&plugin.key)
            .bind(format!("event:{}", event.id))
            .fetch_one(&mut *tx)
            .await?;
            use aphel_sdk::platform::application::shared::JobQueue;
            aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue::new(self.pool.clone())
                .enqueue(
                    &mut tx,
                    aphel_sdk::platform::application::shared::Job::new(
                        RUN_JOB,
                        serde_json::json!({"id": id}),
                    )
                    .with_unique_key(format!("plugin.event:{id}")),
                )
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

pub const RUN_JOB: &str = "plugin.event";

pub struct Run {
    pool: MySqlPool,
}

impl Run {
    pub fn shared(pool: MySqlPool) -> Arc<dyn JobHandler> {
        Arc::new(Self { pool })
    }
}

#[async_trait]
impl JobHandler for Run {
    fn kind(&self) -> &str {
        RUN_JOB
    }

    async fn handle(
        &self,
        _context: &JobContext,
        payload: serde_json::Value,
    ) -> Result<(), AppError> {
        let id = payload
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::Validation("plugin.event requires an id".to_owned()))?;
        let mut tx =
            aphel_sdk::platform::database::transaction::MySqlTx::from(self.pool.begin().await?);
        let row = sqlx::query("SELECT plugin_key, input, execution, status FROM plugins_executions WHERE id = ? FOR UPDATE")
            .bind(id).fetch_one(&mut *tx).await?;
        if row.try_get::<String, _>("status")? == "completed" {
            return Ok(());
        }
        let key: String = row.try_get("plugin_key")?;
        let snapshot = registry::current(&self.pool).await?;
        let plugin = snapshot.plugin(&key).ok_or_else(|| {
            AppError::unavailable_code("plugin_disabled", "the plugin is disabled or uninstalled")
        })?;
        let Json(input): Json<serde_json::Value> = row.try_get("input")?;
        let Json(execution): Json<ExecutionContext> = row.try_get("execution")?;
        let text = |key: &str| {
            input
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let actor = Actor::from_parts(
            &text("actor_type"),
            input
                .get("actor_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        )?;
        let call = wit::Event {
            id: text("id"),
            event_type: text("event_type"),
            actor: event_actor(&actor),
            record_type: input
                .get("record_type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            record_id: input
                .get("record_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            occurred_at: text("occurred_at"),
            payload_json: input["payload"].to_string(),
        };
        let outcome = if let Some(authority) = input.get("authority") {
            let authority: crate::effects::Authority = serde_json::from_value(authority.clone())
                .map_err(|_| AppError::Internal("invalid continuation authority".to_owned()))?;
            let scope = authority
                .resolve(&self.pool)
                .await?
                .restricted(|permission| plugin.manifest.allows(permission))
                .acting_as("plugin", &plugin.key);
            if !plugin.manifest.events.contains(&call.event_type) {
                return Err(AppError::Validation(
                    "HTTP continuation event is no longer declared".to_owned(),
                ));
            }
            crate::runtime::on_event_with_scope(plugin, &call, execution, scope).await
        } else {
            on_event(plugin, &call, execution).await
        };
        match outcome {
            Ok(()) => {
                sqlx::query("UPDATE plugins_executions SET status = 'completed', completed_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?").bind(id).execute(&mut *tx).await?;
            }
            Err(error) => {
                sqlx::query("UPDATE plugins_executions SET last_error = ? WHERE id = ?")
                    .bind(error.client_message())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Err(error);
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
