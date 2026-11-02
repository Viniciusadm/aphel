use async_trait::async_trait;
use serde_json::json;

use ::notifications::application::{
    NotificationIntent, NotificationSource, NotificationSourceContext, requested_job_intents,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::errors::AppError;

use crate::model::{ORG_EXPORT_COMPLETED, ORG_EXPORT_FAILED};

const ORG_EXPORTS_LINK: &str = "/data/org-export";

pub(crate) struct OrganizationExportNotifications;

#[async_trait]
impl NotificationSource for OrganizationExportNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(event_type, ORG_EXPORT_COMPLETED | ORG_EXPORT_FAILED)
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let kind = if event.event_type == ORG_EXPORT_COMPLETED {
            exports::notifications::EXPORT_READY_NOTIFICATION
        } else {
            exports::notifications::EXPORT_FAILED_NOTIFICATION
        };
        Ok(requested_job_intents(
            event,
            kind,
            Some(ORG_EXPORTS_LINK.to_owned()),
            json!({ "organization_export_id": event.payload.get("export_id") }),
        ))
    }
}
