use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::application::jobs::{EventConsumerRegistry, ScheduledJobDefinition};
use crate::application::shared::{
    Actor, DomainEvent, EnqueueOutcome, Job, JobContext, JobHandler, RecordRef,
};
use crate::errors::AppError;
use crate::infrastructure::events::{causation_column, origin_columns};
use crate::infrastructure::jobs::queue::insert_job;
use crate::infrastructure::repositories::mapper;

pub(crate) const OUTBOX_DISPATCH_KIND: &str = "outbox.dispatch";
pub(crate) const EVENT_CONSUME_KIND: &str = "event.consume";
pub(crate) const OUTBOX_DISPATCH_CRON: &str = "*/5 * * * * *";
pub(crate) const DEFAULT_OUTBOX_BATCH_SIZE: u32 = 100;
pub(crate) const OUTBOX_DISPATCH_TIME_BUDGET: Duration = Duration::from_secs(20);
const FOLLOW_UP_UNIQUE_KEY: &str = "outbox.dispatch:follow-up";

const SELECT_UNDISPATCHED_EVENTS: &str = "SELECT id, event_type, record_type, record_id, actor_type, actor_id, payload, causation, origin_type, origin_id, occurred_at \
     FROM domain_events WHERE dispatched_at IS NULL \
     ORDER BY occurred_at, id LIMIT ? FOR UPDATE SKIP LOCKED";
const SELECT_EVENT_BY_ID: &str = "SELECT id, event_type, record_type, record_id, actor_type, actor_id, payload, causation, origin_type, origin_id, occurred_at \
     FROM domain_events WHERE id = ?";

pub(crate) fn outbox_dispatch_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(OUTBOX_DISPATCH_KIND, OUTBOX_DISPATCH_CRON)
        .expect("the outbox dispatch cron expression is valid")
        .with_max_attempts(1)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EventConsumePayload {
    pub(crate) event_id: String,
    pub(crate) consumer: String,
}

impl EventConsumePayload {
    pub(crate) fn unique_key(&self) -> String {
        format!("event:{}:{}", self.event_id, self.consumer)
    }
}

pub(crate) fn domain_event_from_row(row: &MySqlRow) -> Result<DomainEvent, AppError> {
    let id: String = row.try_get("id")?;
    let record_type: Option<String> = row.try_get("record_type")?;
    let record_id: Option<String> = row.try_get("record_id")?;
    let actor_type: String = row.try_get("actor_type")?;
    let actor_id: Option<String> = row.try_get("actor_id")?;
    let Json(payload): Json<Value> = row.try_get("payload")?;
    let record = match (record_type, record_id) {
        (Some(record_type), Some(record_id)) => Some(RecordRef::parse(&record_type, &record_id)?),
        _ => None,
    };
    Ok(DomainEvent {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("domain_events.id", &id))?,
        actor: Actor::from_parts(&actor_type, actor_id)?,
        event_type: row.try_get("event_type")?,
        record,
        payload,
        causation: causation_column(row)?,
        origin: origin_columns(row),
        occurred_at: mapper::utc(row.try_get::<NaiveDateTime, _>("occurred_at")?),
    })
}

pub(crate) struct OutboxDispatchHandler {
    pool: MySqlPool,
    consumers: Arc<EventConsumerRegistry>,
    batch_size: u32,
}

impl OutboxDispatchHandler {
    pub(crate) fn new(pool: MySqlPool, consumers: Arc<EventConsumerRegistry>) -> Self {
        Self {
            pool,
            consumers,
            batch_size: DEFAULT_OUTBOX_BATCH_SIZE,
        }
    }

    pub(crate) async fn dispatch_batch(&self) -> Result<usize, AppError> {
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query(SELECT_UNDISPATCHED_EVENTS)
            .bind(self.batch_size)
            .fetch_all(&mut *transaction)
            .await?;

        for row in &rows {
            let event = domain_event_from_row(row)?;
            for consumer in self.consumers.accepting(&event.event_type) {
                let payload = EventConsumePayload {
                    event_id: event.id.to_string(),
                    consumer: consumer.name().to_owned(),
                };
                let job = Job::new(
                    EVENT_CONSUME_KIND,
                    serde_json::to_value(&payload).map_err(|error| {
                        AppError::Internal(format!("failed to serialize event delivery: {error}"))
                    })?,
                )
                .with_unique_key(payload.unique_key());
                insert_job(&mut *transaction, job).await?;
            }
            sqlx::query("UPDATE domain_events SET dispatched_at = UTC_TIMESTAMP(6) WHERE id = ?")
                .bind(event.id.to_string())
                .execute(&mut *transaction)
                .await?;
        }

        let dispatched = rows.len();
        if dispatched == self.batch_size as usize {
            let follow_up = Job::new(OUTBOX_DISPATCH_KIND, Value::Object(Default::default()))
                .with_max_attempts(1)
                .with_unique_key(FOLLOW_UP_UNIQUE_KEY);
            if let EnqueueOutcome::Enqueued(_) = insert_job(&mut *transaction, follow_up).await? {
                debug!("outbox has more events; enqueued a follow-up dispatch");
            }
        }
        transaction.commit().await?;
        Ok(dispatched)
    }

    pub(crate) async fn drain(&self, budget: Duration) -> Result<usize, AppError> {
        let started = Instant::now();
        let mut total = 0;
        loop {
            let dispatched = self.dispatch_batch().await?;
            total += dispatched;
            if dispatched < self.batch_size as usize || started.elapsed() >= budget {
                return Ok(total);
            }
        }
    }
}

#[async_trait]
impl JobHandler for OutboxDispatchHandler {
    fn kind(&self) -> &str {
        OUTBOX_DISPATCH_KIND
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let dispatched = self.drain(OUTBOX_DISPATCH_TIME_BUDGET).await?;
        if dispatched > 0 {
            debug!(dispatched, "dispatched domain events");
        }
        Ok(())
    }
}

pub(crate) struct EventConsumeHandler {
    pool: MySqlPool,
    consumers: Arc<EventConsumerRegistry>,
}

impl EventConsumeHandler {
    pub(crate) fn new(pool: MySqlPool, consumers: Arc<EventConsumerRegistry>) -> Self {
        Self { pool, consumers }
    }
}

#[async_trait]
impl JobHandler for EventConsumeHandler {
    fn kind(&self) -> &str {
        EVENT_CONSUME_KIND
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let delivery: EventConsumePayload = serde_json::from_value(payload)
            .map_err(|error| AppError::Validation(format!("invalid event delivery: {error}")))?;
        let consumer = self.consumers.get(&delivery.consumer).ok_or_else(|| {
            AppError::Internal(format!(
                "no event consumer is registered with name {}",
                delivery.consumer
            ))
        })?;

        let row = sqlx::query(SELECT_EVENT_BY_ID)
            .bind(&delivery.event_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            warn!(event_id = %delivery.event_id, consumer = %delivery.consumer, "domain event no longer exists; skipping delivery");
            return Ok(());
        };

        let event = domain_event_from_row(&row)?;
        consumer.consume(&event).await
    }
}
