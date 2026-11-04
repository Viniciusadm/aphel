use async_trait::async_trait;

use crate::store::settings::EngagementReader;
use crate::{EMAIL_RECEIVED, EMAIL_SENT, EMAIL_THREAD_LINKED, EMAIL_THREAD_UNLINKED};
use aphel_kernel::application::shared::{DomainEvent, EventConsumer};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

pub(crate) const EMAIL_ENGAGEMENT_CONSUMER: &str = "email.engagement";

pub(crate) struct EmailEngagementConsumer {
    state: AppState,
}

impl EmailEngagementConsumer {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl EventConsumer for EmailEngagementConsumer {
    fn name(&self) -> &str {
        EMAIL_ENGAGEMENT_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            EMAIL_RECEIVED | EMAIL_SENT | EMAIL_THREAD_LINKED | EMAIL_THREAD_UNLINKED
        )
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = &event.record else {
            return Ok(());
        };
        let sources = aphel_kernel::modules::app()
            .all_contributions::<dyn aphel_sdk::email::EmailEngagementProjection>();
        let mut matching = sources
            .iter()
            .filter(|(_, source)| source.record_type() == record.record_type.as_str());
        let Some((_, source)) = matching.next() else {
            return Ok(());
        };
        if matching.next().is_some() {
            return Err(AppError::Configuration(format!(
                "multiple email engagement projections for {}",
                record.record_type.as_str()
            )));
        }
        source
            .refresh(&self.state.db()?, record, &EngagementReader)
            .await
    }
}
