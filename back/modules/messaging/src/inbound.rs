use chrono::DateTime;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::actor;
use crate::channels::{ChannelKind, InboundMessage};
use crate::store::{self, StoredChannel, StoredConversation, StoredMessage};
use crate::text::{self, EXCERPT_CHARS, PREVIEW_CHARS};
use crate::{MESSAGE_RECEIVED, error_code};
use aphel_kernel::Ctx;
use aphel_kernel::application::query::{Filter, FilterOperator, RecordQuery, RecordQueryEngine};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, DomainEvent, FieldChange, RecordRef, RecordType,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, field_set};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IngestOutcome {
    Stored {
        conversation_id: Uuid,
        message_id: Uuid,
    },
    Duplicate,
}

pub(crate) fn message_payload(
    channel: &StoredChannel,
    conversation: &StoredConversation,
    message: &StoredMessage,
) -> Value {
    json!({
        "conversation_id": conversation.id,
        "message_id": message.id,
        "channel_id": channel.id,
        "channel_kind": channel.kind.as_str(),
        "channel_name": channel.name,
        "direction": message.direction,
        "peer": conversation.peer,
        "peer_name": conversation.peer_name,
        "subject": message.subject,
        "excerpt": text::truncate(&message.body, EXCERPT_CHARS),
        "message_type": message.message_type,
        "status": message.status,
        "contact_id": conversation.contact_id,
        "deal_id": conversation.deal_id,
    })
}

pub(crate) fn linked_records(conversation: &StoredConversation) -> Vec<RecordRef> {
    conversation
        .contact_id
        .map(|id| RecordRef::new(RecordType::CONTACT, id))
        .into_iter()
        .chain(
            conversation
                .deal_id
                .map(|id| RecordRef::new(RecordType::DEAL, id)),
        )
        .collect()
}

pub(crate) async fn publish_on_records(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    event_type: &str,
    conversation: &StoredConversation,
    payload: Value,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(scope);
    let now = text::now().and_utc();
    let records = linked_records(conversation);
    if records.is_empty() {
        return ctx
            .events()
            .publish(
                tx,
                DomainEvent::new(actor, event_type, now).with_payload(payload),
            )
            .await;
    }
    for record in records {
        ctx.events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), event_type, now)
                    .with_record(record)
                    .with_payload(payload.clone()),
            )
            .await?;
    }
    Ok(())
}

pub(crate) async fn audit_conversation(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    event_type: &str,
    conversation: &StoredConversation,
    changes: Vec<FieldChange>,
) -> Result<(), AppError> {
    let mut entry = AuditEntry::new(Actor::from_scope(scope), event_type, text::now().and_utc())
        .with_changes(changes);
    if let Some(record) = linked_records(conversation).into_iter().next() {
        entry = entry.with_record(record);
    }
    ctx.audit().record(tx, entry).await
}

pub(crate) fn preview(message: &StoredMessage) -> Option<String> {
    let base = if message.body.trim().is_empty() {
        message.subject.clone().unwrap_or_default()
    } else {
        message.body.clone()
    };
    let preview = text::single_line(&base, PREVIEW_CHARS);
    (!preview.is_empty()).then_some(preview)
}

async fn contact_by_peer(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    kind: ChannelKind,
    peer: &str,
    field_keys: &[String],
) -> Option<Uuid> {
    let keys: Vec<&str> = match kind {
        ChannelKind::Email => vec!["email"],
        ChannelKind::Whatsapp => vec!["mobile", "phone"],
    }
    .into_iter()
    .filter(|key| field_keys.iter().any(|field| field == key))
    .collect();
    if keys.is_empty() {
        return None;
    }
    let conditions: Vec<Filter> = keys
        .iter()
        .map(|key| Filter::condition(*key, FilterOperator::Eq, json!(peer)))
        .collect();
    RecordQueryEngine::new(&ctx.query())
        .query(
            scope,
            RecordType::CONTACT.as_str(),
            RecordQuery {
                filter: Some(Filter::or(conditions)),
                window: aphel_kernel::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 1,
                },
                ..RecordQuery::default()
            },
        )
        .await
        .ok()
        .and_then(|page| page.items.into_iter().next())
}

async fn create_contact(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    values: Map<String, Value>,
) -> Result<Uuid, AppError> {
    let state = ctx.state();
    let created = ctx
        .registry()
        .handler(RecordType::CONTACT.as_str())?
        .create(&RecordCtx::new(scope, &state), Value::Object(values))
        .await?;
    created
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| AppError::Internal("the created contact has no id".to_owned()))
}

async fn resolve_contact(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    channel: &StoredChannel,
    message: &InboundMessage,
) -> Result<Option<Uuid>, AppError> {
    let keys: Vec<String> = match field_set(&ctx.state(), scope, RecordType::CONTACT.as_str()).await
    {
        Ok(fields) => fields
            .active()
            .map(|field| field.key().to_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    if let Some(id) = contact_by_peer(ctx, scope, channel.kind, &message.from, &keys).await {
        return Ok(Some(id));
    }
    if !actor::creates_unknown_contacts(ctx).await?
        || scope.scope_for(&Permission::CONTACTS_CREATE).is_none()
    {
        return Ok(None);
    }
    let display = message
        .from_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| message.from.clone());
    let (first_name, last_name) = text::split_name(&display);
    let mut values = Map::new();
    values.insert(
        "first_name".to_owned(),
        Value::String(text::truncate(&first_name, 140)),
    );
    if let Some(last_name) = last_name {
        values.insert(
            "last_name".to_owned(),
            Value::String(text::truncate(&last_name, 140)),
        );
    }
    let target = match channel.kind {
        ChannelKind::Email => Some("email"),
        ChannelKind::Whatsapp => ["mobile", "phone"]
            .into_iter()
            .find(|key| keys.iter().any(|field| field == key)),
    };
    if let Some(target) = target {
        values.insert(target.to_owned(), Value::String(message.from.clone()));
    }
    match create_contact(ctx, scope, values).await {
        Ok(id) => Ok(Some(id)),
        Err(error) => {
            store::record_channel_error(
                ctx.pool(),
                channel,
                &text::truncate(&format!("contact not created: {}", error_code(&error)), 500),
            )
            .await?;
            Ok(None)
        }
    }
}

async fn existing_conversation(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
    message: &InboundMessage,
) -> Result<Option<StoredConversation>, AppError> {
    let pool = ctx.pool().clone();
    match channel.kind {
        ChannelKind::Whatsapp => {
            store::latest_conversation_with_peer(&pool, channel.id, &message.from).await
        }
        ChannelKind::Email => {
            let ids: Vec<String> = message
                .in_reply_to
                .iter()
                .chain(message.references.iter())
                .cloned()
                .collect();
            match store::conversation_by_message_ids(&pool, channel.id, &ids).await? {
                Some(id) => store::find_conversation(&pool, id).await,
                None => Ok(None),
            }
        }
    }
}

pub(crate) async fn ingest(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
    message: InboundMessage,
) -> Result<IngestOutcome, AppError> {
    let pool = ctx.pool().clone();
    if store::message_exists(&pool, channel.id, &message.external_id).await? {
        return Ok(IngestOutcome::Duplicate);
    }
    let actor = actor::acting_scope(ctx, channel).await?;
    if actor.is_none() {
        store::record_channel_error(
            &pool,
            channel,
            "no active member can act for this channel; messages are stored without contact, events or audit",
        )
        .await?;
    }
    let existing = existing_conversation(ctx, channel, &message).await?;
    let contact_id = match existing
        .as_ref()
        .and_then(|conversation| conversation.contact_id)
    {
        Some(id) => Some(id),
        None => match &actor {
            Some(scope) => resolve_contact(ctx, scope, channel, &message).await?,
            None => None,
        },
    };
    let now = text::now();
    let stored = StoredMessage {
        id: Uuid::now_v7(),
        conversation_id: Uuid::nil(),
        channel_id: channel.id,
        direction: "in".to_owned(),
        external_id: Some(message.external_id.clone()),
        sender: message.from.clone(),
        sender_name: message.from_name.clone(),
        subject: message.subject.clone(),
        body: message.body.clone(),
        message_type: message.message_type.clone(),
        status: "received".to_owned(),
        error_code: None,
        error_message: None,
        in_reply_to: message.in_reply_to.clone(),
        thread_references: (!message.references.is_empty())
            .then(|| text::truncate(&message.references.join(" "), 8000)),
        sent_by_user_id: None,
        provider_timestamp: message
            .sent_at
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .map(|value| value.naive_utc()),
        sent_at: None,
        delivered_at: None,
        read_at: None,
        created_at: now,
    };
    let preview = preview(&stored);
    let mut tx = ctx.begin().await?;
    let locked = match &existing {
        Some(found) => store::find_conversation_for_update(&mut tx, found.id).await?,
        None => None,
    };
    let reopened;
    let conversation = match locked {
        Some(mut conversation) => {
            reopened = conversation.status == "resolved";
            if reopened {
                conversation.status = "open".to_owned();
            }
            conversation.unread_count = conversation.unread_count.saturating_add(1);
            conversation.last_message_at = now;
            conversation.last_inbound_at = Some(now);
            conversation.last_message_preview = preview;
            conversation.last_message_direction = Some("in".to_owned());
            if conversation.contact_id.is_none() {
                conversation.contact_id = contact_id;
            }
            if message.from_name.is_some() {
                conversation.peer_name = message.from_name.clone();
            }
            if conversation.subject.is_none() {
                conversation.subject = message.subject.clone();
            }
            store::save_conversation(&mut tx, &conversation).await?;
            conversation
        }
        None => {
            reopened = false;
            let conversation = StoredConversation {
                id: Uuid::now_v7(),
                channel_id: channel.id,
                peer: message.from.clone(),
                peer_name: message.from_name.clone(),
                subject: message.subject.clone(),
                contact_id,
                deal_id: None,
                status: "open".to_owned(),
                assignee_user_id: None,
                unread_count: 1,
                last_message_at: now,
                last_inbound_at: Some(now),
                last_message_preview: preview,
                last_message_direction: Some("in".to_owned()),
                created_at: now,
            };
            store::insert_conversation(&mut tx, &conversation).await?;
            conversation
        }
    };
    let stored = StoredMessage {
        conversation_id: conversation.id,
        ..stored
    };
    if !store::insert_message(&mut tx, &stored).await? {
        tx.rollback().await?;
        return Ok(IngestOutcome::Duplicate);
    }
    if let Some(scope) = &actor {
        let mut payload = message_payload(channel, &conversation, &stored);
        if let Value::Object(map) = &mut payload {
            map.insert("reopened".to_owned(), json!(reopened));
        }
        publish_on_records(
            ctx,
            &mut tx,
            scope,
            MESSAGE_RECEIVED,
            &conversation,
            payload,
        )
        .await?;
        audit_conversation(
            ctx,
            &mut tx,
            scope,
            MESSAGE_RECEIVED,
            &conversation,
            vec![
                FieldChange::new("conversation_id", Value::Null, json!(conversation.id)),
                FieldChange::new("message_id", Value::Null, json!(stored.id)),
                FieldChange::new("channel_id", Value::Null, json!(channel.id)),
            ],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(IngestOutcome::Stored {
        conversation_id: conversation.id,
        message_id: stored.id,
    })
}
