use aphel_kernel::application::mail::MailLocale;
use aphel_sdk::notifications::NotificationTexts;
use aphel_sdk::notifications::NotificationView;
use async_trait::async_trait;
use serde_json::json;

use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, payload_text,
    requested_job_intents,
};

use crate::model::{EXPORT_COMPLETED, EXPORT_FAILED};

pub(crate) struct ExportNotifications;

#[async_trait]
impl NotificationSource for ExportNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(event_type, EXPORT_COMPLETED | EXPORT_FAILED)
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let payload = &event.payload;
        if event.event_type == EXPORT_COMPLETED {
            let file_id = payload_text(payload, "file_id");
            return Ok(requested_job_intents(
                event,
                EXPORT_READY_NOTIFICATION,
                file_id.as_ref().map(|id| format!("/api/v1/files/{id}")),
                json!({
                    "export_id": payload.get("export_id"),
                    "record_type": payload.get("record_type"),
                    "format": payload.get("format"),
                    "file_id": file_id,
                    "row_count": payload.get("row_count"),
                }),
            ));
        }
        Ok(requested_job_intents(
            event,
            EXPORT_FAILED_NOTIFICATION,
            None,
            json!({
                "export_id": payload.get("export_id"),
                "record_type": payload.get("record_type"),
                "format": payload.get("format"),
            }),
        ))
    }
}

pub const EXPORT_READY_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("export_ready");
pub const EXPORT_FAILED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("export_failed");

pub(crate) struct ExportNotificationTexts;

impl NotificationTexts for ExportNotificationTexts {
    fn headline(
        &self,
        view: &NotificationView,
        _actor: &str,
        locale: MailLocale,
    ) -> Option<String> {
        let text = match view.kind {
            EXPORT_READY_NOTIFICATION => {
                locale.pick("Sua exportação está pronta", "Your export is ready")
            }
            EXPORT_FAILED_NOTIFICATION => {
                locale.pick("Sua exportação falhou", "Your export failed")
            }
            _ => return None,
        };
        Some(text.to_owned())
    }

    fn action_label(&self, view: &NotificationView, locale: MailLocale) -> Option<&'static str> {
        (view.kind == EXPORT_READY_NOTIFICATION)
            .then(|| locale.pick("Baixar arquivo", "Download file"))
    }
}
