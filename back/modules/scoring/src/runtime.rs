use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::{
    SCORING_DECAY_CRON, SCORING_DECAY_JOB, SCORING_EVENTS_CONSUMER, SCORING_RECALCULATE_JOB,
    SIGNAL_EVENT_TYPES,
};
use crate::engine::{ScoringEngine, is_lifecycle_event};
use crate::store::ModelStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::custom_objects::RecordTypeCatalog;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{
    DomainEvent, EventConsumer, JobContext, JobHandler,
};
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::audit::MySqlAuditLog;
use aphel_sdk::platform::infrastructure::events::MySqlEventPublisher;

pub(crate) fn scoring_decay_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(SCORING_DECAY_JOB, SCORING_DECAY_CRON)
        .expect("the scoring decay cron expression is valid")
        .with_max_attempts(1)
}

#[derive(Clone)]
pub(crate) struct ScoringRuntime {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl ScoringRuntime {
    pub(crate) fn new(pool: MySqlPool, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }

    async fn ctx<'s>(&self, scope: &'s AccessScope) -> Result<Ctx<'s>, AppError> {
        Ok(Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        ))
    }

    async fn recalculate_models(&self, only: Option<Uuid>) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.ctx(&scope).await?;
        let engine = ScoringEngine::new(&ctx);
        for model in ModelStore::of(&ctx).list(None).await? {
            if !model.is_active || only.is_some_and(|id| id != model.id) {
                continue;
            }
            let records = engine.recalculate_model(&scope, &model).await?;
            tracing::debug!(
                model = model.key.as_str(),
                records,
                "scoring model recalculated"
            );
        }
        Ok(())
    }
}

pub(crate) struct ScoringEventsConsumer(pub(crate) ScoringRuntime);

#[async_trait]
impl EventConsumer for ScoringEventsConsumer {
    fn name(&self) -> &str {
        SCORING_EVENTS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        SIGNAL_EVENT_TYPES.contains(&event_type) || is_lifecycle_event(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        ScoringEngine::new(&ctx).handle_event(event).await
    }
}

pub(crate) struct ScoringRecalculateHandler(pub(crate) ScoringRuntime);

#[async_trait]
impl JobHandler for ScoringRecalculateHandler {
    fn kind(&self) -> &str {
        SCORING_RECALCULATE_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        if let (Some(record_type), Some(record_id)) = (
            payload.get("record_type").and_then(Value::as_str),
            payload.get("record_id").and_then(Value::as_str),
        ) {
            let record =
                aphel_sdk::platform::application::shared::RecordRef::parse(record_type, record_id)?;
            let scope = AccessScope::system();
            let ctx = self.0.ctx(&scope).await?;
            return ScoringEngine::new(&ctx).recalculate_signal(&record).await;
        }
        let model_id = payload
            .get("model_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        let Some(model_id) = model_id else {
            return Ok(());
        };
        self.0.recalculate_models(Some(model_id)).await
    }
}

pub(crate) struct ScoringDecayHandler(pub(crate) ScoringRuntime);

#[async_trait]
impl JobHandler for ScoringDecayHandler {
    fn kind(&self) -> &str {
        SCORING_DECAY_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        if let Err(error) = self.0.recalculate_models(None).await {
            tracing::warn!(%error, "scoring decay failed");
        }
        Ok(())
    }
}
