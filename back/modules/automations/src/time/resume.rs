use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::clock::automation_clock;
use crate::runtime::{AutomationRuntime, resume_payload, run_job};
use crate::store::AutomationTimeStore;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{JobContext, JobHandler, JobQueue};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const RESUME_WAITING_JOB: &str = "automations.resume_waiting";
pub(crate) const RESUME_WAITING_CRON: &str = "30 * * * * *";
pub(crate) const RESUME_BATCH: u32 = 200;
pub(crate) const RESUME_ROUNDS: usize = 5;

pub(crate) fn resume_waiting_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(RESUME_WAITING_JOB, RESUME_WAITING_CRON)
        .expect("the resume cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) struct WaitingRunsResumer(pub(crate) Arc<AutomationRuntime>);

impl WaitingRunsResumer {
    pub(crate) async fn resume_due(&self) -> Result<usize, AppError> {
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        let time = AutomationTimeStore::of(&ctx);
        let mut resumed = 0;
        for _ in 0..RESUME_ROUNDS {
            let now = automation_clock().now();
            let mut tx = ctx.begin().await?;
            let runs = time.claim_due(&mut tx, now, RESUME_BATCH).await?;
            for run in &runs {
                let job = run_job(&resume_payload(run))?;
                ctx.jobs().enqueue(&mut tx, job).await?;
            }
            tx.commit().await?;
            resumed += runs.len();
            if runs.len() < RESUME_BATCH as usize {
                break;
            }
        }
        Ok(resumed)
    }
}

#[async_trait]
impl JobHandler for WaitingRunsResumer {
    fn kind(&self) -> &str {
        RESUME_WAITING_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let resumed = self.resume_due().await?;
        if resumed > 0 {
            tracing::info!(resumed, "waiting automation runs resumed");
        }
        Ok(())
    }
}
