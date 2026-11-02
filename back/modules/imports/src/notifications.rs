use ::notifications::application::NotificationView;
use ::notifications::application::texts::NotificationTexts;
use aphel_kernel::application::mail::MailLocale;
use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use ::notifications::application::{
    NotificationIntent, NotificationSource, NotificationSourceContext, payload_text,
    requested_job_intents,
};
use ::notifications::domain::NotificationKind;
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::errors::AppError;

use crate::model::{IMPORT_COMPLETED, IMPORT_FAILED};

pub(crate) struct ImportNotifications;

#[async_trait]
impl NotificationSource for ImportNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(event_type, IMPORT_COMPLETED | IMPORT_FAILED)
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let payload = &event.payload;
        let import_id =
            payload_text(payload, "import_job_id").filter(|id| Uuid::parse_str(id).is_ok());
        let kind = if event.event_type == IMPORT_COMPLETED {
            IMPORT_FINISHED_NOTIFICATION
        } else {
            IMPORT_FAILED_NOTIFICATION
        };
        Ok(requested_job_intents(
            event,
            kind,
            import_id.as_ref().map(|id| format!("/imports/{id}")),
            json!({
                "import_job_id": import_id,
                "record_type": payload.get("record_type"),
                "totals": payload.get("totals"),
            }),
        ))
    }
}

pub(crate) const IMPORT_FINISHED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("import_finished");
pub(crate) const IMPORT_FAILED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("import_failed");

pub(crate) struct ImportNotificationTexts;

impl NotificationTexts for ImportNotificationTexts {
    fn headline(
        &self,
        view: &NotificationView,
        _actor: &str,
        locale: MailLocale,
    ) -> Option<String> {
        let text = match view.kind {
            IMPORT_FINISHED_NOTIFICATION => {
                locale.pick("Sua importação foi concluída", "Your import finished")
            }
            IMPORT_FAILED_NOTIFICATION => {
                locale.pick("Sua importação falhou", "Your import failed")
            }
            _ => return None,
        };
        Some(text.to_owned())
    }
}
