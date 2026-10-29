use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::notifications::NotificationView;
use aphel_kernel::application::notifications::texts::NotificationTexts;
use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, event_dedupe, payload_text,
    payload_user,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::notifications::NotificationKind;
use aphel_kernel::errors::AppError;

use crate::outgoing::service::WEBHOOK_ENDPOINT_DISABLED_EVENT;

const WEBHOOKS_LINK: &str = "/integrations/webhooks";

pub(crate) struct WebhookDisabledNotifications;

#[async_trait]
impl NotificationSource for WebhookDisabledNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        event_type == WEBHOOK_ENDPOINT_DISABLED_EVENT
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let kind = WEBHOOK_ENDPOINT_DISABLED_NOTIFICATION;
        let Some(recipients) = event.payload.get("recipients").and_then(Value::as_array) else {
            return Ok(Vec::new());
        };
        Ok(recipients
            .iter()
            .filter_map(|value| payload_user(Some(value)))
            .map(|recipient| {
                NotificationIntent::new(
                    recipient,
                    kind,
                    payload_text(&event.payload, "url").unwrap_or_default(),
                    event_dedupe(event, kind),
                )
                .with_link(Some(WEBHOOKS_LINK.to_owned()))
                .with_data(json!({
                    "endpoint_id": event.payload.get("endpoint_id"),
                    "consecutive_failures": event.payload.get("consecutive_failures"),
                }))
            })
            .collect())
    }
}

pub(crate) const WEBHOOK_ENDPOINT_DISABLED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("webhook_endpoint_disabled");

pub(crate) struct WebhookNotificationTexts;

impl NotificationTexts for WebhookNotificationTexts {
    fn headline(
        &self,
        view: &NotificationView,
        _actor: &str,
        locale: MailLocale,
    ) -> Option<String> {
        (view.kind == WEBHOOK_ENDPOINT_DISABLED_NOTIFICATION).then(|| {
            locale
                .pick(
                    "Um webhook foi desativado após falhas seguidas",
                    "A webhook was disabled after repeated failures",
                )
                .to_owned()
        })
    }
}
