use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::{TRASH_PURGE_CRON, TRASH_PURGE_JOB};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{JobContext, JobHandler};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_kernel::state::AppState;

pub(crate) fn trash_purge_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(TRASH_PURGE_JOB, TRASH_PURGE_CRON)
        .expect("the trash purge cron expression is valid")
        .with_max_attempts(2)
}

pub(crate) struct TrashPurgeHandler {
    state: AppState,
}

#[async_trait]
impl JobHandler for TrashPurgeHandler {
    fn kind(&self) -> &str {
        TRASH_PURGE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        let purged = service::purge_expired(&ctx).await?;
        tracing::info!(purged, "trash purge finished");
        Ok(())
    }
}

pub(crate) fn trash_worker(env: &WorkerEnv) -> WorkerParts {
    WorkerParts::default()
        .handler(Arc::new(TrashPurgeHandler { state: env.state() }))
        .schedule(trash_purge_schedule())
}
