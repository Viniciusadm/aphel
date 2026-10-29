use std::sync::Arc;

use async_trait::async_trait;

use crate::runtime::AutomationRuntime;
use crate::time::sweep::TimeTriggerSweep;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer};
use aphel_kernel::errors::AppError;
use pipelines::rotting::{PIPELINE_STAGE_ROTTING_TIGHTENED, tightened_stage};

pub(crate) const ROTTING_CATCH_UP_CONSUMER: &str = "automations.rotting_catch_up";

pub(crate) struct RottingCatchUp(pub(crate) Arc<AutomationRuntime>);

#[async_trait]
impl EventConsumer for RottingCatchUp {
    fn name(&self) -> &str {
        ROTTING_CATCH_UP_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == PIPELINE_STAGE_ROTTING_TIGHTENED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(stage_id) = tightened_stage(event) else {
            return Ok(());
        };
        let fired = TimeTriggerSweep(self.0.clone())
            .catch_up_rotting_stage(stage_id)
            .await?;
        if fired > 0 {
            tracing::info!(fired, stage = %stage_id, "rotting catch-up queued automation runs");
        }
        Ok(())
    }
}
