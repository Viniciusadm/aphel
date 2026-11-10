use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::application::texts::notification_email;
use crate::application::{
    NOTIFICATION_EMAIL_JOB, NotificationDirectory, NotificationPorts, NotificationRepository,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::mail::Mailer;
use aphel_sdk::platform::application::security::Clock;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailDelivery {
    Sent,
    AlreadySent,
    Skipped,
}

pub struct NotificationEmailHandler<P, M> {
    ports: P,
    mailer: M,
    app_base_url: aphel_sdk::platform::application::organization_profile::PublicBaseUrl,
}

impl<P, M> NotificationEmailHandler<P, M>
where
    P: NotificationPorts,
    M: Mailer,
{
    pub fn new(
        ports: P,
        mailer: M,
        app_base_url: impl Into<aphel_sdk::platform::application::organization_profile::PublicBaseUrl>,
    ) -> Self {
        Self {
            ports,
            mailer,
            app_base_url: app_base_url.into(),
        }
    }

    pub async fn deliver(&self, notification_id: Uuid) -> Result<EmailDelivery, AppError> {
        let Some(notification) = self
            .ports
            .notifications()
            .find_deliverable(notification_id)
            .await?
        else {
            return Ok(EmailDelivery::Skipped);
        };
        if notification.emailed_at.is_some() {
            return Ok(EmailDelivery::AlreadySent);
        }
        let Some(recipient) = self
            .ports
            .directory()
            .recipient(&notification.user_id)
            .await?
        else {
            return Ok(EmailDelivery::Skipped);
        };
        let message = notification_email(
            &recipient,
            &notification.view,
            &self.app_base_url.app_root(),
        );
        self.mailer.send(message).await?;
        let now = self.ports.clock().now();
        self.ports
            .notifications()
            .mark_emailed(notification_id, now)
            .await?;
        Ok(EmailDelivery::Sent)
    }
}

fn invalid_payload() -> AppError {
    AppError::Internal("invalid notification.email payload".to_owned())
}

#[async_trait]
impl<P, M> JobHandler for NotificationEmailHandler<P, M>
where
    P: NotificationPorts,
    M: Mailer,
{
    fn kind(&self) -> &str {
        NOTIFICATION_EMAIL_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let notification_id = payload
            .get("notification_id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(invalid_payload)?;
        self.deliver(notification_id).await?;
        Ok(())
    }
}
