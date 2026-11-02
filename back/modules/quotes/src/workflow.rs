use aphel_contract_crm::CrmPermissions;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEAL_QUOTE_ACCEPTED, DEAL_QUOTE_SENT, MAX_ACCEPTANCE_NAME_LENGTH, ManualAcceptQuoteCommand,
    QUOTE_ACCEPTED, QUOTE_APPROVED, QUOTE_DEAL_SYNC_JOB, QUOTE_EXPIRED, QUOTE_EXPIRED_EVENT,
    QUOTE_HAS_NO_ITEMS, QUOTE_INVALID_TRANSITION, QUOTE_LINK_PLACEHOLDER, QUOTE_NOT_CURRENT,
    QUOTE_NOT_SENT, QUOTE_RETURNED_TO_DRAFT, QUOTE_SENT, QUOTE_SUBMITTED, QUOTE_UPDATED, Quote,
    QuoteAcceptance, QuoteDetails, QuoteEmail, QuoteStatus, SendQuoteCommand, SubmitQuoteCommand,
    compute_totals,
};
use crate::gateway::{QuoteApprovals, QuoteGateway};
use crate::service::{
    SystemEvent, event_payload, exceeds, link_url, now, public_url, quote_not_found, quote_record,
    record_deal_event, record_system_event, reload, timezone, today, token_key, visible,
};
use crate::store::QuoteStore;
use crate::token::generate_public_key;
use crate::{AcceptingQuote, KEY, QUOTE, QuoteAccepting};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{Actor, DomainEvent, FieldChange, Job, JobQueue};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::domain::fields::normalize_email;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use approvals::domain::{
    APPROVAL_APPROVED, APPROVAL_CANCELLED, APPROVAL_PENDING, APPROVAL_REJECTED, ApprovalCheckpoint,
    ApprovalMoment, ApprovalStatus, ApprovalSubmission, SubmissionOutcome,
};
use email::EmailAddress;
use email::compose::html::escape_html;

const MAX_COMMENT_LENGTH: usize = 2_000;

fn invalid_transition(message: &str) -> AppError {
    AppError::conflict_code(QUOTE_INVALID_TRANSITION, message.to_owned())
}

fn status_change(from: QuoteStatus, to: QuoteStatus) -> Vec<FieldChange> {
    vec![FieldChange::new(
        "status",
        json!(from.as_str()),
        json!(to.as_str()),
    )]
}

fn outcome_payload(outcome: &SubmissionOutcome) -> Value {
    let (kind, requests) = match outcome {
        SubmissionOutcome::NotRequired => ("not_required", Vec::new()),
        SubmissionOutcome::Pending(requests) => ("pending", requests.clone()),
        SubmissionOutcome::Approved(requests) => ("approved", requests.clone()),
    };
    json!({
        "approval": kind,
        "approval_request_ids": requests
            .iter()
            .map(|request| request.id.to_string())
            .collect::<Vec<_>>(),
    })
}

fn email_body(body: Option<String>, link: &str) -> String {
    let anchor = format!(
        "<a href=\"{}\">{}</a>",
        escape_html(link),
        escape_html(link)
    );
    match body.filter(|body| !body.trim().is_empty()) {
        Some(body) if body.contains(QUOTE_LINK_PLACEHOLDER) => {
            body.replace(QUOTE_LINK_PLACEHOLDER, &anchor)
        }
        Some(body) => format!("{body}<p>{anchor}</p>"),
        None => format!("<p>Segue a cotação para sua análise:</p><p>{anchor}</p>"),
    }
}

pub(crate) async fn submit(
    ctx: &Ctx<'_>,
    quote_id: &str,
    command: SubmitQuoteCommand,
) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    let quote = visible(ctx, &Permission::QUOTES_UPDATE, quote_id).await?;
    ensure_current(&quote)?;
    if !quote.status.can_submit() {
        return Err(invalid_transition(
            "only a draft quote can be sent for approval",
        ));
    }
    ensure_items(ctx, &quote).await?;
    let comment = command
        .comment
        .map(|comment| comment.trim().chars().take(MAX_COMMENT_LENGTH).collect())
        .filter(|comment: &String| !comment.is_empty());
    let discount_flag = discount_flag(ctx, &quote).await?;
    let record = quote_record(quote.id);
    let mut tx = ctx.begin().await?;
    let mut locked = QuoteStore::of(ctx)
        .lock(&mut tx, quote.id)
        .await?
        .ok_or_else(quote_not_found)?;
    if locked.status != QuoteStatus::Draft || !locked.is_current {
        return Err(invalid_transition(
            "only a draft quote can be sent for approval",
        ));
    }
    if let Some(exceeds) = discount_flag
        && exceeds != locked.exceeds_discount_limit
    {
        store_discount_flag(ctx, &mut tx, &mut locked, exceeds).await?;
    }
    let outcome = QuoteApprovals::of(ctx)
        .submit(
            &mut tx,
            scope,
            &ApprovalSubmission {
                checkpoint: ApprovalCheckpoint::quote_send(record.clone()),
                comment,
                ip: command.ip.clone(),
            },
        )
        .await?;
    let next = match outcome {
        SubmissionOutcome::Pending(_) => QuoteStatus::PendingApproval,
        SubmissionOutcome::NotRequired | SubmissionOutcome::Approved(_) => QuoteStatus::Approved,
    };
    let now = now(ctx);
    locked.status = next;
    locked.updated_at = now;
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: QUOTE_SUBMITTED,
            payload: event_payload(&locked, outcome_payload(&outcome)),
            changes: status_change(QuoteStatus::Draft, next),
            ip: command.ip.clone(),
        },
    )
    .await?;
    if next == QuoteStatus::Approved {
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            scope,
            now,
            RecordMutation {
                record,
                event_type: QUOTE_APPROVED,
                payload: event_payload(&locked, outcome_payload(&outcome)),
                changes: Vec::new(),
                ip: command.ip,
            },
        )
        .await?;
    }
    tx.commit().await?;
    reload(ctx, quote.id).await
}

pub(crate) async fn send(
    ctx: &Ctx<'_>,
    quote_id: &str,
    command: SendQuoteCommand,
) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    let quote = visible(ctx, &Permission::QUOTES_SEND, quote_id).await?;
    ensure_current(&quote)?;
    if !quote.status.can_send() {
        return Err(match quote.status {
            QuoteStatus::PendingApproval => AppError::conflict_code(
                APPROVAL_PENDING,
                "the quote is waiting for approval and cannot be sent yet",
            ),
            _ => invalid_transition("this quote can no longer be sent"),
        });
    }
    ensure_items(ctx, &quote).await?;
    if quote.is_expired_on(today(ctx).await?) {
        return Err(AppError::conflict_code(
            QUOTE_EXPIRED,
            "the quote validity date has passed; change it before sending",
        ));
    }
    let mut to = command.to;
    if to.is_empty()
        && let Some(contact_id) = quote.contact_id
        && let Some(address) = QuoteGateway::of(ctx)
            .contact_email(scope, contact_id)
            .await?
    {
        to.push(address);
    }
    if to.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid quote e-mail",
            vec![FieldViolation::new("to", "add at least one recipient")],
        ));
    }
    let discount_flag = discount_flag(ctx, &quote).await?;
    let mut tx = ctx.begin().await?;
    let key = token_key(ctx, &mut tx).await?;
    let public_key = match &quote.public_key {
        Some(public_key) => public_key.clone(),
        None => generate_public_key()?,
    };
    let link = link_url(ctx, &public_key, &key)?;
    let subject = command
        .subject
        .map(|subject| subject.trim().to_owned())
        .filter(|subject| !subject.is_empty())
        .unwrap_or_else(|| format!("Cotação nº {} — {}", quote.number, quote.title));
    let email = QuoteEmail {
        deal_id: quote.deal_id,
        mailbox_id: command.mailbox_id,
        to,
        cc: command.cc,
        subject,
        body_html: email_body(command.body_html, &link),
        ip: command.ip.clone(),
    };
    let recipients: Vec<String> = email
        .to
        .iter()
        .map(|address: &EmailAddress| address.address.clone())
        .collect();
    let record = quote_record(quote.id);
    let mut locked = QuoteStore::of(ctx)
        .lock(&mut tx, quote.id)
        .await?
        .ok_or_else(quote_not_found)?;
    if !locked.is_current || !locked.status.can_send() {
        return Err(invalid_transition("this quote can no longer be sent"));
    }
    if let Some(exceeds) = discount_flag
        && exceeds != locked.exceeds_discount_limit
    {
        store_discount_flag(ctx, &mut tx, &mut locked, exceeds).await?;
    }
    let previous = locked.status;
    if previous != QuoteStatus::Sent {
        QuoteApprovals::of(ctx)
            .checkpoint(
                &mut tx,
                scope,
                &ApprovalCheckpoint::quote_send(record.clone()),
            )
            .await?;
    }
    let now = now(ctx);
    locked.status = QuoteStatus::Sent;
    locked.public_key = Some(public_key);
    locked.sent_at = Some(now);
    locked.sent_by = Some(scope.user_id().clone());
    locked.updated_at = now;
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    let extra = json!({ "recipients": recipients, "resent": previous == QuoteStatus::Sent });
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record,
            event_type: QUOTE_SENT,
            payload: event_payload(&locked, extra.clone()),
            changes: status_change(previous, QuoteStatus::Sent),
            ip: command.ip,
        },
    )
    .await?;
    record_deal_event(
        ctx,
        &mut tx,
        Actor::from_scope(scope),
        &locked,
        DEAL_QUOTE_SENT,
        extra,
    )
    .await?;
    QuoteGateway::of(ctx)
        .send_email(&mut tx, scope, email)
        .await?;
    tx.commit().await?;
    reload(ctx, quote.id).await
}

fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) async fn accept(
    ctx: &Ctx<'_>,
    quote_id: &str,
    command: ManualAcceptQuoteCommand,
) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    let quote = visible(ctx, &Permission::QUOTES_UPDATE, quote_id).await?;
    ensure_current(&quote)?;
    if !quote.status.can_send() {
        return Err(match quote.status {
            QuoteStatus::PendingApproval => AppError::conflict_code(
                APPROVAL_PENDING,
                "the quote is waiting for approval and cannot be accepted yet",
            ),
            _ => invalid_transition("this quote can no longer be accepted"),
        });
    }
    ensure_items(ctx, &quote).await?;
    if quote.is_expired_on(today(ctx).await?) {
        return Err(AppError::conflict_code(
            QUOTE_EXPIRED,
            "the quote validity date has passed; change it before accepting",
        ));
    }
    let mut violations = Vec::new();
    let name = match optional_text(command.name) {
        Some(name) => Some(name),
        None => QuoteStore::of(ctx).people(&quote).await?.contact_name,
    };
    let name = name.unwrap_or_default();
    if name.is_empty() || name.chars().count() > MAX_ACCEPTANCE_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_ACCEPTANCE_NAME_LENGTH} characters"),
        ));
    }
    let email = match optional_text(command.email) {
        Some(email) => match normalize_email(&email) {
            Ok(email) => email,
            Err(error) => {
                violations.push(FieldViolation::new("email", error.message()));
                String::new()
            }
        },
        None => match quote.contact_id {
            Some(contact_id) => QuoteGateway::of(ctx)
                .contact_email(scope, contact_id)
                .await?
                .map(|address| address.address)
                .unwrap_or_default(),
            None => String::new(),
        },
    };
    let note = optional_text(command.note);
    if note
        .as_ref()
        .is_some_and(|note| note.chars().count() > MAX_COMMENT_LENGTH)
    {
        violations.push(FieldViolation::new(
            "notes.note",
            format!("must have at most {MAX_COMMENT_LENGTH} characters"),
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid acceptance", violations));
    }
    let record = quote_record(quote.id);
    let mut tx = ctx.begin().await?;
    let mut locked = QuoteStore::of(ctx)
        .lock(&mut tx, quote.id)
        .await?
        .ok_or_else(quote_not_found)?;
    if !locked.is_current || !locked.status.can_send() {
        return Err(invalid_transition("this quote can no longer be accepted"));
    }
    let previous = locked.status;
    if previous != QuoteStatus::Sent {
        QuoteApprovals::of(ctx)
            .checkpoint(
                &mut tx,
                scope,
                &ApprovalCheckpoint::quote_send(record.clone()),
            )
            .await?;
    }
    let now = now(ctx);
    let acceptance = QuoteAcceptance {
        name: name.clone(),
        email: email.clone(),
        ip: command.ip.clone(),
        user_agent: None,
        recorded_by: Some(scope.user_id().clone()),
    };
    locked.status = QuoteStatus::Accepted;
    locked.accepted_at = Some(now);
    locked.acceptance = Some(acceptance.clone());
    locked.updated_at = now;
    ctx.run_hook(
        QuoteAccepting,
        &mut tx,
        KEY,
        &AcceptingQuote {
            quote: locked.clone(),
            acceptance,
        },
    )
    .await?;
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    let extra = json!({
        "accepted_by_name": name,
        "accepted_by_email": (!email.is_empty()).then_some(email),
        "manual": true,
        "recorded_by": scope.user_id().as_str(),
        "note": note,
        "requested_by": locked.owner_id.as_ref().map(|id| id.as_str()),
    });
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record,
            event_type: QUOTE_ACCEPTED,
            payload: event_payload(&locked, extra.clone()),
            changes: status_change(previous, QuoteStatus::Accepted),
            ip: command.ip,
        },
    )
    .await?;
    record_deal_event(
        ctx,
        &mut tx,
        Actor::from_scope(scope),
        &locked,
        DEAL_QUOTE_ACCEPTED,
        extra,
    )
    .await?;
    ctx.jobs()
        .enqueue(
            &mut tx,
            Job::new(
                QUOTE_DEAL_SYNC_JOB,
                json!({
                    "quote_id": locked.id.to_string(),
                }),
            )
            .with_unique_key(format!("quote-deal-sync:{}", locked.id)),
        )
        .await?;
    tx.commit().await?;
    reload(ctx, quote.id).await
}

pub(crate) async fn public_link(ctx: &Ctx<'_>, quote_id: &str) -> Result<String, AppError> {
    let quote = visible(ctx, &Permission::QUOTES_READ, quote_id).await?;
    public_url(ctx, &quote)
        .await?
        .ok_or_else(|| AppError::conflict_code(QUOTE_NOT_SENT, "the quote has not been sent yet"))
}

pub(crate) async fn apply_approval(ctx: &Ctx<'_>, event: &DomainEvent) -> Result<(), AppError> {
    let Some(record) = &event.record else {
        return Ok(());
    };
    if record.record_type != QUOTE {
        return Ok(());
    }
    let moment = event.payload.get("moment").and_then(Value::as_str);
    if moment.is_some_and(|moment| moment != ApprovalMoment::QuoteSend.as_str()) {
        return Ok(());
    }
    let scope = AccessScope::system();
    let Some(quote) = QuoteStore::of(ctx)
        .find(
            &scope,
            &aphel_kernel::domain::access::RecordScopeFilter::Organization,
            record.record_id,
        )
        .await?
    else {
        return Ok(());
    };
    if quote.status != QuoteStatus::PendingApproval || !quote.is_current {
        return Ok(());
    }
    let requests: Vec<_> = QuoteApprovals::of(ctx)
        .requests_for(&scope, record)
        .await?
        .into_iter()
        .filter(|request| request.moment == ApprovalMoment::QuoteSend)
        .collect();
    let pending = requests
        .iter()
        .any(|request| request.status == ApprovalStatus::Pending);
    let rejected = requests
        .iter()
        .any(|request| request.status == ApprovalStatus::Rejected);
    let next = match event.event_type.as_str() {
        APPROVAL_APPROVED if !pending && !rejected => QuoteStatus::Approved,
        APPROVAL_REJECTED => QuoteStatus::Draft,
        APPROVAL_CANCELLED if !pending => QuoteStatus::Draft,
        _ => return Ok(()),
    };
    let mut tx = ctx.begin().await?;
    let Some(mut locked) = QuoteStore::of(ctx).lock(&mut tx, quote.id).await? else {
        return Ok(());
    };
    if locked.status != QuoteStatus::PendingApproval {
        return Ok(());
    }
    if event.event_type == APPROVAL_REJECTED {
        QuoteApprovals::of(ctx)
            .cancel_open(&mut tx, &scope, record, "another approval was rejected")
            .await?;
    }
    locked.status = next;
    locked.updated_at = now(ctx);
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    let event_type = if next == QuoteStatus::Approved {
        QUOTE_APPROVED
    } else {
        QUOTE_RETURNED_TO_DRAFT
    };
    let payload = &event.payload;
    let field = |key: &str| payload.get(key).cloned().unwrap_or(Value::Null);
    let extra = json!({
        "approval_event": event.event_type,
        "approval_request_id": field("request_id"),
        "requested_by": field("requested_by"),
        "comment": field("resolution_comment"),
    });
    record_system_event(
        ctx,
        &mut tx,
        &locked,
        SystemEvent {
            event_type,
            extra,
            changes: status_change(QuoteStatus::PendingApproval, next),
            ip: None,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn expire_due(ctx: &Ctx<'_>) -> Result<u32, AppError> {
    let scope = AccessScope::system();
    let organization = QuoteStore::of(ctx).organization().await?;
    let today = now(ctx)
        .with_timezone(&timezone(&organization.timezone))
        .date_naive();
    let mut expired = 0;
    for id in QuoteStore::of(ctx).expiring(today).await? {
        if expire(&ctx.with_scope(&scope), id, today).await? {
            expired += 1;
        }
    }
    Ok(expired)
}

pub(crate) async fn expire(
    ctx: &Ctx<'_>,
    id: Uuid,
    today: chrono::NaiveDate,
) -> Result<bool, AppError> {
    let mut tx = ctx.begin().await?;
    let Some(mut locked) = QuoteStore::of(ctx).lock(&mut tx, id).await? else {
        return Ok(false);
    };
    if locked.status != QuoteStatus::Sent || !locked.is_expired_on(today) {
        return Ok(false);
    }
    let now = now(ctx);
    locked.status = QuoteStatus::Expired;
    locked.expired_at = Some(now);
    locked.updated_at = now;
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    record_system_event(
        ctx,
        &mut tx,
        &locked,
        SystemEvent {
            event_type: QUOTE_EXPIRED_EVENT,
            extra: json!({ "valid_until": locked.valid_until.map(|date| date.to_string()) }),
            changes: status_change(QuoteStatus::Sent, QuoteStatus::Expired),
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(true)
}

pub(crate) fn ensure_current(quote: &Quote) -> Result<(), AppError> {
    if quote.is_current {
        Ok(())
    } else {
        Err(AppError::conflict_code(
            QUOTE_NOT_CURRENT,
            "only the current version of a quote can be used",
        ))
    }
}

pub(crate) async fn ensure_items(ctx: &Ctx<'_>, quote: &Quote) -> Result<(), AppError> {
    if QuoteStore::of(ctx).items(quote.id).await?.is_empty() {
        return Err(AppError::validation_code(
            QUOTE_HAS_NO_ITEMS,
            "add at least one item to the quote",
        ));
    }
    Ok(())
}

pub(crate) async fn discount_flag(ctx: &Ctx<'_>, quote: &Quote) -> Result<Option<bool>, AppError> {
    let items = QuoteStore::of(ctx).items(quote.id).await?;
    let Ok(totals) = compute_totals(
        &items
            .iter()
            .map(|item| item.data.clone())
            .collect::<Vec<_>>(),
        quote.discount,
    ) else {
        return Ok(None);
    };
    Ok(Some(exceeds(ctx, quote, &totals).await?))
}

pub(crate) async fn store_discount_flag(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    locked: &mut Quote,
    exceeds: bool,
) -> Result<(), AppError> {
    locked.exceeds_discount_limit = exceeds;
    QuoteStore::of(ctx).update(tx, locked).await?;
    let changes = vec![FieldChange::new(
        "exceeds_discount_limit",
        json!(!exceeds),
        json!(exceeds),
    )];
    record_system_event(
        ctx,
        tx,
        locked,
        SystemEvent {
            event_type: QUOTE_UPDATED,
            extra: aphel_kernel::records::changes_payload(&changes),
            changes,
            ip: None,
        },
    )
    .await
}
