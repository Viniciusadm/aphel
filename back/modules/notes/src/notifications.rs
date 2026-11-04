use aphel_kernel::application::mail::MailLocale;
use aphel_sdk::notifications::NotificationTexts;
use aphel_sdk::notifications::NotificationView;
use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::shared::{DomainEvent, RecordRef};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, event_dedupe,
};

use crate::domain::{NOTE_CREATED, NOTE_UPDATED, NoteVisibility};

pub(crate) struct NoteMentionNotifications;

fn parent_record(payload: &Value) -> Option<RecordRef> {
    let record_type = payload.get("record_type").and_then(Value::as_str)?;
    let record_id = payload.get("record_id").and_then(Value::as_str)?;
    RecordRef::parse(record_type, record_id).ok()
}

#[async_trait]
impl NotificationSource for NoteMentionNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(event_type, NOTE_CREATED | NOTE_UPDATED)
    }

    async fn intents(
        &self,
        context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let Some(mentioned) = event
            .payload
            .get("mentioned_user_ids")
            .and_then(Value::as_array)
        else {
            return Ok(Vec::new());
        };
        if event.payload.get("visibility").and_then(Value::as_str)
            == Some(NoteVisibility::Private.as_str())
        {
            return Ok(Vec::new());
        }
        let Some(record) = parent_record(&event.payload) else {
            return Ok(Vec::new());
        };
        let kind = NOTE_MENTION_NOTIFICATION;
        let mut intents = Vec::new();
        for user_id in mentioned
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|id| UserId::new(id).ok())
        {
            let intent = if context.directory.can_read(&user_id, &record).await? {
                let title = context
                    .directory
                    .record_summary(&record)
                    .await?
                    .map(|summary| summary.title)
                    .unwrap_or_default();
                NotificationIntent::new(user_id, kind, title, event_dedupe(event, kind))
                    .with_record(Some(record.clone()))
                    .with_data(json!({ "restricted": false }))
            } else {
                NotificationIntent::new(user_id, kind, "", event_dedupe(event, kind))
                    .with_data(json!({ "restricted": true }))
            };
            intents.push(intent);
        }
        Ok(intents)
    }
}

pub(crate) const NOTE_MENTION_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("note_mention");

pub(crate) struct NoteNotificationTexts;

impl NotificationTexts for NoteNotificationTexts {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String> {
        (view.kind == NOTE_MENTION_NOTIFICATION).then(|| match locale {
            MailLocale::En => format!("{actor} mentioned you in a note"),
            MailLocale::PtBr => format!("{actor} mencionou você em uma nota"),
        })
    }

    fn notice(&self, view: &NotificationView, locale: MailLocale) -> Option<String> {
        (view.kind == NOTE_MENTION_NOTIFICATION && view.record.is_none()).then(|| {
            locale
                .pick(
                    "A nota está em um registro que você não pode acessar. Peça acesso a quem mencionou você.",
                    "The note is on a record you cannot access. Ask the person who mentioned you for access.",
                )
                .to_owned()
        })
    }
}
