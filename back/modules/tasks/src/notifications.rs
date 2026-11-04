use aphel_kernel::application::mail::MailLocale;
use aphel_sdk::notifications::NotificationView;
use aphel_sdk::notifications::{NotificationTexts, data_text};
use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, event_dedupe, payload_text,
    payload_user,
};

use crate::domain::{ACTIVITY_CREATED, ACTIVITY_UPDATED};

const OPEN_STATUS: &str = "open";

pub(crate) struct TaskAssignedNotifications;

fn owner_change(payload: &Value) -> Option<UserId> {
    payload
        .get("changes")
        .and_then(Value::as_array)?
        .iter()
        .find(|change| change.get("field").and_then(Value::as_str) == Some("owner_id"))
        .and_then(|change| payload_user(change.get("new")))
}

#[async_trait]
impl NotificationSource for TaskAssignedNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(event_type, ACTIVITY_CREATED | ACTIVITY_UPDATED)
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let payload = &event.payload;
        let owner = match event.event_type.as_str() {
            ACTIVITY_CREATED => payload_user(payload.get("owner_id")),
            ACTIVITY_UPDATED => owner_change(payload),
            _ => None,
        };
        let Some(owner) = owner else {
            return Ok(Vec::new());
        };
        if payload.get("status").and_then(Value::as_str) != Some(OPEN_STATUS) {
            return Ok(Vec::new());
        }
        let kind = TASK_ASSIGNED_NOTIFICATION;
        Ok(vec![
            NotificationIntent::new(
                owner,
                kind,
                payload_text(payload, "title").unwrap_or_default(),
                event_dedupe(event, kind),
            )
            .with_record(event.record.clone())
            .with_data(json!({ "due_at": payload.get("due_at").cloned().unwrap_or(Value::Null) })),
        ])
    }
}

pub(crate) const TASK_ASSIGNED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("task_assigned");
pub use aphel_sdk::tasks::TASK_DUE_NOTIFICATION;

pub(crate) struct ActivityNotificationTexts;

impl NotificationTexts for ActivityNotificationTexts {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String> {
        let en = locale == MailLocale::En;
        Some(match view.kind {
            TASK_ASSIGNED_NOTIFICATION if en => format!("{actor} assigned you a task"),
            TASK_ASSIGNED_NOTIFICATION => format!("{actor} atribuiu uma tarefa a você"),
            TASK_DUE_NOTIFICATION => match data_text(view, "window") {
                Some("today") => locale
                    .pick(
                        "Você tem uma tarefa que vence hoje",
                        "You have a task due today",
                    )
                    .to_owned(),
                Some("reminder") => reminder_headline(view, locale),
                _ => locale
                    .pick(
                        "Você tem uma tarefa que vence em até 1 hora",
                        "You have a task due within 1 hour",
                    )
                    .to_owned(),
            },
            _ => return None,
        })
    }

    fn default_path(&self, view: &NotificationView) -> Option<&'static str> {
        (view.kind == TASK_DUE_NOTIFICATION).then_some("/tasks")
    }
}

fn reminder_headline(view: &NotificationView, locale: MailLocale) -> String {
    let minutes = view
        .data
        .get("minutes_before")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let unit = |count: u64, pt: (&str, &str), en: (&str, &str)| {
        let (singular, plural_form) = match locale {
            MailLocale::PtBr => pt,
            MailLocale::En => en,
        };
        plural(count, singular, plural_form)
    };
    let lead = match minutes {
        0 => {
            return locale
                .pick("Sua tarefa vence agora", "Your task is due now")
                .to_owned();
        }
        minutes if minutes % 1440 == 0 => unit(minutes / 1440, ("dia", "dias"), ("day", "days")),
        minutes if minutes % 60 == 0 => unit(minutes / 60, ("hora", "horas"), ("hour", "hours")),
        minutes => unit(minutes, ("minuto", "minutos"), ("minute", "minutes")),
    };
    match locale {
        MailLocale::PtBr => format!("Sua tarefa vence em {lead}"),
        MailLocale::En => format!("Your task is due in {lead}"),
    }
}

fn plural(count: u64, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}
