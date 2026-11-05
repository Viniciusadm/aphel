use async_trait::async_trait;
use serde_json::{Value, json};

use crate::application::{
    NotificationDirectory, NotificationIntent, NotificationPorts, NotificationSourceContext,
    dispatch, event_dedupe, notification_sources, payload_text as text, payload_user as user,
};
use crate::domain::NotificationKind;
use aphel_contract_automation::application::AUTOMATION_MESSAGE;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{Actor, DomainEvent, EventConsumer};

pub const NOTIFICATIONS_CONSUMER: &str = "notifications.events";

pub const AUTOMATION_RUN_SKIPPED: &str = "automation.run_skipped";

pub const NOTIFIED_EVENTS: [&str; 2] = [AUTOMATION_MESSAGE, AUTOMATION_RUN_SKIPPED];

pub struct NotificationEventConsumer<P> {
    ports: P,
}

impl<P> NotificationEventConsumer<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

fn actor_user(event: &DomainEvent) -> Option<UserId> {
    match &event.actor {
        Actor::User(id) => Some(id.clone()),
        _ => None,
    }
}

fn dedupe(event: &DomainEvent, kind: NotificationKind) -> String {
    event_dedupe(event, kind)
}

impl<P> NotificationEventConsumer<P>
where
    P: NotificationPorts,
{
    fn intents(&self, event: &DomainEvent) -> Vec<NotificationIntent> {
        match event.event_type.as_str() {
            AUTOMATION_MESSAGE => self.automation_message(event),
            AUTOMATION_RUN_SKIPPED => self.automation_skipped(event),
            _ => Vec::new(),
        }
    }

    fn recipients(event: &DomainEvent) -> Vec<UserId> {
        let mut recipients = Vec::new();
        for value in event
            .payload
            .get("recipients")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(user) = user(Some(value))
                && !recipients.contains(&user)
            {
                recipients.push(user);
            }
        }
        recipients
    }

    fn automation_message(&self, event: &DomainEvent) -> Vec<NotificationIntent> {
        let kind = match text(&event.payload, "channel").as_deref() {
            Some("email") => NotificationKind::AUTOMATION_EMAIL,
            _ => NotificationKind::AUTOMATION_MESSAGE,
        };
        Self::recipients(event)
            .into_iter()
            .map(|recipient| {
                NotificationIntent::new(
                    recipient,
                    kind,
                    text(&event.payload, "title").unwrap_or_default(),
                    dedupe(event, kind),
                )
                .with_body(text(&event.payload, "body"))
                .with_record(event.record.clone())
                .with_data(json!({
                    "automation_id": event.payload.get("automation_id"),
                    "run_id": event.payload.get("run_id"),
                }))
            })
            .collect()
    }

    fn automation_skipped(&self, event: &DomainEvent) -> Vec<NotificationIntent> {
        let kind = NotificationKind::AUTOMATION_SKIPPED;
        let automation_id = text(&event.payload, "automation_id");
        Self::recipients(event)
            .into_iter()
            .map(|recipient| {
                NotificationIntent::new(
                    recipient,
                    kind,
                    text(&event.payload, "name").unwrap_or_default(),
                    dedupe(event, kind),
                )
                .with_link(
                    automation_id
                        .as_ref()
                        .map(|id| format!("/settings/automations/{id}")),
                )
                .with_data(json!({
                    "automation_id": event.payload.get("automation_id"),
                    "run_id": event.payload.get("run_id"),
                    "reason": event.payload.get("reason"),
                    "record_type": event.payload.get("record_type"),
                    "record_id": event.payload.get("record_id"),
                }))
            })
            .collect()
    }
}

#[async_trait]
impl<P> EventConsumer for NotificationEventConsumer<P>
where
    P: NotificationPorts,
{
    fn name(&self) -> &str {
        NOTIFICATIONS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        NOTIFIED_EVENTS.contains(&event_type)
            || notification_sources()
                .iter()
                .any(|source| source.accepts(event_type))
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let actor = match &event.actor {
            Actor::ApiToken(token_id) => self.ports.directory().api_token_owner(token_id).await?,
            _ => actor_user(event),
        };
        let mut intents = self.intents(event);
        for source in notification_sources()
            .into_iter()
            .filter(|source| source.accepts(&event.event_type))
        {
            let notified: Vec<UserId> = intents
                .iter()
                .map(|intent| intent.recipient.clone())
                .collect();
            let context = NotificationSourceContext {
                directory: self.ports.directory(),
                actor: actor.as_ref(),
                notified: &notified,
            };
            intents.extend(source.intents(&context, event).await?);
        }
        for intent in intents {
            dispatch(&self.ports, intent.with_actor(actor.clone())).await?;
        }
        Ok(())
    }
}
