use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::application::dispatcher::NotificationIntent;
use crate::application::ports::NotificationDirectory;
use crate::application::texts::{LocalizedContent, record_type_label};
use crate::domain::NotificationKind;
use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;

pub const DEFAULT_SOURCE_ORDER: i32 = 0;

pub struct NotificationSourceContext<'a> {
    pub directory: &'a dyn NotificationDirectory,
    pub actor: Option<&'a UserId>,
    pub notified: &'a [UserId],
}

#[async_trait]
pub trait NotificationSource: Send + Sync {
    fn accepts(&self, event_type: &str) -> bool;

    fn order(&self) -> i32 {
        DEFAULT_SOURCE_ORDER
    }

    async fn intents(
        &self,
        context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError>;
}

pub fn event_dedupe(event: &DomainEvent, kind: NotificationKind) -> String {
    format!("event:{}:{}", event.id, kind.as_str())
}

pub fn payload_text(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_owned)
}

pub fn payload_user(value: Option<&Value>) -> Option<UserId> {
    value
        .and_then(Value::as_str)
        .and_then(|id| UserId::new(id).ok())
}

pub fn requested_job_intents(
    event: &DomainEvent,
    kind: NotificationKind,
    link: Option<String>,
    data: Value,
) -> Vec<NotificationIntent> {
    let Some(requested_by) = payload_user(event.payload.get("requested_by")) else {
        return Vec::new();
    };
    let record_type = payload_text(&event.payload, "record_type").unwrap_or_default();
    vec![
        NotificationIntent::new(
            requested_by,
            kind,
            record_type_label(&record_type, MailLocale::default()),
            event_dedupe(event, kind),
        )
        .with_localized(LocalizedContent::RecordType(record_type))
        .with_link(link)
        .with_data(data),
    ]
}

pub fn notification_sources() -> Vec<Arc<dyn NotificationSource>> {
    let mut sources = aphel_kernel::modules::try_app()
        .map(|app| app.contributions::<dyn NotificationSource>())
        .unwrap_or_default();
    sources.sort_by_key(|source| source.order());
    sources
}
