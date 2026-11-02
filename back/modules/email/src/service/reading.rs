use aphel_contract_crm::CrmPermissions;
use std::collections::{BTreeMap, BTreeSet};

use futures::StreamExt;
use serde_json::json;
use uuid::Uuid;

use crate::domain::EmailVisibility;
use crate::mailboxes::mailbox_not_found;
use crate::reading::{
    AttachmentDownload, EMAIL_THREAD_RECORD_TYPE, EMAIL_THREAD_VISIBILITY_CHANGED, MessageDetail,
    ThreadDetail, ThreadSummary, attachment_not_found, record_access_denied, sanitize_email_html,
    thread_not_found,
};
use crate::store::settings::DirectoryStore;
use crate::store::{LinkStore, MailboxStore, MessageStore};
use crate::{EmailBody, EmailMessageView, ParticipantView, RecordLinkView, ThreadView};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef,
};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::records::MySqlRecordAccessChecker;

pub(super) fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}

pub(crate) async fn list_for_record(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    window: ListWindow,
) -> Result<Listing<ThreadSummary>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::EMAILS_READ)?;
    let record = RecordRef::parse(record_type, record_id)?;
    if !record_access(ctx).can_read(scope, &record).await? {
        return Err(record_access_denied(&record));
    }
    let listing = MessageStore::of(ctx)
        .list_threads_for_record(&record, scope.user_id(), window)
        .await?;
    Ok(Listing {
        items: summaries(ctx, listing.items).await?,
        total: listing.total,
    })
}

pub(crate) async fn list_mine(
    ctx: &Ctx<'_>,
    mailbox_id: Option<&str>,
    window: ListWindow,
) -> Result<Listing<ThreadSummary>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::EMAILS_READ)?;
    let emails = MessageStore::of(ctx);
    let listing = match mailbox_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => {
            let id = Uuid::parse_str(id).map_err(|_| {
                AppError::invalid_fields(
                    "invalid query",
                    vec![FieldViolation::new("mailbox_id", "must be a UUID")],
                )
            })?;
            let mailbox = MailboxStore::of(ctx)
                .find(id)
                .await?
                .filter(|mailbox| mailbox.is_owned_by(scope.user_id()))
                .ok_or_else(mailbox_not_found)?;
            emails.list_threads_for_mailbox(mailbox.id, window).await?
        }
        None => {
            let ids = DirectoryStore::of(ctx)
                .threads_for_owner(scope.user_id(), window)
                .await?;
            let mut items = Vec::with_capacity(ids.items.len());
            for id in ids.items {
                if let Some(thread) = emails.find_thread(id).await? {
                    items.push(thread);
                }
            }
            Listing {
                items,
                total: ids.total,
            }
        }
    };
    Ok(Listing {
        items: summaries(ctx, listing.items).await?,
        total: listing.total,
    })
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<ThreadDetail, AppError> {
    ctx.scope.require(&Permission::EMAILS_READ)?;
    let summary = visible(ctx, id).await?;
    let messages = MessageStore::of(ctx)
        .thread_messages(summary.thread.id)
        .await?;
    let mut details = Vec::with_capacity(messages.len());
    for message in messages {
        let body = if summary.access.can_read_content() {
            body(ctx, &message).await?
        } else {
            None
        };
        details.push(MessageDetail { message, body });
    }
    Ok(ThreadDetail {
        summary,
        messages: details,
    })
}

pub(crate) async fn set_visibility(
    ctx: &Ctx<'_>,
    id: &str,
    visibility: Option<&str>,
    ip: Option<String>,
) -> Result<ThreadDetail, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::EMAILS_READ)?;
    let summary = visible(ctx, id).await?;
    if !summary.is_owner {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "only the mailbox owner can change the visibility of a conversation",
        ));
    }
    let visibility = match visibility.map(str::trim) {
        None => None,
        Some(value) => Some(EmailVisibility::parse(value).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid conversation",
                vec![FieldViolation::new(
                    "visibility",
                    "must be private, metadata, shared or null",
                )],
            )
        })?),
    };
    let thread = &summary.thread;
    if visibility == thread.visibility_override {
        return get(ctx, id).await;
    }
    let changes = diff_changes(
        &json!({ "visibility": thread.visibility_override.map(|value| value.as_str()) }),
        &json!({ "visibility": visibility.map(|value| value.as_str()) }),
        &DiffOptions::default(),
    )?;
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(EMAIL_THREAD_RECORD_TYPE, thread.id);
    let mut tx = ctx.begin().await?;
    MessageStore::of(ctx)
        .set_thread_visibility(&mut tx, thread.id, visibility, now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), EMAIL_THREAD_VISIBILITY_CHANGED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "thread_id": thread.id,
                    "mailbox_id": thread.mailbox_id,
                    "visibility": visibility.map(|value| value.as_str()),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, EMAIL_THREAD_VISIBILITY_CHANGED, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    get(ctx, id).await
}

pub(crate) async fn attachment(
    ctx: &Ctx<'_>,
    message_id: &str,
    attachment_id: &str,
) -> Result<AttachmentDownload, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::EMAILS_READ)?;
    let message_id = Uuid::parse_str(message_id).map_err(|_| attachment_not_found())?;
    let attachment_id = Uuid::parse_str(attachment_id).map_err(|_| attachment_not_found())?;
    let message = MessageStore::of(ctx)
        .find_message(message_id)
        .await?
        .ok_or_else(attachment_not_found)?;
    let summary = visible(ctx, &message.thread_id.to_string())
        .await
        .map_err(|_| attachment_not_found())?;
    if !summary.access.can_read_content() {
        return Err(attachment_not_found());
    }
    let attachment = message
        .attachments
        .into_iter()
        .find(|attachment| attachment.id == attachment_id)
        .ok_or_else(attachment_not_found)?;
    let key = DirectoryStore::of(ctx)
        .file_storage_key(attachment.file_id)
        .await?
        .ok_or_else(attachment_not_found)?;
    let content = ctx.storage()?.get_stream(&key).await?;
    Ok(AttachmentDownload {
        attachment,
        content,
    })
}

pub(crate) async fn visible(ctx: &Ctx<'_>, id: &str) -> Result<ThreadSummary, AppError> {
    let id = Uuid::parse_str(id).map_err(|_| thread_not_found())?;
    let thread = MessageStore::of(ctx)
        .find_thread(id)
        .await?
        .ok_or_else(thread_not_found)?;
    let summary = summaries(ctx, vec![thread])
        .await?
        .into_iter()
        .next()
        .ok_or_else(thread_not_found)?;
    if summary.is_owner {
        return Ok(summary);
    }
    let records = record_access(ctx);
    for link in &summary.records {
        if records.can_read(ctx.scope, &link.record).await? {
            return Ok(summary);
        }
    }
    Err(thread_not_found())
}

async fn body(ctx: &Ctx<'_>, message: &EmailMessageView) -> Result<Option<EmailBody>, AppError> {
    let Some(file_id) = message.body_file_id else {
        return Ok(None);
    };
    let Some(key) = DirectoryStore::of(ctx).file_storage_key(file_id).await? else {
        return Ok(None);
    };
    let mut stream = ctx.storage()?.get_stream(&key).await?;
    let mut content = Vec::new();
    while let Some(chunk) = stream.next().await {
        content.extend_from_slice(&chunk?);
    }
    let mut body = EmailBody::decode(&content)?;
    body.html = body.html.as_deref().map(sanitize_email_html);
    Ok(Some(body))
}

async fn summaries(
    ctx: &Ctx<'_>,
    threads: Vec<ThreadView>,
) -> Result<Vec<ThreadSummary>, AppError> {
    let scope = ctx.scope;
    let ids: Vec<Uuid> = threads.iter().map(|thread| thread.id).collect();
    let mut links: BTreeMap<Uuid, Vec<RecordLinkView>> = BTreeMap::new();
    for link in LinkStore::of(ctx).links_for_threads(&ids).await? {
        links.entry(link.thread_id).or_default().push(link);
    }
    let mut participants: BTreeMap<Uuid, Vec<ParticipantView>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for entry in DirectoryStore::of(ctx).thread_participants(&ids).await? {
        if seen.insert((
            entry.thread_id,
            entry.participant.role.as_str(),
            entry.participant.address.clone(),
        )) {
            participants
                .entry(entry.thread_id)
                .or_default()
                .push(entry.participant);
        }
    }
    Ok(threads
        .into_iter()
        .filter_map(|thread| {
            let is_owner = thread.is_owned_by(scope.user_id());
            let access = thread.visibility.access_for(is_owner);
            if !access.can_see() {
                return None;
            }
            Some(ThreadSummary {
                participants: participants.remove(&thread.id).unwrap_or_default(),
                records: links.remove(&thread.id).unwrap_or_default(),
                access,
                is_owner,
                thread,
            })
        })
        .collect())
}
