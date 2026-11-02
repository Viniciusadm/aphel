use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::mail::MailLocale;
use crate::application::notifications::push::push_job;
use crate::application::notifications::texts::LocalizedContent;
use crate::application::notifications::{
    NewNotification, NotificationDirectory, NotificationPorts, NotificationRepository,
    preferences_of,
};
use crate::application::security::Clock;
use crate::application::shared::{Job, JobQueue, RecordRef, TransactionManager};
use crate::domain::UserId;
use crate::domain::notifications::{NotificationChannel, NotificationKind};
use crate::errors::AppError;

pub const NOTIFICATION_EMAIL_JOB: &str = "notification.email";

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationIntent {
    pub recipient: UserId,
    pub kind: NotificationKind,
    pub title: String,
    pub body: Option<String>,
    pub record: Option<RecordRef>,
    pub actor_id: Option<UserId>,
    pub link: Option<String>,
    pub data: Value,
    pub dedupe_key: String,
    pub localized: Option<LocalizedContent>,
}

impl NotificationIntent {
    pub fn new(
        recipient: UserId,
        kind: NotificationKind,
        title: impl Into<String>,
        dedupe_key: impl Into<String>,
    ) -> Self {
        Self {
            recipient,
            kind,
            title: title.into(),
            body: None,
            record: None,
            actor_id: None,
            link: None,
            data: json!({}),
            dedupe_key: dedupe_key.into(),
            localized: None,
        }
    }

    pub fn with_localized(mut self, content: LocalizedContent) -> Self {
        self.localized = Some(content);
        self
    }

    pub fn with_body(mut self, body: Option<String>) -> Self {
        self.body = body;
        self
    }

    pub fn with_record(mut self, record: Option<RecordRef>) -> Self {
        self.link = record.as_ref().and_then(record_path);
        self.record = record;
        self
    }

    pub fn with_link(mut self, link: Option<String>) -> Self {
        self.link = link;
        self
    }

    pub fn with_actor(mut self, actor_id: Option<UserId>) -> Self {
        self.actor_id = actor_id;
        self
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
    Delivered {
        notification_id: Uuid,
        in_app: bool,
        email: bool,
        push: bool,
    },
    Duplicate,
    Muted,
    SkippedRecipient,
}

pub fn record_path(record: &RecordRef) -> Option<String> {
    let record_type = record.record_type.as_str();
    if let Some(key) = record_type.strip_prefix("co_") {
        return Some(format!("/objects/{key}/{}", record.record_id));
    }
    let base = crate::modules::app()
        .record_types()
        .capabilities(record_type)?
        .web_path_value()?
        .to_owned();
    Some(format!("/{base}/{}", record.record_id))
}

pub fn email_job(notification_id: Uuid) -> Job {
    Job::new(
        NOTIFICATION_EMAIL_JOB,
        json!({
            "notification_id": notification_id.to_string(),
        }),
    )
    .with_unique_key(format!("{NOTIFICATION_EMAIL_JOB}:{notification_id}"))
}

pub async fn dispatch<P>(ports: &P, intent: NotificationIntent) -> Result<DispatchOutcome, AppError>
where
    P: NotificationPorts,
{
    if intent.actor_id.as_ref() == Some(&intent.recipient) {
        return Ok(DispatchOutcome::SkippedRecipient);
    }
    let Some(recipient) = ports.directory().recipient(&intent.recipient).await? else {
        return Ok(DispatchOutcome::SkippedRecipient);
    };
    let preferences = preferences_of(ports.preferences(), &intent.recipient).await?;
    let enabled = |channel: NotificationChannel| {
        preferences.iter().any(|setting| {
            setting.kind == intent.kind && setting.channel == channel && setting.enabled
        })
    };
    let in_app = enabled(NotificationChannel::InApp);
    let email = enabled(NotificationChannel::Email);
    let push = enabled(NotificationChannel::Push);
    if !in_app && !email && !push {
        return Ok(DispatchOutcome::Muted);
    }
    let (title, body) = match &intent.localized {
        Some(content) => {
            let (title, body) =
                content.render(MailLocale::resolve(recipient.locale.as_deref(), None));
            (title, body.or(intent.body))
        }
        None => (intent.title, intent.body),
    };
    let notification = NewNotification {
        id: Uuid::now_v7(),
        user_id: intent.recipient,
        kind: intent.kind,
        title,
        body,
        record: intent.record,
        actor_id: intent.actor_id,
        link: intent.link,
        data: intent.data,
        dedupe_key: intent.dedupe_key,
        in_app,
        created_at: ports.clock().now(),
    };
    let mut tx = ports.transactions().begin().await?;
    if !ports.notifications().insert(&mut tx, &notification).await? {
        return Ok(DispatchOutcome::Duplicate);
    }
    if email {
        ports
            .jobs()
            .enqueue(&mut tx, email_job(notification.id))
            .await?;
    }
    if push {
        ports
            .jobs()
            .enqueue(&mut tx, push_job(notification.id))
            .await?;
    }
    ports.transactions().commit(tx).await?;
    Ok(DispatchOutcome::Delivered {
        notification_id: notification.id,
        in_app,
        email,
        push,
    })
}
