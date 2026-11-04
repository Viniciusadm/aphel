use serde_json::json;
use uuid::Uuid;

use crate::application::push::push_job;
use crate::application::{
    NewNotification, NotificationDirectory, NotificationPorts, NotificationRepository,
    preferences_of,
};
use crate::domain::NotificationChannel;
use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{Job, JobQueue, TransactionManager};
use aphel_kernel::errors::AppError;

pub const NOTIFICATION_EMAIL_JOB: &str = "notification.email";

pub use aphel_sdk::notifications::{DispatchOutcome, NotificationIntent, record_path};

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
            let (title, body) = crate::application::texts::render_localized(
                content,
                MailLocale::resolve(recipient.locale.as_deref(), None),
            );
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
