use aphel_contract_crm::CrmRecordTypes;
use async_trait::async_trait;

use crate::store::settings::DirectoryStore;
use crate::{EMAIL_RECEIVED, EMAIL_SENT, EMAIL_THREAD_LINKED, EMAIL_THREAD_UNLINKED};
use aphel_kernel::application::shared::{DomainEvent, EventConsumer, RecordType};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

pub(crate) const EMAIL_ENGAGEMENT_CONSUMER: &str = "email.engagement";

pub(crate) fn tracks_engagement(record_type: &RecordType) -> bool {
    *record_type == RecordType::CONTACT || *record_type == RecordType::COMPANY
}

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
        if !tracks_engagement(&record.record_type) {
            return Ok(());
        }
        DirectoryStore::new(self.state.db()?)
            .refresh_engagement(record)
            .await
    }
}
