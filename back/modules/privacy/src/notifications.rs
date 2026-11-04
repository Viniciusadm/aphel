use async_trait::async_trait;
use serde_json::json;

use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, payload_text,
    requested_job_intents,
};

use crate::domain::{PRIVACY_REQUEST_COMPLETED, PRIVACY_REQUEST_FAILED};

const EXPORT_REQUEST_KIND: &str = "export";
const PRIVACY_REQUESTS_LINK: &str = "/data/privacy?tab=requests";

pub(crate) struct PrivacyExportNotifications;

#[async_trait]
impl NotificationSource for PrivacyExportNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            PRIVACY_REQUEST_COMPLETED | PRIVACY_REQUEST_FAILED
        )
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        if payload_text(&event.payload, "kind").as_deref() != Some(EXPORT_REQUEST_KIND) {
            return Ok(Vec::new());
        }
        let kind = if event.event_type == PRIVACY_REQUEST_COMPLETED {
            exports::notifications::EXPORT_READY_NOTIFICATION
        } else {
            exports::notifications::EXPORT_FAILED_NOTIFICATION
        };
        Ok(requested_job_intents(
            event,
            kind,
            Some(PRIVACY_REQUESTS_LINK.to_owned()),
            json!({ "privacy_request_id": event.payload.get("request_id") }),
        ))
    }
}
