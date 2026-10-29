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
    async fn publish(&self, tx: &mut MySqlTx, event: DomainEvent) -> Result<(), AppError> {
        let (record_type, record_id) = match &event.record {
            Some(record) => (
                Some(record.record_type.as_str().to_owned()),
                Some(record.record_id.to_string()),
            ),
            None => (None, None),
        };
        sqlx::query(
            "INSERT INTO domain_events (id, event_type, record_type, record_id, actor_type, actor_id, payload, causation, origin_type, origin_id, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.id.to_string())
        .bind(&event.event_type)
        .bind(record_type)
        .bind(record_id)
        .bind(event.actor.kind())
        .bind(event.actor.id())
        .bind(Json(&event.payload))
        .bind(event.causation.as_ref().map(Json))
        .bind(event.origin.as_ref().map(|origin| origin.kind.as_str()))
        .bind(event.origin.as_ref().and_then(|origin| origin.id.as_deref()))
        .bind(mapper::naive(event.occurred_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
