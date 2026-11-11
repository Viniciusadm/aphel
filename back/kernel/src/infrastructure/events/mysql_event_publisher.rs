use async_trait::async_trait;
use sqlx::types::Json;

use crate::application::shared::{DomainEvent, EventPublisher};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Debug, Clone, Copy, Default)]
pub struct MySqlEventPublisher;

impl MySqlEventPublisher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventPublisher<MySqlTx> for MySqlEventPublisher {
    async fn publish(&self, tx: &mut MySqlTx, mut event: DomainEvent) -> Result<(), AppError> {
        if !tx.dispatch_requested {
            tx.dispatch_requested = true;
            tx.after_commit.push(Box::pin(async {
                crate::infrastructure::query::mysql_executor::invalidate_counts();
                if let Ok(pool) = crate::state::runtime_pool() {
                    crate::infrastructure::jobs::outbox::request_dispatch(&pool).await;
                }
            }));
        }
        if let Some(execution) = crate::application::shared::ExecutionContext::current() {
            if execution.executor_kind == "plugin" {
                event.actor = crate::application::shared::Actor::plugin(execution.executor.clone());
            }
            event.execution = Some(execution);
        }
        let (record_type, record_id) = match &event.record {
            Some(record) => (
                Some(record.record_type.as_str().to_owned()),
                Some(record.record_id.to_string()),
            ),
            None => (None, None),
        };
        sqlx::query(
            "INSERT INTO domain_events (id, event_type, record_type, record_id, actor_type, actor_id, payload, execution, causation, origin_type, origin_id, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.id.to_string())
        .bind(&event.event_type)
        .bind(record_type)
        .bind(record_id)
        .bind(event.actor.kind())
        .bind(event.actor.id())
        .bind(Json(&event.payload))
        .bind(event.execution.as_ref().map(Json))
        .bind(event.causation.as_ref().map(Json))
        .bind(event.origin.as_ref().map(|origin| origin.kind.as_str()))
        .bind(event.origin.as_ref().and_then(|origin| origin.id.as_deref()))
        .bind(mapper::naive(event.occurred_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
