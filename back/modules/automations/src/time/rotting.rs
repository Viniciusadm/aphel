use crate::runtime::AutomationRuntime;
use crate::time::sweep::TimeTriggerSweep;
use aphel_contract_automation::application::time::triggers::TimeTriggerPlanner;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer};
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use std::sync::Arc;
pub(crate) struct TimeCatchUpConsumer(pub(crate) Arc<AutomationRuntime>);
#[async_trait]
impl EventConsumer for TimeCatchUpConsumer {
    fn name(&self) -> &str {
        "automations.time_catch_up"
    }
    fn accepts(&self, event_type: &str) -> bool {
        aphel_kernel::modules::app()
            .contributions::<TimeTriggerPlanner>()
            .iter()
            .any(|planner| {
                planner
                    .catch_up
                    .as_ref()
                    .is_some_and(|policy| policy.event_type == event_type)
            })
    }
    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        for planner in aphel_kernel::modules::app().contributions::<TimeTriggerPlanner>() {
            let Some(policy) = planner
                .catch_up
                .as_ref()
                .filter(|policy| policy.event_type == event.event_type)
            else {
                continue;
            };
            let Some(key) = (policy.event_key)(event) else {
                continue;
            };
            for record_type in policy.record_types {
                TimeTriggerSweep(self.0.clone())
                    .catch_up_trigger(record_type, planner.kind, key, policy)
                    .await?;
            }
        }
        Ok(())
    }
}
