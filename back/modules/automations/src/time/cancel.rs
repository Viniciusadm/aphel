use std::sync::Arc;

use async_trait::async_trait;

use crate::clock::automation_clock;
use crate::runtime::AutomationRuntime;
use crate::store::{AutomationTimeStore, CancelTarget};
use aphel_kernel::application::automations::triggers::is_automatable;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::automations::SkipReason;
use aphel_kernel::errors::AppError;

pub(crate) const CANCEL_WAITING_CONSUMER: &str = "automations.cancel_waiting";

pub(crate) struct WaitingRunsCanceller(pub(crate) Arc<AutomationRuntime>);

fn deleted_record_type(event_type: &str) -> Option<&str> {
    event_type
        .strip_suffix(".deleted")
        .filter(|record_type| is_automatable(record_type))
}

#[async_trait]
impl EventConsumer for WaitingRunsCanceller {
    fn name(&self) -> &str {
        CANCEL_WAITING_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        deleted_record_type(event_type).is_some()
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = event.record.as_ref().filter(|record| {
            deleted_record_type(&event.event_type)
                .and_then(|key| aphel_kernel::application::shared::RecordType::parse(key).ok())
                .is_some_and(|key| key == record.record_type)
        }) else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        let mut tx = ctx.begin().await?;
        let cancelled = AutomationTimeStore::of(&ctx)
            .cancel_waiting(
                &mut tx,
                &CancelTarget::Record(record.clone()),
                SkipReason::RecordDeleted,
                automation_clock().now(),
            )
            .await?;
        tx.commit().await?;
        if cancelled > 0 {
            tracing::info!(cancelled, record = %record.record_id, "waiting automation runs cancelled");
        }
        Ok(())
    }
}
