use aphel_contract_crm::CrmPermissions;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::compose::events::{email_event_payload, email_record, publish_email_event};
use crate::domain::{EmailDirection, EmailKind, EmailStatus, ParticipantRole, normalize_address};
use crate::store::sending::{OutboundStore, TrackingStore};
use crate::store::{MailboxStore, MessageStore};
use crate::tracking::{
    EMAIL_TRACKING_OBJECTION_ADDED, EMAIL_TRACKING_OBJECTION_EXISTS,
    EMAIL_TRACKING_OBJECTION_REMOVED, EMAIL_TRACKING_UPDATED, MAX_OBJECTION_REASON_LENGTH,
    NewTrackingObjection, SignedToken, TrackingKind, TrackingObjection, TrackingSettings,
    address_hash, decode_key, generate_key, link_not_found, objection_not_found,
    tracking_key_reference, tracking_scope,
};
use crate::{EMAIL_BOUNCED, EMAIL_CLICKED, EMAIL_OPENED, EmailMessageView, EngagementKind};
use aphel_kernel::Ctx;
use aphel_kernel::domain::{Secret, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::secrets::SecretVault;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::privacy::mask_email;

pub(crate) async fn existing_tracking_key(ctx: &Ctx<'_>) -> Result<Option<Vec<u8>>, AppError> {
    Ok(ctx
        .state()
        .secrets
        .reveal(ctx.scope, &tracking_key_reference())
        .await?
        .and_then(|secret| decode_key(secret.expose())))
}

pub(crate) async fn tracking_key(ctx: &Ctx<'_>) -> Result<Vec<u8>, AppError> {
    if let Some(key) = existing_tracking_key(ctx).await? {
        return Ok(key);
    }
    let generated = generate_key()?;
    let secrets = ctx.secrets()?;
    let mut tx = ctx.begin().await?;
    secrets
        .store(
            &mut tx,
            ctx.scope,
            &tracking_key_reference(),
            &Secret::new(generated),
        )
        .await?;
    tx.commit().await?;
    existing_tracking_key(ctx)
        .await?
        .ok_or_else(|| AppError::Internal("e-mail tracking key was not stored".to_owned()))
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<TrackingSettings, AppError> {
    if ctx.scope.require(&Permission::SETTINGS_MANAGE).is_err() {
        ctx.scope.require(&Permission::EMAILS_SEND)?;
    }
    TrackingStore::of(ctx).settings().await
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    settings: TrackingSettings,
    ip: Option<String>,
) -> Result<TrackingSettings, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let tracking = TrackingStore::of(ctx);
    let current = tracking.settings().await?;
    if settings.any_enabled() {
        tracking_key(ctx).await?;
    }
    if current == settings {
        return Ok(current);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let changes: Vec<FieldChange> = [
        (
            "opens_enabled",
            current.opens_enabled,
            settings.opens_enabled,
        ),
        (
            "clicks_enabled",
            current.clicks_enabled,
            settings.clicks_enabled,
        ),
        ("default_on", current.default_on, settings.default_on),
    ]
    .into_iter()
    .filter(|(_, old, new)| old != new)
    .map(|(field, old, new)| FieldChange::new(field, json!(old), json!(new)))
    .collect();
    let mut tx = ctx.begin().await?;
    tracking
        .save_settings(&mut tx, &settings, scope.user_id(), now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), EMAIL_TRACKING_UPDATED, now).with_payload(json!({
                "opens_enabled": settings.opens_enabled,
                "clicks_enabled": settings.clicks_enabled,
                "default_on": settings.default_on,
            })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, EMAIL_TRACKING_UPDATED, now)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    tracking.settings().await
}

pub(crate) async fn objections(ctx: &Ctx<'_>) -> Result<Vec<TrackingObjection>, AppError> {
    ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
    TrackingStore::of(ctx).objections().await
}

pub(crate) async fn add_objection(
    ctx: &Ctx<'_>,
    address: &str,
    reason: Option<String>,
    ip: Option<String>,
) -> Result<TrackingObjection, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::PRIVACY_MANAGE)?;
    let mut violations = Vec::new();
    let address = normalize_address(address)
        .map_err(|message| violations.push(FieldViolation::new("address", message)))
        .ok();
    let reason = reason
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty());
    if reason
        .as_ref()
        .is_some_and(|reason| reason.chars().count() > MAX_OBJECTION_REASON_LENGTH)
    {
        violations.push(FieldViolation::new(
            "reason",
            format!("must have at most {MAX_OBJECTION_REASON_LENGTH} characters"),
        ));
    }
    let (Some(address), true) = (address, violations.is_empty()) else {
        return Err(AppError::invalid_fields("invalid objection", violations));
    };
    let objection = NewTrackingObjection {
        id: Uuid::now_v7(),
        address_hash: address_hash(&address),
        label: mask_email(&address),
        reason,
        created_by: Some(scope.user_id().clone()),
    };
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let tracking = TrackingStore::of(ctx);
    let mut tx = ctx.begin().await?;
    if !tracking.insert_objection(&mut tx, &objection, now).await? {
        return Err(AppError::conflict_code(
            EMAIL_TRACKING_OBJECTION_EXISTS,
            "this address already objected to e-mail tracking",
        ));
    }
    publish_objection(
        ctx,
        &mut tx,
        &actor,
        EMAIL_TRACKING_OBJECTION_ADDED,
        objection.id,
        &objection.label,
        ip,
    )
    .await?;
    tx.commit().await?;
    tracking
        .find_objection(objection.id)
        .await?
        .ok_or_else(objection_not_found)
}

pub(crate) async fn remove_objection(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::PRIVACY_MANAGE)?;
    let id = Uuid::parse_str(id.trim()).map_err(|_| objection_not_found())?;
    let tracking = TrackingStore::of(ctx);
    let objection = tracking
        .find_objection(id)
        .await?
        .ok_or_else(objection_not_found)?;
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    if !tracking.delete_objection(&mut tx, id).await? {
        return Err(objection_not_found());
    }
    publish_objection(
        ctx,
        &mut tx,
        &actor,
        EMAIL_TRACKING_OBJECTION_REMOVED,
        objection.id,
        &objection.label,
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn record_open(
    ctx: &Ctx<'_>,
    token: &str,
    viewer: Option<&UserId>,
) -> Result<(), AppError> {
    let scope = tracking_scope();
    let ctx = ctx.with_scope(&scope);
    let Some((signed, message)) = resolve(&ctx, token).await? else {
        return Ok(());
    };
    if !matches!(signed.token.kind, TrackingKind::Open) {
        return Ok(());
    }
    let settings = TrackingStore::of(&ctx).settings().await?;
    if !settings.opens_enabled || !should_record(&ctx, &message, viewer).await? {
        return Ok(());
    }
    engage(&ctx, &message, EngagementKind::Opened, json!({})).await
}

pub(crate) async fn record_click(
    ctx: &Ctx<'_>,
    token: &str,
    viewer: Option<&UserId>,
) -> Result<String, AppError> {
    let scope = tracking_scope();
    let ctx = ctx.with_scope(&scope);
    let Some((signed, message)) = resolve(&ctx, token).await? else {
        return Err(link_not_found());
    };
    let TrackingKind::Click(position) = signed.token.kind else {
        return Err(link_not_found());
    };
    let url = OutboundStore::of(&ctx)
        .tracked_link(message.id, u32::from(position))
        .await?
        .ok_or_else(link_not_found)?;
    let settings = TrackingStore::of(&ctx).settings().await?;
    if settings.clicks_enabled
        && should_record(&ctx, &message, viewer).await?
        && let Err(error) = engage(
            &ctx,
            &message,
            EngagementKind::Clicked,
            json!({ "link_position": position }),
        )
        .await
    {
        tracing::warn!(%error, "failed to record an e-mail click");
    }
    Ok(url)
}

pub(crate) async fn process_received(ctx: &Ctx<'_>, email_id: Uuid) -> Result<(), AppError> {
    let scope = tracking_scope();
    let ctx = ctx.with_scope(&scope);
    let emails = MessageStore::of(&ctx);
    let Some(received) = emails.find_message(email_id).await? else {
        return Ok(());
    };
    if received.direction != EmailDirection::Inbound {
        return Ok(());
    }
    let mut candidates: Vec<String> = Vec::new();
    if received.kind == EmailKind::Bounce
        && let Some(reported) = &received.report_for_message_id
    {
        candidates.push(reported.clone());
    }
    if let Some(in_reply_to) = &received.in_reply_to {
        candidates.push(in_reply_to.clone());
    }
    candidates.extend(received.references.iter().rev().cloned());
    let mut seen = Vec::new();
    for candidate in candidates {
        if seen.contains(&candidate) {
            continue;
        }
        seen.push(candidate.clone());
        let Some(original) = emails.find_message_by_header(&candidate).await? else {
            continue;
        };
        if original.direction != EmailDirection::Outbound
            || original.mailbox_id != received.mailbox_id
            || original.status != EmailStatus::Sent
        {
            continue;
        }
        return match received.kind {
            EmailKind::Bounce => {
                engage(
                    &ctx,
                    &original,
                    EngagementKind::Bounced,
                    json!({ "bounce_email_id": received.id }),
                )
                .await
            }
            EmailKind::Message => {
                engage(
                    &ctx,
                    &original,
                    EngagementKind::Replied,
                    json!({ "reply_email_id": received.id }),
                )
                .await
            }
            EmailKind::AutoReply => Ok(()),
        };
    }
    Ok(())
}

async fn resolve(
    ctx: &Ctx<'_>,
    token: &str,
) -> Result<Option<(SignedToken, EmailMessageView)>, AppError> {
    let Some(signed) = SignedToken::parse(token) else {
        return Ok(None);
    };
    let Some(key) = existing_tracking_key(ctx).await? else {
        return Ok(None);
    };
    if !signed.verify(&key) {
        return Ok(None);
    }
    let Some(message) = MessageStore::of(ctx)
        .find_message(signed.token.email_id)
        .await?
    else {
        return Ok(None);
    };
    if message.direction != EmailDirection::Outbound
        || !message.tracking_enabled
        || message.status != EmailStatus::Sent
    {
        return Ok(None);
    }
    Ok(Some((signed, message)))
}

async fn should_record(
    ctx: &Ctx<'_>,
    message: &EmailMessageView,
    viewer: Option<&UserId>,
) -> Result<bool, AppError> {
    if let Some(viewer) = viewer {
        if message.created_by.as_ref() == Some(viewer) {
            return Ok(false);
        }
        if let Some(mailbox) = MailboxStore::of(ctx).find(message.mailbox_id).await?
            && mailbox.is_owned_by(viewer)
        {
            return Ok(false);
        }
    }
    let hashes: Vec<String> = message
        .participants
        .iter()
        .filter(|participant| participant.role != ParticipantRole::From)
        .map(|participant| address_hash(&participant.address))
        .collect();
    Ok(!TrackingStore::of(ctx).has_objection(&hashes).await?)
}

async fn engage(
    ctx: &Ctx<'_>,
    message: &EmailMessageView,
    kind: EngagementKind,
    extra: Value,
) -> Result<(), AppError> {
    let now = ctx.now();
    let (event_type, field) = match kind {
        EngagementKind::Opened => (Some(EMAIL_OPENED), "opened_at"),
        EngagementKind::Clicked => (Some(EMAIL_CLICKED), "clicked_at"),
        EngagementKind::Bounced => (Some(EMAIL_BOUNCED), "bounced_at"),
        EngagementKind::Replied => (None, "replied_at"),
    };
    let mut tx = ctx.begin().await?;
    let first = MessageStore::of(ctx)
        .record_engagement(&mut tx, message.id, kind, now)
        .await?;
    if first {
        if let Some(event_type) = event_type {
            let payload = email_event_payload(ctx, message, extra).await?;
            publish_email_event(ctx, &mut tx, &Actor::System, event_type, &payload).await?;
        }
        ctx.audit()
            .record(
                &mut tx,
                AuditEntry::new(Actor::System, event_type.unwrap_or("email.replied"), now)
                    .with_record(email_record(message))
                    .with_changes(vec![FieldChange::new(field, Value::Null, json!(now))]),
            )
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn publish_objection(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    actor: &Actor,
    action: &'static str,
    id: Uuid,
    label: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), action, now)
                .with_payload(json!({ "objection_id": id, "label": label })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor.clone(), action, now)
                .with_changes(vec![FieldChange::new("label", Value::Null, json!(label))])
                .with_ip(ip),
        )
        .await
}
