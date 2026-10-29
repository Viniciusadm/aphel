use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::runner::{self, sequence_system_scope};
use crate::{
    EXIT_EVENT_TYPES, SEQUENCE_EXITS_CONSUMER, SEQUENCE_STEP_JOB, SEQUENCES_TICK_CRON,
    SEQUENCES_TICK_JOB, exits,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer, JobContext, JobHandler};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_kernel::state::AppState;

pub(crate) fn sequences_tick_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(SEQUENCES_TICK_JOB, SEQUENCES_TICK_CRON)
        .expect("the sequences tick cron expression is valid")
        .with_max_attempts(1)
}

#[derive(Clone)]
pub(crate) struct SequenceRuntime {
    state: AppState,
}

impl SequenceRuntime {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }

    async fn ctx<'s>(&self, scope: &'s AccessScope) -> Result<Ctx<'s>, AppError> {
        Ctx::from_state(&self.state, scope).await
    }
}

pub(crate) struct SequenceTickHandler(pub(crate) SequenceRuntime);

#[async_trait]
impl JobHandler for SequenceTickHandler {
    fn kind(&self) -> &str {
        SEQUENCES_TICK_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = sequence_system_scope();
        let ctx = self.0.ctx(&scope).await?;
        let queued = runner::dispatch(&ctx).await?;
        if queued > 0 {
            tracing::debug!(queued, "sequence steps queued");
        }
        Ok(())
    }
}

pub(crate) struct SequenceStepHandler(pub(crate) SequenceRuntime);

#[async_trait]
impl JobHandler for SequenceStepHandler {
    fn kind(&self) -> &str {
        SEQUENCE_STEP_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(enrollment_id) = payload
            .get("enrollment_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
        else {
            return Ok(());
        };
        let scope = sequence_system_scope();
        let ctx = self.0.ctx(&scope).await?;
        runner::process(&ctx, enrollment_id, context.attempt < context.max_attempts).await
    }
}

pub(crate) struct SequenceExitConsumer(pub(crate) SequenceRuntime);

#[async_trait]
impl EventConsumer for SequenceExitConsumer {
    fn name(&self) -> &str {
        SEQUENCE_EXITS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        EXIT_EVENT_TYPES.contains(&event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let scope = sequence_system_scope();
        let ctx = self.0.ctx(&scope).await?;
        exits::handle(&ctx, event).await
    }
}

pub(crate) fn sequence_worker(env: &WorkerEnv) -> WorkerParts {
    let runtime = SequenceRuntime::new(env.state());
    WorkerParts::default()
        .handler(Arc::new(SequenceTickHandler(runtime.clone())))
        .handler(Arc::new(SequenceStepHandler(runtime.clone())))
        .consumer(Arc::new(SequenceExitConsumer(runtime)))
        .schedule(sequences_tick_schedule())
}
