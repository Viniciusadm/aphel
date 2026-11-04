use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::HashMap;

use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::compose::events::{
    EMAIL_MESSAGE_RECORD, email_event_payload, email_record, publish_email_event,
};
use crate::compose::html::{html_to_text, sanitize_html};
use crate::compose::{
    BODY_FILE_NAME, ComposedEmail, DeliveryOutcome, EMAIL_ATTACHMENTS_TOO_LARGE, EMAIL_CANCELLED,
    EMAIL_FAILED, EMAIL_NOT_CANCELLABLE, EMAIL_SCHEDULED, MAILBOX_UNAVAILABLE,
    MAX_ATTACHMENT_BYTES, MAX_ATTACHMENTS, MAX_BODY_BYTES, MAX_FAILURE_REASON_LENGTH,
    MAX_RECIPIENTS, MAX_SCHEDULE_DAYS, MAX_SUBJECT_LENGTH, NO_MAILBOX_AVAILABLE, OwnerEmailRequest,
    RecordInput, SEND_SCHEDULED_JOB, SendEmailCommand, email_not_found, generate_message_id,
    invalid_email as invalid, mailbox_not_found,
};
use crate::domain::{
    EmailDirection, EmailKind, EmailStatus, LinkOrigin, MailboxStatus, ParticipantRole,
    email_preview, normalize_address, normalize_message_id, reply_subject,
};
use crate::hooks::OutgoingMessage;
use crate::service::records::{self, can_read};
use crate::service::templates::{find_visible_template, load_variable_values};
use crate::service::tracking::tracking_key;
use crate::store::sending::{OutboundStore, TrackingStore};
use crate::store::{LinkStore, MailboxStore, MessageStore};
use crate::templates::{is_email_record_type, render_email, render_template};
use crate::tracking::{address_hash, tracked_html};
use crate::{
    EMAIL_SENT, EmailAddress, EmailAttachmentContent, EmailBody, EmailMessageView,
    MAILBOX_CREDENTIALS_SECRET, MAILBOX_SECRET_OWNER, MailProviderError, MailboxAccount,
    MailboxCredentials, MailboxView, MessageStatusChange, NewEmailAttachment, NewEmailMessage,
    NewParticipant, NewRecordLink, NewThread, ProviderEmail, SentEmail, SharedMailboxProvider,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::files::{FileRepository, NewFile, sha256_hex, storage_key};
use aphel_kernel::application::secrets::{SecretKey, SecretVault};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, Job, JobQueue,
    RecordRef, RecordType,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::storage::mysql_files::MySqlFileRepository;

const IMMEDIATE_TOLERANCE_SECONDS: i64 = 5;

fn clean_name(name: Option<&str>) -> Option<String> {
    name.map(|name| name.replace(['\r', '\n', '"'], " ").trim().to_owned())
        .filter(|name| !name.is_empty())
        .map(|name| name.chars().take(200).collect())
}

fn normalize_recipients(
    field: &str,
    addresses: &[EmailAddress],
    seen: &mut Vec<String>,
    violations: &mut Vec<FieldViolation>,
) -> Vec<EmailAddress> {
    let mut normalized = Vec::new();
    for (index, recipient) in addresses.iter().enumerate() {
        match normalize_address(&recipient.address) {
            Ok(address) => {
                if seen.contains(&address) {
                    continue;
                }
                seen.push(address.clone());
                normalized.push(EmailAddress {
                    address,
                    name: clean_name(recipient.name.as_deref()),
                });
            }
            Err(message) => violations.push(FieldViolation::new(
                format!("{field}.{index}.address"),
                message,
            )),
        }
    }
    normalized
}

fn truncate_reason(message: &str) -> String {
    message.chars().take(MAX_FAILURE_REASON_LENGTH).collect()
}

fn participants_of(message: &EmailMessageView, role: ParticipantRole) -> Vec<EmailAddress> {
    message
        .participants_with(role)
        .map(|participant| EmailAddress {
            address: participant.address.clone(),
            name: participant.name.clone(),
        })
        .collect()
}

struct Draft {
    mailbox: MailboxView,
    thread_id: Option<Uuid>,
    in_reply_to: Option<String>,
    references: Vec<String>,
    subject: String,
    body: EmailBody,
    to: Vec<EmailAddress>,
    cc: Vec<EmailAddress>,
    bcc: Vec<EmailAddress>,
    record: Option<RecordRef>,
    attachments: Vec<NewEmailAttachment>,
    scheduled_at: Option<DateTime<Utc>>,
    tracking: bool,
}

pub(crate) async fn send(
    ctx: &Ctx<'_>,
    command: SendEmailCommand,
) -> Result<ComposedEmail, AppError> {
    ctx.scope.require(&Permission::EMAILS_SEND)?;
    let sender = ctx.scope.user_id().clone();
    compose(ctx, &sender, command).await
}

pub async fn enqueue_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    mut command: SendEmailCommand,
) -> Result<Uuid, AppError> {
    ctx.scope.require(&Permission::EMAILS_SEND)?;
    let sender = ctx.scope.user_id();
    command.scheduled_at.get_or_insert_with(|| ctx.now());
    let draft = draft(ctx, sender, command.clone()).await?;
    let draft = filter_outgoing_in_tx(ctx, tx, sender, draft).await?;
    let message_id = generate_message_id(&draft.mailbox.address);
    let recipients: Vec<String> = draft
        .to
        .iter()
        .chain(&draft.cc)
        .chain(&draft.bcc)
        .map(|recipient| recipient.address.clone())
        .collect();
    let mut people = std::collections::HashMap::new();
    for found in crate::party_directory::contacts(ctx, &recipients, true).await? {
        people.entry(found.address).or_insert(found.contact_id);
    }
    let body_file = store_body(ctx, sender, &draft.body).await?;
    let id = Uuid::now_v7();
    persist_draft_in_tx(
        ctx,
        tx,
        sender,
        &draft,
        id,
        &message_id,
        &body_file,
        &people,
        command.ip,
    )
    .await?;
    Ok(id)
}

pub(crate) async fn send_as_owner(
    ctx: &Ctx<'_>,
    request: OwnerEmailRequest,
) -> Result<ComposedEmail, AppError> {
    ctx.scope.require(&Permission::EMAILS_SEND)?;
    let owner = UserId::new(request.owner_id)
        .map_err(|_| AppError::Validation("the record has no valid owner".to_owned()))?;
    compose(
        ctx,
        &owner,
        SendEmailCommand {
            to: request.to,
            subject: request.subject,
            body_html: request.body_html,
            template_id: request.template_id,
            record: Some(request.record),
            tracking: request.tracking,
            ..SendEmailCommand::default()
        },
    )
    .await
}

pub(crate) async fn cancel(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<ComposedEmail, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::EMAILS_SEND)?;
    let message = own_message(ctx, id).await?;
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    if !OutboundStore::of(ctx)
        .transition_status(
            &mut tx,
            message.id,
            EmailStatus::Scheduled,
            EmailStatus::Cancelled,
            now,
        )
        .await?
    {
        return Err(AppError::conflict_code(
            EMAIL_NOT_CANCELLABLE,
            "only scheduled e-mails that were not sent yet can be cancelled",
        ));
    }
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), EMAIL_CANCELLED, now).with_payload(json!({
                "email_id": message.id,
                "thread_id": message.thread_id,
                "mailbox_id": message.mailbox_id,
            })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, EMAIL_CANCELLED, now)
                .with_record(email_record(&message))
                .with_changes(vec![FieldChange::new(
                    "status",
                    json!(EmailStatus::Scheduled.as_str()),
                    json!(EmailStatus::Cancelled.as_str()),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    composed(ctx, message.id).await
}

pub(crate) async fn send_scheduled(
    ctx: &Ctx<'_>,
    email_id: Uuid,
    can_retry: bool,
) -> Result<DeliveryOutcome, AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let claimed = OutboundStore::of(ctx)
        .transition_status(
            &mut tx,
            email_id,
            EmailStatus::Scheduled,
            EmailStatus::Sending,
            now,
        )
        .await?;
    tx.commit().await?;
    if !claimed {
        if let Some(message) = MessageStore::of(ctx).find_message(email_id).await?
            && message.status == EmailStatus::Sending
        {
            let error = AppError::conflict_code(
                "email_delivery_unknown",
                "the previous send has an unknown outcome; confirm delivery with the provider before sending again",
            );
            record_failure(ctx, &message, &Actor::System, None, error.client_message()).await?;
            return Ok(DeliveryOutcome::Failed(error));
        }
        return Ok(DeliveryOutcome::Skipped);
    }
    let requested_by = ctx.scope.user_id().clone();
    deliver(ctx, email_id, Actor::System, Some(requested_by), can_retry).await
}

pub(crate) async fn fail_scheduled(
    ctx: &Ctx<'_>,
    email_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let claimed = OutboundStore::of(ctx)
        .transition_status(
            &mut tx,
            email_id,
            EmailStatus::Scheduled,
            EmailStatus::Sending,
            now,
        )
        .await?;
    tx.commit().await?;
    if !claimed {
        return Ok(());
    }
    let Some(message) = MessageStore::of(ctx).find_message(email_id).await? else {
        return Ok(());
    };
    record_failure(ctx, &message, &Actor::System, None, reason).await
}

pub(crate) async fn composed(ctx: &Ctx<'_>, id: Uuid) -> Result<ComposedEmail, AppError> {
    let message = MessageStore::of(ctx)
        .find_message(id)
        .await?
        .ok_or_else(email_not_found)?;
    let body = match message.body_file_id {
        Some(file_id) => read_body(ctx, file_id).await?,
        None => EmailBody::default(),
    };
    Ok(ComposedEmail { message, body })
}

async fn own_message(ctx: &Ctx<'_>, id: &str) -> Result<EmailMessageView, AppError> {
    let scope = ctx.scope;
    let id = Uuid::parse_str(id.trim()).map_err(|_| email_not_found())?;
    let message = MessageStore::of(ctx)
        .find_message(id)
        .await?
        .filter(|message| message.direction == EmailDirection::Outbound)
        .ok_or_else(email_not_found)?;
    let mailbox = MailboxStore::of(ctx)
        .find(message.mailbox_id)
        .await?
        .ok_or_else(email_not_found)?;
    if mailbox.is_owned_by(scope.user_id()) || message.created_by.as_ref() == Some(scope.user_id())
    {
        return Ok(message);
    }
    Err(email_not_found())
}

async fn compose(
    ctx: &Ctx<'_>,
    sender: &UserId,
    command: SendEmailCommand,
) -> Result<ComposedEmail, AppError> {
    let mut command = command;
    command.scheduled_at.get_or_insert_with(|| ctx.now());
    let draft = draft(ctx, sender, command.clone()).await?;
    let mut tx = ctx.begin().await?;
    let draft = filter_outgoing_in_tx(ctx, &mut tx, sender, draft).await?;
    let message_id = generate_message_id(&draft.mailbox.address);
    let recipients: Vec<String> = draft
        .to
        .iter()
        .chain(&draft.cc)
        .chain(&draft.bcc)
        .map(|recipient| recipient.address.clone())
        .collect();
    let mut people = std::collections::HashMap::new();
    for found in crate::party_directory::contacts(ctx, &recipients, true).await? {
        people.entry(found.address).or_insert(found.contact_id);
    }
    let body_file = store_body(ctx, sender, &draft.body).await?;
    let email_id = Uuid::now_v7();
    let result = persist_draft_in_tx(
        ctx,
        &mut tx,
        sender,
        &draft,
        email_id,
        &message_id,
        &body_file,
        &people,
        command.ip.clone(),
    )
    .await;
    if let Err(error) = result {
        tx.rollback().await?;
        if let Err(cleanup) = ctx.storage()?.delete(&body_file.storage_key).await {
            tracing::warn!(error = %cleanup, "failed to remove orphan e-mail body");
        }
        return Err(error);
    }
    tx.commit().await?;
    composed(ctx, email_id).await
}

async fn filter_outgoing_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    sender: &UserId,
    mut draft: Draft,
) -> Result<Draft, AppError> {
    let message = ctx
        .apply_hook(
            crate::hooks::OutgoingEmail,
            tx,
            crate::KEY,
            OutgoingMessage {
                mailbox_id: draft.mailbox.id,
                sender_id: sender.clone(),
                from: EmailAddress {
                    address: draft.mailbox.address.clone(),
                    name: draft.mailbox.display_name.clone(),
                },
                to: draft.to.clone(),
                cc: draft.cc.clone(),
                bcc: draft.bcc.clone(),
                subject: draft.subject.clone(),
                body: draft.body.clone(),
                record: draft.record.clone(),
                thread_id: draft.thread_id,
                scheduled: draft.scheduled_at.is_some(),
                tracking: draft.tracking,
            },
        )
        .await?;
    if message.to.is_empty() {
        return Err(invalid("to", "must have at least one recipient"));
    }
    if message.subject.trim().is_empty() {
        return Err(invalid("subject", "must not be empty"));
    }
    draft.to = message.to;
    draft.cc = message.cc;
    draft.bcc = message.bcc;
    draft.subject = message.subject;
    draft.body = message.body;
    draft.tracking = draft.tracking && message.tracking;
    Ok(draft)
}

async fn draft(
    ctx: &Ctx<'_>,
    sender: &UserId,
    command: SendEmailCommand,
) -> Result<Draft, AppError> {
    let emails = MessageStore::of(ctx);
    let mailboxes = MailboxStore::of(ctx);
    let mut record = match &command.record {
        Some(RecordInput {
            record_type,
            record_id,
        }) => {
            let record = RecordRef::parse(record_type, record_id)?;
            if !is_email_record_type(record.record_type.as_str()) {
                return Err(invalid(
                    "record.record_type",
                    "must be contact, company, deal or a custom object",
                ));
            }
            if !can_read(ctx, &record).await? {
                return Err(AppError::forbidden_code(
                    PERMISSION_DENIED,
                    format!(
                        "missing permission to access {} records",
                        record.record_type
                    ),
                ));
            }
            Some(record)
        }
        None => None,
    };

    let replied = match command
        .reply_to_message_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => {
            let id = Uuid::parse_str(id).map_err(|_| email_not_found())?;
            let message = emails.find_message(id).await?.ok_or_else(email_not_found)?;
            let mailbox = mailboxes
                .find(message.mailbox_id)
                .await?
                .filter(|mailbox| mailbox.is_owned_by(sender))
                .ok_or_else(email_not_found)?;
            Some((message, mailbox))
        }
        None => None,
    };

    let mailbox = match command
        .mailbox_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => {
            let id = Uuid::parse_str(id).map_err(|_| mailbox_not_found())?;
            let mailbox = mailboxes
                .find(id)
                .await?
                .filter(|mailbox| mailbox.is_owned_by(sender))
                .ok_or_else(mailbox_not_found)?;
            if let Some((_, replied_mailbox)) = &replied
                && replied_mailbox.id != mailbox.id
            {
                return Err(invalid(
                    "mailbox_id",
                    "a reply must be sent from the mailbox of the conversation",
                ));
            }
            mailbox
        }
        None => match &replied {
            Some((_, mailbox)) => mailbox.clone(),
            None => default_mailbox(ctx, sender).await?,
        },
    };
    if !mailbox.status.can_send() {
        return Err(AppError::validation_code(
            MAILBOX_UNAVAILABLE,
            "the mailbox is disconnected; reconnect it before sending",
        ));
    }
    if record.is_none()
        && let Some((message, _)) = &replied
    {
        record = thread_record(ctx, message.thread_id).await?;
    }

    let mut violations = Vec::new();
    let mut seen = Vec::new();
    let to = normalize_recipients("to", &command.to, &mut seen, &mut violations);
    let cc = normalize_recipients("cc", &command.cc, &mut seen, &mut violations);
    let bcc = normalize_recipients("bcc", &command.bcc, &mut seen, &mut violations);
    if to.is_empty() && violations.is_empty() {
        violations.push(FieldViolation::new(
            "to",
            "must have at least one recipient",
        ));
    }
    if to.len() + cc.len() + bcc.len() > MAX_RECIPIENTS {
        violations.push(FieldViolation::new(
            "to",
            format!("must have at most {MAX_RECIPIENTS} recipients in total"),
        ));
    }
    if command.attachment_file_ids.len() > MAX_ATTACHMENTS {
        violations.push(FieldViolation::new(
            "attachment_file_ids",
            format!("must have at most {MAX_ATTACHMENTS} files"),
        ));
    }
    let now = ctx.now();
    let scheduled_at = match command.scheduled_at {
        Some(at) if at > now + Duration::seconds(IMMEDIATE_TOLERANCE_SECONDS) => {
            if at > now + Duration::days(MAX_SCHEDULE_DAYS) {
                violations.push(FieldViolation::new(
                    "scheduled_at",
                    format!("must be at most {MAX_SCHEDULE_DAYS} days ahead"),
                ));
            }
            Some(at)
        }
        _ => Some(now),
    };

    let template = match command
        .template_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => Some(find_visible_template(ctx, id).await?),
        None => None,
    };
    let subject_source = command
        .subject
        .clone()
        .filter(|subject| !subject.trim().is_empty())
        .or_else(|| template.as_ref().map(|template| template.subject.clone()))
        .or_else(|| replied.as_ref().map(|(message, _)| message.subject.clone()))
        .unwrap_or_default();
    let html_source = command
        .body_html
        .clone()
        .filter(|body| !body.trim().is_empty())
        .or_else(|| template.as_ref().map(|template| template.body_html.clone()));
    let text_source = command
        .body_text
        .clone()
        .filter(|body| !body.trim().is_empty());
    if html_source.is_none() && text_source.is_none() {
        violations.push(FieldViolation::new(
            "body_html",
            "body_html, body_text or template_id is required",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid e-mail", violations));
    }

    let values = load_variable_values(ctx, sender, record.as_ref()).await?;
    let rendered = render_email(
        &subject_source,
        html_source.as_deref().unwrap_or_default(),
        &values,
    );
    let mut subject = rendered.subject;
    if replied.is_some() {
        subject = reply_subject(&subject);
    }
    if subject.is_empty() {
        return Err(invalid("subject", "must not be empty"));
    }
    if subject.chars().count() > MAX_SUBJECT_LENGTH {
        return Err(invalid(
            "subject",
            format!("must have at most {MAX_SUBJECT_LENGTH} characters"),
        ));
    }
    let body = match (&html_source, &text_source) {
        (Some(_), Some(text)) => EmailBody {
            text: Some(render_template(
                text,
                &values,
                false,
                &mut Default::default(),
            )),
            html: Some(rendered.html),
        },
        (Some(_), None) => EmailBody {
            text: Some(rendered.text),
            html: Some(rendered.html),
        },
        (None, Some(text)) => EmailBody {
            text: Some(render_template(
                text,
                &values,
                false,
                &mut Default::default(),
            )),
            html: None,
        },
        (None, None) => EmailBody::default(),
    };
    let body_size =
        body.text.as_ref().map_or(0, String::len) + body.html.as_ref().map_or(0, String::len);
    if body_size > MAX_BODY_BYTES {
        return Err(invalid(
            "body_html",
            format!("must have at most {MAX_BODY_BYTES} bytes"),
        ));
    }

    let mut attachments = Vec::new();
    let mut total_size = 0u64;
    for file_id in &command.attachment_file_ids {
        let file = records::describe_attachment(ctx, file_id).await?;
        if attachments
            .iter()
            .any(|existing: &NewEmailAttachment| existing.file_id == file.file_id)
        {
            continue;
        }
        total_size += file.size;
        attachments.push(NewEmailAttachment {
            file_id: file.file_id,
            name: file.name,
            mime_type: file.mime_type,
            size: file.size,
            content_id: None,
            inline: false,
        });
    }
    if total_size > MAX_ATTACHMENT_BYTES {
        return Err(AppError::validation_code(
            EMAIL_ATTACHMENTS_TOO_LARGE,
            format!(
                "attachments must add up to at most {} MB",
                MAX_ATTACHMENT_BYTES / (1024 * 1024)
            ),
        ));
    }

    let settings = TrackingStore::of(ctx).settings().await?;
    let requested = command.tracking.unwrap_or(settings.default_on);
    let tracking = requested
        && settings.any_enabled()
        && body.html.is_some()
        && !objected(ctx, &to, &cc, &bcc).await?;

    let (thread_id, in_reply_to, references) = match &replied {
        Some((message, _)) => {
            let mut references = message.references.clone();
            if !references.contains(&message.message_id) {
                references.push(message.message_id.clone());
            }
            (
                Some(message.thread_id),
                normalize_message_id(&message.message_id),
                references,
            )
        }
        None => (None, None, Vec::new()),
    };

    Ok(Draft {
        mailbox,
        thread_id,
        in_reply_to,
        references,
        subject,
        body,
        to,
        cc,
        bcc,
        record,
        attachments,
        scheduled_at,
        tracking,
    })
}

async fn thread_record(ctx: &Ctx<'_>, thread_id: Uuid) -> Result<Option<RecordRef>, AppError> {
    let mut links = LinkStore::of(ctx).links_for_threads(&[thread_id]).await?;
    links.sort_by_key(|link| link.record.record_type.as_str() != "people.person");
    for link in links {
        if is_email_record_type(link.record.record_type.as_str())
            && can_read(ctx, &link.record).await?
        {
            return Ok(Some(link.record));
        }
    }
    Ok(None)
}

async fn objected(
    ctx: &Ctx<'_>,
    to: &[EmailAddress],
    cc: &[EmailAddress],
    bcc: &[EmailAddress],
) -> Result<bool, AppError> {
    let hashes: Vec<String> = to
        .iter()
        .chain(cc)
        .chain(bcc)
        .map(|recipient| address_hash(&recipient.address))
        .collect();
    TrackingStore::of(ctx).has_objection(&hashes).await
}

async fn default_mailbox(ctx: &Ctx<'_>, sender: &UserId) -> Result<MailboxView, AppError> {
    let mailboxes = MailboxStore::of(ctx).list_for_user(sender).await?;
    mailboxes
        .iter()
        .find(|mailbox| mailbox.status == MailboxStatus::Active)
        .or_else(|| mailboxes.iter().find(|mailbox| mailbox.status.can_send()))
        .cloned()
        .ok_or_else(|| {
            AppError::validation_code(
                NO_MAILBOX_AVAILABLE,
                "connect a mailbox before sending e-mails",
            )
        })
}

async fn store_body(ctx: &Ctx<'_>, sender: &UserId, body: &EmailBody) -> Result<NewFile, AppError> {
    let content = body.encode()?;
    let now = ctx.now();
    let file_id = Uuid::now_v7();
    let file = NewFile {
        id: file_id,
        storage_key: storage_key(file_id, now),
        original_name: BODY_FILE_NAME.to_owned(),
        mime_type: EmailBody::MIME_TYPE.to_owned(),
        size: content.len() as u64,
        checksum: sha256_hex(&content),
        uploaded_by: sender.clone(),
        created_at: now,
    };
    ctx.storage()?
        .put(&file.storage_key, Bytes::from(content))
        .await?;
    Ok(file)
}

#[allow(clippy::too_many_arguments)]
async fn persist_draft_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    sender: &UserId,
    draft: &Draft,
    email_id: Uuid,
    message_id: &str,
    body_file: &NewFile,
    people: &HashMap<String, Uuid>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let emails = MessageStore::of(ctx);
    let now = ctx.now();
    let mut participants = vec![NewParticipant {
        role: ParticipantRole::From,
        address: draft.mailbox.address.clone(),
        name: draft.mailbox.display_name.clone(),
        contact_id: None,
    }];
    for (role, recipients) in [
        (ParticipantRole::To, &draft.to),
        (ParticipantRole::Cc, &draft.cc),
        (ParticipantRole::Bcc, &draft.bcc),
    ] {
        participants.extend(recipients.iter().map(|recipient| NewParticipant {
            role,
            address: recipient.address.clone(),
            name: recipient.name.clone(),
            contact_id: people.get(&recipient.address).copied(),
        }));
    }
    let preview_source = draft
        .body
        .text
        .clone()
        .or_else(|| draft.body.html.as_deref().map(html_to_text))
        .unwrap_or_default();
    let status = if draft.scheduled_at.is_some() {
        EmailStatus::Scheduled
    } else {
        EmailStatus::Sending
    };
    let thread_id = match draft.thread_id {
        Some(thread_id) => thread_id,
        None => {
            let thread_id = Uuid::now_v7();
            emails
                .insert_thread(
                    tx,
                    &NewThread {
                        id: thread_id,
                        mailbox_id: draft.mailbox.id,
                        subject: draft.subject.clone(),
                        visibility: None,
                    },
                    now,
                )
                .await?;
            thread_id
        }
    };
    MySqlFileRepository::new(ctx.pool().clone())
        .insert_file(tx, body_file)
        .await?;
    let inserted = emails
        .insert_message(
            tx,
            &NewEmailMessage {
                id: email_id,
                mailbox_id: draft.mailbox.id,
                thread_id,
                direction: EmailDirection::Outbound,
                kind: EmailKind::Message,
                status,
                message_id: message_id.to_owned(),
                in_reply_to: draft.in_reply_to.clone(),
                references: draft.references.clone(),
                report_for_message_id: None,
                provider_message_id: None,
                subject: draft.subject.clone(),
                preview: email_preview(&preview_source),
                body_file_id: Some(body_file.id),
                has_html: draft.body.html.is_some(),
                tracking_enabled: draft.tracking,
                sent_at: None,
                scheduled_at: draft.scheduled_at,
                created_by: Some(sender.clone()),
                participants,
                attachments: draft.attachments.clone(),
            },
            now,
        )
        .await?;
    if !inserted {
        return Err(AppError::Internal(
            "generated Message-ID already exists".to_owned(),
        ));
    }
    let mut linked: Vec<RecordRef> = Vec::new();
    if let Some(record) = &draft.record {
        linked.push(record.clone());
    }
    for contact_id in people.values() {
        let contact = RecordRef::new(RecordType::CONTACT, *contact_id);
        if !linked.contains(&contact) {
            linked.push(contact);
        }
    }
    let links = LinkStore::of(ctx);
    for record in linked {
        links
            .link(
                tx,
                &NewRecordLink {
                    thread_id,
                    record,
                    origin: LinkOrigin::Automatic,
                    created_by: None,
                },
                now,
            )
            .await?;
    }
    emails.refresh_thread(tx, thread_id, now).await?;
    if let Some(run_at) = draft.scheduled_at {
        let actor = Actor::from_scope(scope);
        ctx.jobs()
            .enqueue(
                tx,
                Job::new(
                    SEND_SCHEDULED_JOB,
                    json!({ "email_id": email_id, "requested_by": sender.as_str(), "authority": aphel_kernel::infrastructure::deferred_authority::DeferredAuthority::capture(scope) }),
                )
                .run_at(run_at)
                .with_unique_key(format!("{SEND_SCHEDULED_JOB}:{email_id}")),
            )
            .await?;
        ctx.events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), EMAIL_SCHEDULED, now).with_payload(json!({
                    "email_id": email_id,
                    "thread_id": thread_id,
                    "mailbox_id": draft.mailbox.id,
                    "scheduled_at": run_at,
                    "requested_by": sender.as_str(),
                })),
            )
            .await?;
        ctx.audit()
            .record(
                tx,
                AuditEntry::new(actor, EMAIL_SCHEDULED, now)
                    .with_record(RecordRef::new(EMAIL_MESSAGE_RECORD, email_id))
                    .with_changes(draft_changes(draft, Some(run_at)))
                    .with_ip(ip),
            )
            .await?;
    }
    Ok(())
}

pub(crate) async fn deliver(
    ctx: &Ctx<'_>,
    email_id: Uuid,
    actor: Actor,
    requested_by: Option<UserId>,
    can_retry: bool,
) -> Result<DeliveryOutcome, AppError> {
    let message = MessageStore::of(ctx)
        .find_message(email_id)
        .await?
        .ok_or_else(email_not_found)?;
    let outgoing = match prepare(ctx, &message).await {
        Ok(prepared) => prepared,
        Err(error) => {
            let reason = error.client_message().to_owned();
            record_failure(ctx, &message, &actor, requested_by.as_ref(), &reason).await?;
            return Ok(DeliveryOutcome::Failed(error));
        }
    };
    let (provider, account, outgoing) = outgoing;
    match provider.send(&account, &outgoing).await {
        Ok(sent) => {
            let view = record_success(ctx, &message, &actor, requested_by.as_ref(), sent).await?;
            Ok(DeliveryOutcome::Sent(Box::new(view)))
        }
        Err(error) if can_retry && error.is_retryable() => {
            let now = ctx.now();
            let mut tx = ctx.begin().await?;
            OutboundStore::of(ctx)
                .transition_status(
                    &mut tx,
                    message.id,
                    EmailStatus::Sending,
                    EmailStatus::Scheduled,
                    now,
                )
                .await?;
            tx.commit().await?;
            Ok(DeliveryOutcome::Retry(error.into()))
        }
        Err(error) => {
            record_failure(
                ctx,
                &message,
                &actor,
                requested_by.as_ref(),
                &provider_reason(&error),
            )
            .await?;
            Ok(DeliveryOutcome::Failed(error.into()))
        }
    }
}

async fn prepare(
    ctx: &Ctx<'_>,
    message: &EmailMessageView,
) -> Result<(SharedMailboxProvider, MailboxAccount, ProviderEmail), AppError> {
    let scope = ctx.scope;
    let state = ctx.state();
    let mailbox = MailboxStore::of(ctx)
        .find(message.mailbox_id)
        .await?
        .ok_or_else(mailbox_not_found)?;
    if !mailbox.status.can_send() {
        return Err(AppError::validation_code(
            MAILBOX_UNAVAILABLE,
            "the mailbox is disconnected; reconnect it before sending",
        ));
    }
    let provider = crate::store::mail_providers().require_for(scope, &mailbox.provider)?;
    let credentials = match state
        .secrets
        .reveal(
            scope,
            &SecretKey::new(
                MAILBOX_SECRET_OWNER,
                mailbox.id.to_string(),
                MAILBOX_CREDENTIALS_SECRET,
            ),
        )
        .await?
    {
        Some(secret) => MailboxCredentials::from_secret(&secret)?,
        None => MailboxCredentials::default(),
    };
    let user_id = Uuid::parse_str(mailbox.owner.as_str())
        .map_err(|_| AppError::Internal("mailbox owner id is not a UUID".to_owned()))?;
    let account = MailboxAccount {
        mailbox_id: mailbox.id,
        user_id,
        address: mailbox.address.clone(),
        settings: mailbox.settings.clone(),
        credentials,
        persistence: crate::service::mailboxes::credential_persistence(ctx),
    };
    let body = match message.body_file_id {
        Some(file_id) => read_body(ctx, file_id).await?,
        None => EmailBody::default(),
    };
    let mut html = body.html.clone();
    if message.tracking_enabled
        && let Some(original) = &body.html
    {
        let settings = TrackingStore::of(ctx).settings().await?;
        if settings.any_enabled() {
            let key = tracking_key(ctx).await?;
            let tracked = tracked_html(
                original,
                &settings,
                &state.public_base_url().get(),
                &key,
                message.id,
            )?;
            let now = ctx.now();
            let mut tx = ctx.begin().await?;
            OutboundStore::of(ctx)
                .replace_tracked_links(&mut tx, message.id, &tracked.urls, now)
                .await?;
            tx.commit().await?;
            html = Some(tracked.html);
        }
    }
    let text = body
        .text
        .clone()
        .or_else(|| body.html.as_deref().map(html_to_text))
        .unwrap_or_default();
    let files = MySqlFileRepository::new(ctx.pool().clone());
    let mut attachments = Vec::new();
    for attachment in &message.attachments {
        let file = files.find_file(attachment.file_id).await?.ok_or_else(|| {
            AppError::Validation(format!(
                "the attachment {} is no longer available",
                attachment.name
            ))
        })?;
        attachments.push(EmailAttachmentContent {
            name: attachment.name.clone(),
            mime_type: attachment.mime_type.clone(),
            content: read_file(ctx, &file.storage_key).await?,
            content_id: attachment.content_id.clone(),
            inline: attachment.inline,
        });
    }
    let from = EmailAddress {
        address: mailbox.address.clone(),
        name: mailbox.display_name.clone(),
    };
    let outgoing = ProviderEmail {
        message_id: message.message_id.clone(),
        from,
        to: participants_of(message, ParticipantRole::To),
        cc: participants_of(message, ParticipantRole::Cc),
        bcc: participants_of(message, ParticipantRole::Bcc),
        reply_to: None,
        subject: message.subject.clone(),
        text,
        html: html.map(|html| sanitize_html(&html)),
        in_reply_to: message.in_reply_to.clone(),
        references: message.references.clone(),
        attachments,
        headers: Vec::new(),
    };
    Ok((provider, account, outgoing))
}

async fn record_success(
    ctx: &Ctx<'_>,
    message: &EmailMessageView,
    actor: &Actor,
    requested_by: Option<&UserId>,
    sent: SentEmail,
) -> Result<EmailMessageView, AppError> {
    let scope = ctx.scope;
    let emails = MessageStore::of(ctx);
    let now = ctx.now();
    let returned_id =
        normalize_message_id(&sent.message_id).filter(|returned| returned != &message.message_id);
    let secrets = ctx.secrets()?;
    let mut tx = ctx.begin().await?;
    emails
        .update_message_status(
            &mut tx,
            message.id,
            &MessageStatusChange {
                status: Some(EmailStatus::Sent),
                message_id: returned_id,
                provider_message_id: Some(sent.provider_message_id.clone()),
                sent_at: Some(Some(now)),
                scheduled_at: None,
                failure_reason: Some(None),
            },
            now,
        )
        .await?;
    if let Some(credentials) = &sent.credentials {
        secrets
            .store(
                &mut tx,
                scope,
                &SecretKey::new(
                    MAILBOX_SECRET_OWNER,
                    message.mailbox_id.to_string(),
                    MAILBOX_CREDENTIALS_SECRET,
                ),
                &credentials.to_secret(),
            )
            .await?;
    }
    emails
        .refresh_thread(&mut tx, message.thread_id, now)
        .await?;
    let payload = email_event_payload(
        ctx,
        message,
        json!({
            "requested_by": requested_by.map(UserId::as_str),
            "scheduled": message.scheduled_at.is_some(),
            "tracking": message.tracking_enabled,
        }),
    )
    .await?;
    publish_email_event(ctx, &mut tx, actor, EMAIL_SENT, &payload).await?;
    let mut changes = vec![FieldChange::new(
        "status",
        json!(message.status.as_str()),
        json!(EmailStatus::Sent.as_str()),
    )];
    changes.extend(message_changes(message));
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor.clone(), EMAIL_SENT, now)
                .with_record(email_record(message))
                .with_changes(changes),
        )
        .await?;
    tx.commit().await?;
    emails
        .find_message(message.id)
        .await?
        .ok_or_else(email_not_found)
}

async fn record_failure(
    ctx: &Ctx<'_>,
    message: &EmailMessageView,
    actor: &Actor,
    requested_by: Option<&UserId>,
    reason: &str,
) -> Result<(), AppError> {
    let emails = MessageStore::of(ctx);
    let now = ctx.now();
    let reason = truncate_reason(reason);
    let mut tx = ctx.begin().await?;
    emails
        .update_message_status(
            &mut tx,
            message.id,
            &MessageStatusChange {
                status: Some(EmailStatus::Failed),
                failure_reason: Some(Some(reason.clone())),
                ..MessageStatusChange::default()
            },
            now,
        )
        .await?;
    emails
        .refresh_thread(&mut tx, message.thread_id, now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), EMAIL_FAILED, now).with_payload(json!({
                "email_id": message.id,
                "thread_id": message.thread_id,
                "mailbox_id": message.mailbox_id,
                "requested_by": requested_by.map(UserId::as_str),
                "reason": reason,
            })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor.clone(), EMAIL_FAILED, now)
                .with_record(email_record(message))
                .with_changes(vec![
                    FieldChange::new(
                        "status",
                        json!(message.status.as_str()),
                        json!(EmailStatus::Failed.as_str()),
                    ),
                    FieldChange::new("failure_reason", Value::Null, json!(reason)),
                ]),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn read_body(ctx: &Ctx<'_>, file_id: Uuid) -> Result<EmailBody, AppError> {
    let Some(file) = MySqlFileRepository::new(ctx.pool().clone())
        .find_file(file_id)
        .await?
    else {
        return Ok(EmailBody::default());
    };
    EmailBody::decode(&read_file(ctx, &file.storage_key).await?)
}

async fn read_file(ctx: &Ctx<'_>, storage_key: &str) -> Result<Vec<u8>, AppError> {
    let mut stream = ctx.storage()?.get_stream(storage_key).await?;
    let mut content = Vec::new();
    while let Some(chunk) = stream.next().await {
        content.extend_from_slice(&chunk?);
    }
    Ok(content)
}

fn provider_reason(error: &MailProviderError) -> String {
    format!("{}: {}", error.kind.as_str(), error.message)
}

fn addresses(recipients: &[EmailAddress]) -> Value {
    json!(
        recipients
            .iter()
            .map(|recipient| recipient.address.as_str())
            .collect::<Vec<_>>()
    )
}

fn draft_changes(draft: &Draft, scheduled_at: Option<DateTime<Utc>>) -> Vec<FieldChange> {
    vec![
        FieldChange::new("subject", Value::Null, json!(draft.subject)),
        FieldChange::new("to", Value::Null, addresses(&draft.to)),
        FieldChange::new("cc", Value::Null, addresses(&draft.cc)),
        FieldChange::new("bcc_count", Value::Null, json!(draft.bcc.len())),
        FieldChange::new("attachments", Value::Null, json!(draft.attachments.len())),
        FieldChange::new("tracking", Value::Null, json!(draft.tracking)),
        FieldChange::new("scheduled_at", Value::Null, json!(scheduled_at)),
    ]
}

fn message_changes(message: &EmailMessageView) -> Vec<FieldChange> {
    vec![
        FieldChange::new("subject", Value::Null, json!(message.subject)),
        FieldChange::new(
            "to",
            Value::Null,
            addresses(&participants_of(message, ParticipantRole::To)),
        ),
        FieldChange::new(
            "cc",
            Value::Null,
            addresses(&participants_of(message, ParticipantRole::Cc)),
        ),
        FieldChange::new(
            "bcc_count",
            Value::Null,
            json!(message.participants_with(ParticipantRole::Bcc).count()),
        ),
        FieldChange::new("attachments", Value::Null, json!(message.attachments.len())),
        FieldChange::new("tracking", Value::Null, json!(message.tracking_enabled)),
    ]
}
