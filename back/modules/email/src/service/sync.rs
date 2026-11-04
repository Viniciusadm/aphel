use aphel_contract_crm::CrmRecordTypes;
use std::collections::{BTreeMap, BTreeSet};

use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::capture::{
    ParticipantFilter, company_name_for_domain, is_personal_domain, split_display_name,
};
use crate::domain::{
    EmailDirection, EmailKind, EmailStatus, LinkOrigin, MailboxStatus, ParticipantRole,
    address_domain, classify_message, email_preview, normalize_address, normalize_message_id,
};
use crate::hooks::{LinkingEmail, apply_linking};
use crate::mailboxes::status_reason;
use crate::store::settings::{DirectoryStore, SettingsStore};
use crate::store::{LinkStore, MailboxStore, MessageStore};
use crate::sync::{
    AUTHENTICATION_FAILURE, CaptureSettings, MAX_HEADER_ID_LENGTH, MAX_PARTICIPANT_NAME_LENGTH,
    MAX_PROVIDER_MESSAGE_ID_LENGTH, MAX_SUBJECT_LENGTH, SyncMailboxRequest, SyncOutcome,
    SyncSettings, html_to_text, sync_mailbox_job, synthetic_message_id,
};
use crate::{
    EMAIL_RECEIVED, EMAIL_SENT, EmailAddress, EmailBody, EmailEventPayload, FetchedEmail,
    MailProviderError, MailProviderErrorKind, MailboxSyncState, MailboxView, NewEmailAttachment,
    NewEmailMessage, NewParticipant, NewRecordLink, NewThread, SyncRequest,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::files::{NewFile, sanitize_file_name, sha256_hex, storage_key};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, JobQueue, RecordRef,
    RecordType, SharedFileStorage,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;

const MAX_BACKOFF_MINUTES: i64 = 360;
const BODY_FILE_NAME: &str = "message.json";

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn clean_name(name: Option<&str>) -> Option<String> {
    name.map(|name| {
        truncate(
            name.trim().trim_matches('"').trim(),
            MAX_PARTICIPANT_NAME_LENGTH,
        )
    })
    .filter(|name| !name.is_empty())
}

fn header_id(value: Option<&str>) -> Option<String> {
    value
        .and_then(normalize_message_id)
        .map(|id| truncate(&id, MAX_HEADER_ID_LENGTH))
}

fn backoff(failures: u32) -> Duration {
    let minutes = 5i64.saturating_mul(1i64 << failures.min(10));
    Duration::minutes(minutes.min(MAX_BACKOFF_MINUTES))
}

struct ParsedParticipants {
    participants: Vec<NewParticipant>,
    from: Option<String>,
}

fn collect_participants(email: &FetchedEmail) -> ParsedParticipants {
    let mut participants = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push = |role: ParticipantRole, address: &EmailAddress| {
        let Ok(normalized) = normalize_address(&address.address) else {
            return None;
        };
        if seen.insert((role.as_str(), normalized.clone())) {
            participants.push(NewParticipant {
                role,
                address: normalized.clone(),
                name: clean_name(address.name.as_deref()),
                contact_id: None,
            });
        }
        Some(normalized)
    };
    let from = email
        .from
        .as_ref()
        .and_then(|address| push(ParticipantRole::From, address));
    for (role, list) in [
        (ParticipantRole::To, &email.to),
        (ParticipantRole::Cc, &email.cc),
        (ParticipantRole::Bcc, &email.bcc),
        (ParticipantRole::ReplyTo, &email.reply_to),
    ] {
        for address in list {
            push(role, address);
        }
    }
    ParsedParticipants { participants, from }
}

fn capture_candidates(
    participants: &[NewParticipant],
    direction: EmailDirection,
    external: &[String],
) -> Vec<(String, Option<String>)> {
    let roles: &[ParticipantRole] = match direction {
        EmailDirection::Inbound => &[ParticipantRole::From],
        EmailDirection::Outbound => &[ParticipantRole::To, ParticipantRole::Cc],
    };
    let mut seen = BTreeSet::new();
    participants
        .iter()
        .filter(|participant| roles.contains(&participant.role))
        .filter(|participant| external.contains(&participant.address))
        .filter(|participant| seen.insert(participant.address.clone()))
        .map(|participant| (participant.address.clone(), participant.name.clone()))
        .collect()
}

struct SyncContext {
    filter: ParticipantFilter,
    capture: CaptureSettings,
}

pub(crate) async fn dispatch(ctx: &Ctx<'_>, settings: SyncSettings) -> Result<u32, AppError> {
    let now = ctx.now();
    let due = MailboxStore::of(ctx)
        .due_for_sync(
            now - Duration::minutes(i64::from(settings.interval_minutes)),
            settings.dispatch_limit,
        )
        .await?;
    if due.is_empty() {
        return Ok(0);
    }
    let ids: Vec<Uuid> = due.iter().map(|mailbox| mailbox.mailbox_id).collect();
    let paused = DirectoryStore::of(ctx).paused_mailboxes(&ids, now).await?;
    let mut queued = 0;
    for mailbox in due {
        if paused.contains(&mailbox.mailbox_id) {
            continue;
        }
        ctx.jobs()
            .enqueue_now(sync_mailbox_job(mailbox.mailbox_id, &mailbox.owner))
            .await?;
        queued += 1;
    }
    Ok(queued)
}

pub(crate) async fn sync_mailbox(
    ctx: &Ctx<'_>,
    settings: SyncSettings,
    request: &SyncMailboxRequest,
) -> Result<SyncOutcome, AppError> {
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&request.owner)
        .await?
    else {
        return Ok(SyncOutcome::default());
    };
    let scope = build_access_scope(request.owner.clone(), access);
    let ctx = ctx.with_scope(&scope);
    let ctx = &ctx;
    let Some(mailbox) = MailboxStore::of(ctx)
        .find(request.mailbox_id)
        .await?
        .filter(|mailbox| mailbox.is_owned_by(&request.owner))
        .filter(|mailbox| mailbox.status.can_sync())
    else {
        return Ok(SyncOutcome::default());
    };
    let state = ctx.state();
    let started_at = ctx.now();
    let provider = match crate::store::mail_providers().require_for(ctx.scope, &mailbox.provider) {
        Ok(provider) => provider,
        Err(_) => {
            save_state(
                ctx,
                mailbox.id,
                MailboxStatus::Error,
                Some("the mail provider is not available".to_owned()),
                None,
                None,
                None,
            )
            .await?;
            return Ok(SyncOutcome::default());
        }
    };
    let mut account = crate::service::mailboxes::account(ctx, &mailbox).await?;
    save_state(
        ctx,
        mailbox.id,
        MailboxStatus::Syncing,
        None,
        None,
        Some(started_at),
        None,
    )
    .await?;
    let context = sync_context(ctx).await?;
    let mut cursor = mailbox.sync_cursor.clone();
    let since = started_at - Duration::days(i64::from(mailbox.history_days));
    let mut outcome = SyncOutcome::default();
    let mut has_more = false;
    for _ in 0..settings.max_batches.max(1) {
        let batch = match provider
            .sync_since(
                &account,
                SyncRequest {
                    cursor: cursor.clone(),
                    since,
                    limit: settings.batch_size,
                },
            )
            .await
        {
            Ok(batch) => batch,
            Err(error) => {
                fail(ctx, &mailbox, &error).await?;
                return Ok(outcome);
            }
        };
        if let Some(credentials) = batch.credentials.clone() {
            crate::service::mailboxes::persist_credentials(ctx, mailbox.id, &credentials).await?;
            account.credentials = credentials;
        }
        for email in batch.messages {
            if import(ctx, &state.storage, &mailbox, &context, email).await? {
                outcome.imported += 1;
            } else {
                outcome.skipped += 1;
            }
        }
        cursor = Some(batch.cursor.clone());
        has_more = batch.has_more;
        save_state(
            ctx,
            mailbox.id,
            MailboxStatus::Syncing,
            None,
            Some(batch.cursor),
            Some(started_at),
            None,
        )
        .await?;
        if !has_more {
            break;
        }
    }
    let finished_at = ctx.now();
    DirectoryStore::of(ctx)
        .clear_sync_failure(mailbox.id)
        .await?;
    save_state(
        ctx,
        mailbox.id,
        MailboxStatus::Active,
        None,
        None,
        None,
        (!has_more).then_some(finished_at),
    )
    .await?;
    Ok(outcome)
}

async fn sync_context(ctx: &Ctx<'_>) -> Result<SyncContext, AppError> {
    let settings = SettingsStore::of(ctx);
    let mut internal = MailboxStore::of(ctx).organization_addresses().await?;
    internal.extend(DirectoryStore::of(ctx).member_addresses().await?);
    let blocklist = settings.blocklist().await?;
    Ok(SyncContext {
        filter: ParticipantFilter::new(internal, &blocklist),
        capture: settings.capture_settings().await?,
    })
}

async fn fail(
    ctx: &Ctx<'_>,
    mailbox: &MailboxView,
    error: &MailProviderError,
) -> Result<(), AppError> {
    let now = ctx.now();
    let directory = DirectoryStore::of(ctx);
    let current = directory.sync_failure(mailbox.id).await?;
    let failures = current.map(|failure| failure.failures).unwrap_or_default();
    let retry_after = match error.kind {
        MailProviderErrorKind::Authentication => None,
        _ => Some(now + backoff(failures)),
    };
    let kind = if error.kind == MailProviderErrorKind::Authentication {
        AUTHENTICATION_FAILURE
    } else {
        error.kind.as_str()
    };
    directory
        .record_sync_failure(mailbox.id, kind, retry_after, now)
        .await?;
    tracing::warn!(
        mailbox_id = %mailbox.id,
        kind = error.kind.as_str(),
        "mailbox synchronization failed"
    );
    save_state(
        ctx,
        mailbox.id,
        MailboxStatus::Error,
        Some(status_reason(&error.to_string())),
        None,
        None,
        None,
    )
    .await
}

async fn save_state(
    ctx: &Ctx<'_>,
    id: Uuid,
    status: MailboxStatus,
    reason: Option<String>,
    cursor: Option<serde_json::Value>,
    sync_started_at: Option<DateTime<Utc>>,
    last_synced_at: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    let now = ctx.now();
    MailboxStore::of(ctx)
        .save_sync_state(
            id,
            &MailboxSyncState {
                status,
                status_reason: reason,
                cursor,
                sync_started_at,
                last_synced_at,
            },
            now,
        )
        .await
}

async fn import(
    ctx: &Ctx<'_>,
    storage: &SharedFileStorage,
    mailbox: &MailboxView,
    context: &SyncContext,
    email: FetchedEmail,
) -> Result<bool, AppError> {
    let emails = MessageStore::of(ctx);
    let directory = DirectoryStore::of(ctx);
    let links = LinkStore::of(ctx);
    let message_id =
        header_id(email.message_id.as_deref()).unwrap_or_else(|| synthetic_message_id(&email));
    if emails.message_exists(mailbox.id, &message_id).await? {
        return Ok(false);
    }
    let ParsedParticipants {
        mut participants,
        from,
    } = collect_participants(&email);
    let direction = if email.sent_by_account || from.as_deref() == Some(&mailbox.address) {
        EmailDirection::Outbound
    } else {
        EmailDirection::Inbound
    };
    let external: BTreeSet<String> = participants
        .iter()
        .map(|participant| participant.address.clone())
        .filter(|address| address != &mailbox.address && context.filter.is_external(address))
        .collect();
    if external.is_empty() {
        return Ok(false);
    }
    let kind = classify_message(
        from.as_deref(),
        email.content_type.as_deref(),
        email.auto_submitted.as_deref(),
    );
    let external: Vec<String> = external.into_iter().collect();
    let mut people: BTreeMap<String, Uuid> = BTreeMap::new();
    for found in crate::party_directory::contacts(ctx, &external, false).await? {
        people.entry(found.address).or_insert(found.contact_id);
    }
    let mut captured: Vec<(String, Uuid)> = Vec::new();
    if context.capture.create_contacts && kind == EmailKind::Message {
        for (address, name) in capture_candidates(&participants, direction, &external) {
            if people.contains_key(&address) {
                continue;
            }
            match capture_contact(ctx, mailbox, &context.capture, &address, name, direction).await {
                Ok(Some(contact_id)) => {
                    people.insert(address.clone(), contact_id);
                    captured.push((address, contact_id));
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(
                    mailbox_id = %mailbox.id,
                    "could not capture a contact from e-mail: {error}"
                ),
            }
        }
    }
    for participant in &mut participants {
        participant.contact_id = people.get(&participant.address).copied();
    }

    let thread_lookup: Vec<String> = header_id(email.in_reply_to.as_deref())
        .into_iter()
        .chain(email.references.iter().filter_map(|id| header_id(Some(id))))
        .collect();
    let existing_thread = emails
        .find_thread_by_message_ids(mailbox.id, &thread_lookup)
        .await?;
    let (thread_id, visibility, mut records) = match existing_thread {
        Some(thread_id) => {
            let visibility = emails
                .find_thread(thread_id)
                .await?
                .map(|thread| thread.visibility)
                .unwrap_or(mailbox.default_visibility);
            let records: Vec<RecordRef> = links
                .links_for_threads(&[thread_id])
                .await?
                .into_iter()
                .map(|link| link.record)
                .collect();
            (thread_id, visibility, records)
        }
        None => (Uuid::now_v7(), mailbox.default_visibility, Vec::new()),
    };
    let contact_ids: Vec<Uuid> = people
        .values()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut new_links: Vec<RecordRef> = contact_ids
        .iter()
        .map(|id| RecordRef::new(RecordType::CONTACT, *id))
        .collect();
    if !contact_ids.is_empty() {
        new_links.extend(
            crate::party_directory::contact_records(
                ctx,
                &contact_ids,
                context.capture.link_open_deals,
            )
            .await?,
        );
    }
    let linking = apply_linking(
        ctx,
        LinkingEmail {
            mailbox_id: mailbox.id,
            owner_id: mailbox.owner.clone(),
            thread_id,
            message_id: message_id.clone(),
            direction,
            kind,
            from: from.clone(),
            participants: participants
                .iter()
                .map(|participant| participant.address.clone())
                .collect(),
            contact_ids: contact_ids.clone(),
            records: new_links,
        },
    )
    .await?;
    let new_links = linking.records;
    for record in &new_links {
        if !records.contains(record) {
            records.push(record.clone());
        }
    }
    let mut previous_threads: Vec<(String, Uuid, Vec<Uuid>)> = Vec::new();
    for (address, contact_id) in &captured {
        let threads = directory.threads_with_address(mailbox.id, address).await?;
        previous_threads.push((address.clone(), *contact_id, threads));
    }

    let now = ctx.now();
    let owner = mailbox.owner.clone();
    let mut files = Vec::new();
    let text = email
        .text
        .clone()
        .or_else(|| email.html.as_deref().map(html_to_text));
    let preview = text.as_deref().map(email_preview).unwrap_or_default();
    let body = EmailBody {
        text: email.text.clone(),
        html: email.html.clone(),
    };
    let body_file_id = if body.text.is_some() || body.html.is_some() {
        let content = body.encode()?;
        Some(
            store_file(
                storage,
                &owner,
                BODY_FILE_NAME,
                EmailBody::MIME_TYPE,
                content,
                now,
                &mut files,
            )
            .await?,
        )
    } else {
        None
    };
    let mut attachments = Vec::new();
    for attachment in email.attachments {
        let name = sanitize_file_name(&attachment.name);
        let size = attachment.content.len() as u64;
        let file_id = store_file(
            storage,
            &owner,
            &name,
            &attachment.mime_type,
            attachment.content,
            now,
            &mut files,
        )
        .await?;
        attachments.push(NewEmailAttachment {
            file_id,
            name: truncate(&name, 255),
            mime_type: truncate(&attachment.mime_type, 255),
            size,
            content_id: attachment
                .content_id
                .map(|id| truncate(id.trim_matches(['<', '>']), 255)),
            inline: attachment.inline,
        });
    }
    let message = NewEmailMessage {
        id: Uuid::now_v7(),
        mailbox_id: mailbox.id,
        thread_id,
        direction,
        kind,
        status: match direction {
            EmailDirection::Inbound => EmailStatus::Received,
            EmailDirection::Outbound => EmailStatus::Sent,
        },
        message_id,
        in_reply_to: header_id(email.in_reply_to.as_deref()),
        references: email
            .references
            .iter()
            .filter_map(|id| header_id(Some(id)))
            .collect(),
        report_for_message_id: header_id(email.report_for_message_id.as_deref()),
        provider_message_id: email
            .provider_message_id
            .as_deref()
            .map(|id| truncate(id, MAX_PROVIDER_MESSAGE_ID_LENGTH)),
        subject: truncate(email.subject.trim(), MAX_SUBJECT_LENGTH),
        preview,
        body_file_id,
        has_html: email.html.is_some(),
        tracking_enabled: false,
        sent_at: Some(email.date.unwrap_or(now)),
        scheduled_at: None,
        created_by: None,
        participants,
        attachments,
    };

    let mut tx = ctx.begin().await?;
    for file in &files {
        directory.insert_file(&mut tx, file).await?;
    }
    if existing_thread.is_none() {
        emails
            .insert_thread(
                &mut tx,
                &NewThread {
                    id: thread_id,
                    mailbox_id: mailbox.id,
                    subject: message.subject.clone(),
                    visibility: None,
                },
                now,
            )
            .await?;
    }
    if !emails.insert_message(&mut tx, &message, now).await? {
        drop(tx);
        discard_files(storage, &files).await;
        return Ok(false);
    }
    for record in &new_links {
        link(&links, &mut tx, thread_id, record, now).await?;
    }
    for (address, contact_id) in &people {
        emails
            .set_participant_contact(&mut tx, thread_id, address, Some(*contact_id))
            .await?;
    }
    for (address, contact_id, threads) in &previous_threads {
        for previous in threads.iter().filter(|previous| **previous != thread_id) {
            emails
                .set_participant_contact(&mut tx, *previous, address, Some(*contact_id))
                .await?;
            link(
                &links,
                &mut tx,
                *previous,
                &RecordRef::new(RecordType::CONTACT, *contact_id),
                now,
            )
            .await?;
        }
    }
    emails.refresh_thread(&mut tx, thread_id, now).await?;
    let payload = EmailEventPayload {
        mailbox_id: mailbox.id,
        thread_id,
        email_id: message.id,
        owner_id: owner,
        direction,
        kind,
        visibility,
        records,
        is_reply: message.in_reply_to.is_some() || !message.references.is_empty(),
        extra: json!({ "source": "sync" }),
    };
    publish(ctx, &mut tx, &payload, now).await?;
    tx.commit().await?;
    Ok(true)
}

async fn link(
    links: &LinkStore,
    tx: &mut MySqlTx,
    thread_id: Uuid,
    record: &RecordRef,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    links
        .link(
            tx,
            &NewRecordLink {
                thread_id,
                record: record.clone(),
                origin: LinkOrigin::Automatic,
                created_by: None,
            },
            now,
        )
        .await?;
    Ok(())
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    payload: &EmailEventPayload,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let event_type = match payload.direction {
        EmailDirection::Inbound => EMAIL_RECEIVED,
        EmailDirection::Outbound => EMAIL_SENT,
    };
    let body = payload.to_value();
    for target in payload.targets() {
        let mut event = DomainEvent::new(Actor::System, event_type, now).with_payload(body.clone());
        if let Some(record) = target {
            event = event.with_record(record);
        }
        ctx.events().publish(tx, event).await?;
    }
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(Actor::System, event_type, now)
                .with_record(RecordRef::new(
                    crate::compose::EMAIL_MESSAGE_RECORD,
                    payload.email_id,
                ))
                .with_changes(vec![
                    FieldChange::new("mailbox_id", Value::Null, json!(payload.mailbox_id)),
                    FieldChange::new("thread_id", Value::Null, json!(payload.thread_id)),
                    FieldChange::new("direction", Value::Null, json!(payload.direction.as_str())),
                    FieldChange::new("kind", Value::Null, json!(payload.kind.as_str())),
                    FieldChange::new("source", Value::Null, json!("sync")),
                ]),
        )
        .await?;
    Ok(())
}

async fn store_file(
    storage: &SharedFileStorage,
    owner: &UserId,
    name: &str,
    mime_type: &str,
    content: Vec<u8>,
    now: DateTime<Utc>,
    files: &mut Vec<NewFile>,
) -> Result<Uuid, AppError> {
    let id = Uuid::now_v7();
    let key = storage_key(id, now);
    let checksum = sha256_hex(&content);
    let size = content.len() as u64;
    storage.put(&key, Bytes::from(content)).await?;
    files.push(NewFile {
        id,
        storage_key: key,
        original_name: truncate(name, 255),
        mime_type: truncate(mime_type, 127),
        size,
        checksum,
        uploaded_by: owner.clone(),
        created_at: now,
    });
    Ok(id)
}

async fn discard_files(storage: &SharedFileStorage, files: &[NewFile]) {
    for file in files {
        if let Err(error) = storage.delete(&file.storage_key).await {
            tracing::warn!(file_id = %file.id, "could not delete a discarded e-mail file: {error}");
        }
    }
}

async fn capture_contact(
    ctx: &Ctx<'_>,
    mailbox: &MailboxView,
    settings: &CaptureSettings,
    address: &str,
    name: Option<String>,
    direction: EmailDirection,
) -> Result<Option<Uuid>, AppError> {
    let person_type = RecordType::parse(aphel_sdk::people::RECORD_TYPE)?;
    if ctx.registry().get(&person_type).is_none() || !aphel_sdk::records::enabled(ctx, &person_type)
    {
        return Ok(None);
    }
    let organization_type = RecordType::parse(aphel_sdk::organizations::RECORD_TYPE)?;
    let directory = DirectoryStore::of(ctx);
    let opposite = match direction {
        EmailDirection::Inbound => EmailDirection::Outbound,
        EmailDirection::Outbound => EmailDirection::Inbound,
    };
    if !directory
        .exchanged_with(mailbox.id, address, opposite)
        .await?
    {
        return Ok(None);
    }
    let mut company_id = None;
    if settings.create_companies
        && ctx.registry().get(&organization_type).is_some()
        && aphel_sdk::records::enabled(ctx, &organization_type)
        && let Some(domain) = address_domain(address)
        && !is_personal_domain(domain)
    {
        company_id = match crate::party_directory::company_by_domain(ctx, domain).await? {
            Some(id) => Some(id),
            None => Some(create_company(ctx, &company_name_for_domain(domain), domain).await?),
        };
    }
    let contact_id = create_contact(ctx, address, name.as_deref(), company_id).await?;
    Ok(Some(contact_id))
}

async fn create_contact(
    ctx: &Ctx<'_>,
    address: &str,
    name: Option<&str>,
    company_id: Option<Uuid>,
) -> Result<Uuid, AppError> {
    let (first_name, last_name) = split_display_name(name, address);
    let mut values = serde_json::Map::new();
    values.insert("first_name".into(), json!(first_name));
    values.insert("email".into(), json!(address));
    values.insert("source".into(), json!("email"));
    if let Some(last_name) = last_name {
        values.insert("last_name".into(), json!(last_name));
    }
    if let Some(company_id) = company_id {
        values.insert("company_id".into(), json!(company_id));
    }
    for (key, value) in
        aphel_sdk::party::capture_defaults(ctx, &RecordType::parse(aphel_sdk::people::RECORD_TYPE)?)
            .await?
    {
        if values.insert(key.clone(), value).is_some() {
            return Err(AppError::Configuration(format!(
                "party capture default conflicts with {key}"
            )));
        }
    }
    created_id(
        ctx.create_record(aphel_sdk::people::RECORD_TYPE, Value::Object(values))
            .await?,
    )
}

async fn create_company(ctx: &Ctx<'_>, name: &str, domain: &str) -> Result<Uuid, AppError> {
    created_id(
        ctx.create_record(
            aphel_sdk::organizations::RECORD_TYPE,
            json!({ "name": name, "domain": domain, "source": "email" }),
        )
        .await?,
    )
}

fn created_id(record: Value) -> Result<Uuid, AppError> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| AppError::Internal("created capture record has no identity".into()))
}
