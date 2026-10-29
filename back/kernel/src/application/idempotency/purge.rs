use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;

use crate::application::idempotency::{IdempotencyStore, IdempotencyUseCases};
use crate::application::jobs::ScheduledJobDefinition;
use crate::application::shared::{JobContext, JobHandler};
use crate::errors::AppError;

pub const IDEMPOTENCY_PURGE_JOB: &str = "idempotency.purge";
pub const IDEMPOTENCY_PURGE_CRON: &str = "0 40 * * * *";

pub fn idempotency_purge_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(IDEMPOTENCY_PURGE_JOB, IDEMPOTENCY_PURGE_CRON)
        .expect("the idempotency purge cron expression is valid")
        .with_max_attempts(1)
}

pub struct IdempotencyPurgeHandler<S> {
    use_cases: IdempotencyUseCases<S>,
}

impl<S> IdempotencyPurgeHandler<S>
where
    S: IdempotencyStore,
{
    pub fn new(store: S) -> Self {
        Self {
            use_cases: IdempotencyUseCases::new(store),
        }
    }
}

#[async_trait]
impl<S> JobHandler for IdempotencyPurgeHandler<S>
where
    S: IdempotencyStore,
{
    fn kind(&self) -> &str {
        IDEMPOTENCY_PURGE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let purged = self.use_cases.purge_expired(Utc::now()).await?;
        tracing::info!(purged, "idempotency keys purge finished");
        Ok(())
    }
}
