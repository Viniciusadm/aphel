use serde_json::json;

use crate::domain::{
    AcceptQuoteCommand, DEAL_QUOTE_ACCEPTED, DEAL_QUOTE_REJECTED, MAX_ACCEPTANCE_NAME_LENGTH,
    MAX_REJECTION_REASON_LENGTH, PublicQuote, PublicQuoteState, QUOTE_ACCEPTED, QUOTE_CLOSED,
    QUOTE_DEAL_SYNC_JOB, QUOTE_EXPIRED, QUOTE_REJECTED, QUOTE_SUPERSEDED, QUOTE_VIEWED, Quote,
    QuoteAcceptance, QuoteStatus, RejectQuoteCommand, compute_totals,
};
use crate::service::{
    SystemEvent, any, existing_token_key, now, quote_not_found, record_deal_event,
    record_system_event, timezone,
};
use crate::store::QuoteStore;
use crate::token::SignedLinkToken;
use crate::{AcceptingQuote, KEY, QuoteAccepting};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{Actor, FieldChange, Job, JobQueue};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::normalize_email;
use aphel_kernel::errors::{AppError, FieldViolation};

const MAX_USER_AGENT_LENGTH: usize = 512;

fn state(quote: &Quote, today: chrono::NaiveDate) -> PublicQuoteState {
    if !quote.is_current {
        return PublicQuoteState::Superseded;
    }
    match quote.status {
        QuoteStatus::Accepted => PublicQuoteState::Accepted,
        QuoteStatus::Rejected => PublicQuoteState::Rejected,
        QuoteStatus::Expired => PublicQuoteState::Expired,
        QuoteStatus::Sent if quote.is_expired_on(today) => PublicQuoteState::Expired,
        QuoteStatus::Sent => PublicQuoteState::Open,
        _ => PublicQuoteState::Superseded,
    }
}

fn closed(state: PublicQuoteState) -> AppError {
    match state {
        PublicQuoteState::Superseded => AppError::conflict_code(
            QUOTE_SUPERSEDED,
            "this quote was replaced by a newer version",
        ),
        PublicQuoteState::Expired => {
            AppError::conflict_code(QUOTE_EXPIRED, "this quote is no longer valid")
        }
        _ => AppError::conflict_code(QUOTE_CLOSED, "this quote was already answered"),
    }
}

fn status_change(to: QuoteStatus) -> Vec<FieldChange> {
    vec![FieldChange::new(
        "status",
        json!(QuoteStatus::Sent.as_str()),
        json!(to.as_str()),
    )]
}

pub(crate) async fn resolve_token(
    ctx: &Ctx<'_>,
    token: &str,
) -> Result<(AccessScope, Quote), AppError> {
    let signed = SignedLinkToken::parse(token).ok_or_else(quote_not_found)?;
    let quote_id = QuoteStore::of(ctx)
        .by_public_key(&signed.public_key)
        .await?
        .ok_or_else(quote_not_found)?;
    let scope = AccessScope::system();
    let key = existing_token_key(&ctx.with_scope(&scope))
        .await?
        .ok_or_else(quote_not_found)?;
    if !signed.verify(&key) {
        return Err(quote_not_found());
    }
    let quote = any(&ctx.with_scope(&scope), quote_id).await?;
    Ok((scope, quote))
}

pub(crate) async fn public_today(ctx: &Ctx<'_>) -> Result<chrono::NaiveDate, AppError> {
    let organization = QuoteStore::of(ctx).organization().await?;
    Ok(now(ctx)
        .with_timezone(&timezone(&organization.timezone))
        .date_naive())
}

pub(crate) async fn public_quote(
    ctx: &Ctx<'_>,
    quote: Quote,
    state: PublicQuoteState,
) -> Result<PublicQuote, AppError> {
    let organization = QuoteStore::of(ctx).organization().await?;
    let items = QuoteStore::of(ctx).items(quote.id).await?;
    let people = QuoteStore::of(ctx).people(&quote).await?;
    let totals = compute_totals(
        &items
            .iter()
            .map(|item| item.data.clone())
            .collect::<Vec<_>>(),
        quote.discount,
    )
    .unwrap_or_default();
    Ok(PublicQuote {
        organization,
        items,
        items_discount: totals.items_discount,
        total_discount: totals.total_discount,
        state,
        owner_name: people.owner_name,
        owner_email: people.owner_email,
        contact_name: people.contact_name,
        company_name: people.company_name,
        quote,
    })
}

pub(crate) async fn public_view(ctx: &Ctx<'_>, token: &str) -> Result<PublicQuote, AppError> {
    let (scope, mut quote) = resolve_token(ctx, token).await?;
    let today = public_today(&ctx.with_scope(&scope)).await?;
    let state = state(&quote, today);
    if state == PublicQuoteState::Open && quote.viewed_at.is_none() {
        let mut tx = ctx.begin().await?;
        if let Some(mut locked) = QuoteStore::of(ctx).lock(&mut tx, quote.id).await?
            && locked.viewed_at.is_none()
        {
            locked.viewed_at = Some(now(ctx));
            QuoteStore::of(ctx).update(&mut tx, &locked).await?;
            record_system_event(
                ctx,
                &mut tx,
                &locked,
                SystemEvent {
                    event_type: QUOTE_VIEWED,
                    extra: json!({}),
                    changes: Vec::new(),
                    ip: None,
                },
            )
            .await?;
            tx.commit().await?;
            quote = locked;
        }
    }
    public_quote(&ctx.with_scope(&scope), quote, state).await
}

pub(crate) async fn public_accept(
    ctx: &Ctx<'_>,
    token: &str,
    command: AcceptQuoteCommand,
) -> Result<PublicQuote, AppError> {
    let mut violations = Vec::new();
    let name = command.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_ACCEPTANCE_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_ACCEPTANCE_NAME_LENGTH} characters"),
        ));
    }
    let email = match normalize_email(command.email.trim()) {
        Ok(email) => email,
        Err(error) => {
            violations.push(FieldViolation::new("email", error.message()));
            String::new()
        }
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid acceptance", violations));
    }
    let (scope, quote) = resolve_token(ctx, token).await?;
    let today = public_today(&ctx.with_scope(&scope)).await?;
    let current = state(&quote, today);
    if current != PublicQuoteState::Open {
        return Err(closed(current));
    }
    let mut tx = ctx.begin().await?;
    let mut locked = QuoteStore::of(ctx)
        .lock(&mut tx, quote.id)
        .await?
        .ok_or_else(quote_not_found)?;
    let current = state(&locked, today);
    if current != PublicQuoteState::Open {
        return Err(closed(current));
    }
    let now = now(ctx);
    locked.status = QuoteStatus::Accepted;
    locked.accepted_at = Some(now);
    locked.acceptance = Some(QuoteAcceptance {
        name: name.clone(),
        email: email.clone(),
        ip: command.ip.clone(),
        user_agent: command
            .user_agent
            .map(|agent| agent.chars().take(MAX_USER_AGENT_LENGTH).collect()),
        recorded_by: None,
    });
    locked.updated_at = now;
    if let Some(acceptance) = locked.acceptance.clone() {
        ctx.run_hook::<QuoteAccepting>(
            &mut tx,
            KEY,
            &AcceptingQuote {
                quote: locked.clone(),
                acceptance,
            },
        )
        .await?;
    }
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    let extra = json!({
        "accepted_by_name": name,
        "accepted_by_email": email,
        "requested_by": locked.owner_id.as_ref().map(UserId::as_str),
    });
    record_system_event(
        ctx,
        &mut tx,
        &locked,
        SystemEvent {
            event_type: QUOTE_ACCEPTED,
            extra: extra.clone(),
            changes: status_change(QuoteStatus::Accepted),
            ip: command.ip,
        },
    )
    .await?;
    record_deal_event(
        ctx,
        &mut tx,
        Actor::System,
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
    public_quote(&ctx.with_scope(&scope), locked, PublicQuoteState::Accepted).await
}

pub(crate) async fn public_reject(
    ctx: &Ctx<'_>,
    token: &str,
    command: RejectQuoteCommand,
) -> Result<PublicQuote, AppError> {
    let reason = command
        .reason
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty());
    if reason
        .as_ref()
        .is_some_and(|reason| reason.chars().count() > MAX_REJECTION_REASON_LENGTH)
    {
        return Err(AppError::invalid_fields(
            "invalid rejection",
            vec![FieldViolation::new(
                "reason",
                format!("must have at most {MAX_REJECTION_REASON_LENGTH} characters"),
            )],
        ));
    }
    let (scope, quote) = resolve_token(ctx, token).await?;
    let today = public_today(&ctx.with_scope(&scope)).await?;
    let current = state(&quote, today);
    if current != PublicQuoteState::Open {
        return Err(closed(current));
    }
    let mut tx = ctx.begin().await?;
    let mut locked = QuoteStore::of(ctx)
        .lock(&mut tx, quote.id)
        .await?
        .ok_or_else(quote_not_found)?;
    let current = state(&locked, today);
    if current != PublicQuoteState::Open {
        return Err(closed(current));
    }
    let now = now(ctx);
    locked.status = QuoteStatus::Rejected;
    locked.rejected_at = Some(now);
    locked.rejection_reason = reason.clone();
    locked.updated_at = now;
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    let extra = json!({
        "reason": reason,
        "requested_by": locked.owner_id.as_ref().map(UserId::as_str),
    });
    record_system_event(
        ctx,
        &mut tx,
        &locked,
        SystemEvent {
            event_type: QUOTE_REJECTED,
            extra: extra.clone(),
            changes: status_change(QuoteStatus::Rejected),
            ip: command.ip,
        },
    )
    .await?;
    record_deal_event(
        ctx,
        &mut tx,
        Actor::System,
        &locked,
        DEAL_QUOTE_REJECTED,
        extra,
    )
    .await?;
    tx.commit().await?;
    public_quote(&ctx.with_scope(&scope), locked, PublicQuoteState::Rejected).await
}
