use serde_json::Value;

use crate::store::{LinkStore, MessageStore};
use crate::{EmailEventPayload, EmailMessageView};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{
    Actor, DomainEvent, EventPublisher, RecordRef, RecordType,
};
use aphel_sdk::platform::database::transaction::Tx;

pub(crate) const EMAIL_MESSAGE_RECORD: RecordType = RecordType::from_static("email.email_message");

pub(crate) fn email_record(message: &EmailMessageView) -> RecordRef {
    RecordRef::new(EMAIL_MESSAGE_RECORD, message.id)
}

pub(crate) async fn email_event_payload(
    ctx: &Ctx<'_>,
    message: &EmailMessageView,
    extra: Value,
) -> Result<EmailEventPayload, AppError> {
    let thread = MessageStore::of(ctx)
        .find_thread(message.thread_id)
        .await?
        .ok_or_else(|| AppError::Internal("e-mail thread disappeared".to_owned()))?;
    let records = LinkStore::of(ctx)
        .links_for_threads(&[thread.id])
        .await?
        .into_iter()
        .map(|link| link.record)
        .collect();
    Ok(EmailEventPayload {
        mailbox_id: message.mailbox_id,
        thread_id: thread.id,
        email_id: message.id,
        owner_id: thread.owner.id.clone(),
        direction: message.direction,
        kind: message.kind,
        visibility: thread.visibility,
        records,
        is_reply: message.in_reply_to.is_some(),
        extra,
    })
}

pub(crate) async fn publish_email_event(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    actor: &Actor,
    event_type: &str,
    payload: &EmailEventPayload,
) -> Result<(), AppError> {
    let now = ctx.now();
    let body = payload.to_value();
    for target in payload.targets() {
        let mut event = DomainEvent::new(actor.clone(), event_type, now).with_payload(body.clone());
        if let Some(record) = target {
            event = event.with_record(record);
        }
        ctx.events().publish(tx, event).await?;
    }
    Ok(())
}
