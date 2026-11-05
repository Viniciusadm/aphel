use async_trait::async_trait;
use serde_json::json;

use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, requested_job_intents,
};
use aphel_sdk::platform::application::shared::DomainEvent;

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
            aphel_sdk::exports::EXPORT_READY_NOTIFICATION
        } else {
            aphel_sdk::exports::EXPORT_FAILED_NOTIFICATION
        };
        Ok(requested_job_intents(
            event,
            kind,
            Some(ORG_EXPORTS_LINK.to_owned()),
            json!({ "organization_export_id": event.payload.get("export_id") }),
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
        view.data.get("organization_export_id")?;
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
        (view.data.get("organization_export_id").is_some()
            && matches!(
                view.kind,
                aphel_sdk::exports::EXPORT_READY_NOTIFICATION
                    | aphel_sdk::exports::EXPORT_FAILED_NOTIFICATION
            ))
        .then(|| locale.pick("Ver exportações", "View exports"))
    }
}
