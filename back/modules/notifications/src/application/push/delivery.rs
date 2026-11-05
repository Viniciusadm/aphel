use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::push::{
    PushMessage, PushOutcome, PushPorts, PushSender, PushSubscriptionRepository,
};
use crate::application::{NotificationPorts, NotificationRepository, NotificationView};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::security::Clock;
use aphel_sdk::platform::application::shared::{Job, JobContext, JobHandler};

pub const NOTIFICATION_PUSH_JOB: &str = "notification.push";
pub const PUSH_TITLE_MAX_CHARS: usize = 120;
pub const PUSH_BODY_MAX_CHARS: usize = 180;
pub const PUSH_DEFAULT_URL: &str = "/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushDelivery {
    Sent { delivered: usize, removed: usize },
    AlreadySent,
    Skipped,
}

pub fn push_job(notification_id: Uuid) -> Job {
    Job::new(
        NOTIFICATION_PUSH_JOB,
        json!({
            "notification_id": notification_id.to_string(),
        }),
    )
    .with_unique_key(format!("{NOTIFICATION_PUSH_JOB}:{notification_id}"))
}

fn truncate(value: &str, max: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut truncated: String = value.chars().take(max.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

pub fn push_message(view: &NotificationView) -> PushMessage {
    let url = view
        .link
        .as_deref()
        .filter(|link| link.starts_with('/') && !link.starts_with("//"))
        .unwrap_or(PUSH_DEFAULT_URL)
        .to_owned();
    PushMessage {
        title: truncate(&view.title, PUSH_TITLE_MAX_CHARS),
        body: view
            .body
            .as_deref()
            .map(|body| truncate(body, PUSH_BODY_MAX_CHARS))
            .filter(|body| !body.is_empty()),
        url,
        tag: view.id.to_string(),
    }
}

pub struct NotificationPushHandler<N, P, S> {
    notifications: N,
    push: P,
    sender: S,
}

impl<N, P, S> NotificationPushHandler<N, P, S>
where
    N: NotificationPorts,
    P: PushPorts,
    S: PushSender,
{
    pub fn new(notifications: N, push: P, sender: S) -> Self {
        Self {
            notifications,
            push,
            sender,
        }
    }

    pub async fn deliver(&self, notification_id: Uuid) -> Result<PushDelivery, AppError> {
        let Some(notification) = self
            .notifications
            .notifications()
            .find_deliverable(notification_id)
            .await?
        else {
            return Ok(PushDelivery::Skipped);
        };
        let now = self.push.clock().now();
        let subscriptions = self.push.subscriptions();
        let targets = subscriptions
            .active_targets(&notification.user_id, now)
            .await?;
        if targets.is_empty() {
            return Ok(PushDelivery::Skipped);
        }
        if !subscriptions
            .claim_notification(notification_id, now)
            .await?
        {
            return Ok(PushDelivery::AlreadySent);
        }
        let message = push_message(&notification.view);
        let (mut delivered, mut removed, mut failed) = (0, 0, 0);
        for target in &targets {
            match self.sender.send(target, &message).await {
                Ok(PushOutcome::Delivered) => {
                    delivered += 1;
                    subscriptions.mark_used(target.id, now).await?;
                }
                Ok(PushOutcome::Gone) => {
                    removed += 1;
                    subscriptions.delete(target.id).await?;
                }
                Ok(PushOutcome::Failed(reason)) => {
                    failed += 1;
                    tracing::warn!(subscription_id = %target.id, %reason, "web push delivery failed");
                }
                Err(error) => {
                    failed += 1;
                    tracing::warn!(subscription_id = %target.id, %error, "web push delivery failed");
                }
            }
        }
        if delivered == 0 && failed > 0 {
            subscriptions.release_notification(notification_id).await?;
            return Err(AppError::Internal(
                "web push delivery failed for every subscription".to_owned(),
            ));
        }
        Ok(PushDelivery::Sent { delivered, removed })
    }
}

fn invalid_payload() -> AppError {
    AppError::Internal("invalid notification.push payload".to_owned())
}

#[async_trait]
impl<N, P, S> JobHandler for NotificationPushHandler<N, P, S>
where
    N: NotificationPorts,
    P: PushPorts,
    S: PushSender,
{
    fn kind(&self) -> &str {
        NOTIFICATION_PUSH_JOB
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
