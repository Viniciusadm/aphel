use async_trait::async_trait;
use serde_json::json;

use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, payload_text,
    requested_job_intents,
};
use aphel_sdk::platform::application::shared::DomainEvent;

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
            aphel_sdk::exports::EXPORT_READY_NOTIFICATION
        } else {
            aphel_sdk::exports::EXPORT_FAILED_NOTIFICATION
        };
        Ok(requested_job_intents(
            event,
            kind,
            Some(PRIVACY_REQUESTS_LINK.to_owned()),
            json!({ "privacy_request_id": event.payload.get("request_id") }),
        ))
    }
}

pub(crate) struct ExportNotificationTexts;
impl aphel_sdk::notifications::NotificationTexts for ExportNotificationTexts {
    fn headline(
        &self,
        view: &aphel_sdk::notifications::NotificationView,
        _actor: &str,
        locale: aphel_sdk::platform::application::mail::MailLocale,
    ) -> Option<String> {
        view.data.get("privacy_request_id")?;
        let text = match view.kind {
            aphel_sdk::exports::EXPORT_READY_NOTIFICATION => {
                locale.pick("Sua exportação está pronta", "Your export is ready")
            }
            aphel_sdk::exports::EXPORT_FAILED_NOTIFICATION => {
                locale.pick("Sua exportação falhou", "Your export failed")
            }
            _ => return None,
        };
        Some(text.to_owned())
    }
    fn action_label(
        &self,
        view: &aphel_sdk::notifications::NotificationView,
        locale: aphel_sdk::platform::application::mail::MailLocale,
    ) -> Option<&'static str> {
        (view.data.get("privacy_request_id").is_some()
            && matches!(
                view.kind,
                aphel_sdk::exports::EXPORT_READY_NOTIFICATION
                    | aphel_sdk::exports::EXPORT_FAILED_NOTIFICATION
            ))
        .then(|| locale.pick("Ver solicitações", "View requests"))
    }
}
