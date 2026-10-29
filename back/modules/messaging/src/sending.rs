use chrono::Duration as ChronoDuration;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::actor;
use crate::channels::{self, ChannelContext, ChannelError, ChannelKind, OutgoingMessage};
use crate::errors::{
    self, CHANNEL_INACTIVE, INVALID_REPLY, WHATSAPP_WINDOW_CLOSED, channel_not_found,
    conversation_not_found,
};
use crate::inbound::{audit_conversation, message_payload, preview, publish_on_records};
use crate::store::{self, StoredChannel, StoredConversation, StoredMessage};
use crate::text;
use crate::{MESSAGE_FAILED, MESSAGE_SENT, job_kind, user_uuid};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{FieldChange, Job, JobContext};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const SEND_JOB: &str = "send";
pub(crate) const SEND_ATTEMPTS: u32 = 5;
const WHATSAPP_WINDOW_HOURS: i64 = 24;

pub(crate) fn whatsapp_window_end(
    channel: &StoredChannel,
    conversation: &StoredConversation,
) -> Option<chrono::NaiveDateTime> {
    if channel.kind != ChannelKind::Whatsapp {
        return None;
    }
    conversation
        .last_inbound_at
        .map(|at| at + ChronoDuration::hours(WHATSAPP_WINDOW_HOURS))
}

fn reply_subject(conversation: &StoredConversation) -> Option<String> {
    let subject = conversation.subject.clone()?;
    let lower = subject.to_lowercase();
    if lower.starts_with("re:") {
        Some(subject)
    } else {
        Some(text::truncate(&format!("Re: {subject}"), 500))
    }
}

pub(crate) async fn queue_reply(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    conversation_id: Uuid,
    body: &str,
) -> Result<(StoredChannel, StoredConversation, StoredMessage), AppError> {
    let pool = ctx.pool().clone();
    let conversation = store::find_conversation(&pool, conversation_id)
        .await?
        .ok_or_else(conversation_not_found)?;
    let channel = store::find_channel(&pool, conversation.channel_id)
        .await?
        .ok_or_else(channel_not_found)?;
    if !channel.is_active {
        return Err(AppError::conflict_code(
            CHANNEL_INACTIVE,
            "the channel of this conversation is inactive",
        ));
    }
    let body = text::clean_body(body);
    let max = channel.kind.handler().max_body_chars();
    if body.is_empty() {
        return Err(AppError::invalid_fields_code(
            INVALID_REPLY,
            "write a message",
            vec![FieldViolation::new("body", "escreva a mensagem")],
        ));
    }
    if body.chars().count() > max {
        return Err(AppError::invalid_fields_code(
            INVALID_REPLY,
            "the message is too long",
            vec![FieldViolation::new(
                "body",
                format!("a mensagem pode ter até {max} caracteres"),
            )],
        ));
    }
    if channel.kind == ChannelKind::Whatsapp
        && whatsapp_window_end(&channel, &conversation).is_none_or(|end| end < text::now())
    {
        return Err(AppError::validation_code(
            WHATSAPP_WINDOW_CLOSED,
            "more than 24 hours since the last message from the customer; WhatsApp only accepts approved template messages now",
        ));
    }
    let now = text::now();
    let mut tx = ctx.begin().await?;
    let mut conversation = store::find_conversation_for_update(&mut tx, conversation_id)
        .await?
        .ok_or_else(conversation_not_found)?;
    let message = StoredMessage {
        id: Uuid::now_v7(),
        conversation_id,
        channel_id: channel.id,
        direction: "out".to_owned(),
        external_id: None,
        sender: channel.address(),
        sender_name: None,
        subject: match channel.kind {
            ChannelKind::Email => reply_subject(&conversation),
            ChannelKind::Whatsapp => None,
        },
        body,
        message_type: "text".to_owned(),
        status: "queued".to_owned(),
        error_code: None,
        error_message: None,
        in_reply_to: None,
        thread_references: None,
        sent_by_user_id: Some(user_uuid(scope)?),
        provider_timestamp: None,
        sent_at: None,
        delivered_at: None,
        read_at: None,
        created_at: now,
    };
    store::insert_message(&mut tx, &message).await?;
    conversation.last_message_at = now;
    conversation.last_message_preview = preview(&message);
    conversation.last_message_direction = Some("out".to_owned());
    conversation.unread_count = 0;
    store::save_conversation(&mut tx, &conversation).await?;
    ctx.jobs()
        .enqueue(
            &mut tx,
            Job::new(job_kind(SEND_JOB), json!({ "message_id": message.id }))
                .with_max_attempts(SEND_ATTEMPTS),
        )
        .await?;
    publish_on_records(
        ctx,
        &mut tx,
        scope,
        MESSAGE_SENT,
        &conversation,
        message_payload(&channel, &conversation, &message),
    )
    .await?;
    audit_conversation(
        ctx,
        &mut tx,
        scope,
        MESSAGE_SENT,
        &conversation,
        vec![
            FieldChange::new("conversation_id", Value::Null, json!(conversation.id)),
            FieldChange::new("message_id", Value::Null, json!(message.id)),
            FieldChange::new("channel_id", Value::Null, json!(channel.id)),
        ],
    )
    .await?;
    tx.commit().await?;
    Ok((channel, conversation, message))
}

fn email_domain(address: &str) -> String {
    address
        .split_once('@')
        .map(|(_, domain)| domain.to_owned())
        .unwrap_or_else(|| "aphel.local".to_owned())
}

async fn outgoing(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
    conversation: &StoredConversation,
    message: &StoredMessage,
) -> Result<OutgoingMessage, AppError> {
    let mut outgoing = OutgoingMessage {
        to: conversation.peer.clone(),
        to_name: conversation.peer_name.clone(),
        body: message.body.clone(),
        subject: message.subject.clone(),
        message_id: format!(
            "{}@{}",
            message.id.simple(),
            email_domain(&channel.address())
        ),
        in_reply_to: None,
        references: Vec::new(),
    };
    if channel.kind == ChannelKind::Email
        && let Some(last) = store::last_inbound_message(ctx.pool(), conversation.id).await?
        && let Some(id) = last.external_id.filter(|id| !id.starts_with("imap-"))
    {
        let mut references: Vec<String> = last
            .thread_references
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        references.push(id.clone());
        let skip = references.len().saturating_sub(20);
        outgoing.references = references.into_iter().skip(skip).collect();
        outgoing.in_reply_to = Some(id);
    }
    Ok(outgoing)
}

async fn fail(
    ctx: &Ctx<'_>,
    scope: Option<&AccessScope>,
    channel: &StoredChannel,
    conversation: &StoredConversation,
    message: &StoredMessage,
    error: &ChannelError,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    store::mark_message_failed(&mut tx, message, error.code, &error.message).await?;
    if let Some(scope) = scope {
        let failed = StoredMessage {
            status: "failed".to_owned(),
            ..message.clone()
        };
        let mut payload = message_payload(channel, conversation, &failed);
        if let Value::Object(map) = &mut payload {
            map.insert("error_code".to_owned(), json!(error.code));
        }
        publish_on_records(ctx, &mut tx, scope, MESSAGE_FAILED, conversation, payload).await?;
        audit_conversation(
            ctx,
            &mut tx,
            scope,
            MESSAGE_FAILED,
            conversation,
            vec![
                FieldChange::new("message_id", Value::Null, json!(message.id)),
                FieldChange::new("error_code", Value::Null, json!(error.code)),
            ],
        )
        .await?;
    }
    Ok(tx.commit().await?)
}

pub(crate) async fn deliver(
    ctx: &Ctx<'_>,
    job: &JobContext,
    message_id: Uuid,
) -> Result<(), AppError> {
    let pool = ctx.pool().clone();
    let Some(message) = store::find_message(&pool, message_id).await? else {
        return Ok(());
    };
    if !store::claim_message_for_sending(&pool, message_id).await? {
        return Ok(());
    }
    let Some(conversation) = store::find_conversation(&pool, message.conversation_id).await? else {
        return Ok(());
    };
    let Some(channel) = store::find_channel(&pool, message.channel_id).await? else {
        return Ok(());
    };
    let scope = actor::user_scope_or_actor(ctx, &channel, message.sent_by_user_id).await?;
    let Some(scope) = scope else {
        let error = ChannelError::permanent(
            errors::ACTING_USER_UNAVAILABLE,
            "nobody can send through this channel anymore",
        );
        return fail(ctx, None, &channel, &conversation, &message, &error).await;
    };
    if !channel.is_active {
        let error = ChannelError::permanent(CHANNEL_INACTIVE, "the channel is inactive");
        return fail(ctx, Some(&scope), &channel, &conversation, &message, &error).await;
    }
    let credentials = channels::load_credentials(ctx, &channel).await?;
    let outgoing = outgoing(ctx, &channel, &conversation, &message).await?;
    let context = ChannelContext {
        config: &channel.config,
        credentials: &credentials,
    };
    match channel.kind.handler().send(&context, &outgoing).await {
        Ok(sent) => {
            let references = (!outgoing.references.is_empty())
                .then(|| text::truncate(&outgoing.references.join(" "), 8000));
            store::mark_message_sent(
                &pool,
                &message,
                &text::truncate(&sent.external_id, 255),
                outgoing.in_reply_to.as_deref(),
                references.as_deref(),
            )
            .await
        }
        Err(error) if error.permanent || job.attempt >= job.max_attempts => {
            fail(ctx, Some(&scope), &channel, &conversation, &message, &error).await
        }
        Err(error) => {
            store::record_message_attempt(&pool, &message, error.code, &error.message).await?;
            Err(AppError::unavailable_code(error.code, error.message))
        }
    }
}
