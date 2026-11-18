use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::incoming::store::IncomingWebhookStore;
use crate::incoming::{
    ActionFieldError, ActionOutcome, ActionStatus, ActionTarget, DEFAULT_TOLERANCE_SECONDS,
    ExecutionContext, FailureKind, FieldCatalog, INCOMING_WEBHOOK_DISABLED,
    INCOMING_WEBHOOK_OWNER_UNAVAILABLE, INCOMING_WEBHOOK_PERMISSION_DENIED,
    INCOMING_WEBHOOK_PROCESSED, INCOMING_WEBHOOK_RECORD_TYPE, INTERNAL_ERROR,
    INVALID_WEBHOOK_PAYLOAD, IncomingDelivery, IncomingRequestLog, IncomingWebhook,
    MAX_DELIVERY_ID_LENGTH, MAX_PROCESSED_BODY_BYTES, MAX_TOLERANCE_SECONDS, MIN_TOLERANCE_SECONDS,
    ReceiveFailure, ReceiveOutcome, RequestStatus, SECRET_OWNER_TYPE, SIGNING_SECRET_NAME,
    SignatureFailure, WEBHOOK_MAPPING_FAILED, WEBHOOK_PAYLOAD_TOO_LARGE, WEBHOOK_REPLAYED,
    encode_hex, log_scope, looks_like_token, outcome, parse_payload, run_action, signature_failure,
    truncated_body, validate_actions, verify_signature,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::Secret;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::{AccessRepository, Authorizer, build_access_scope};
use aphel_sdk::platform::application::audit::{DiffOptions, diff_values};
use aphel_sdk::platform::application::auth::UserRepository;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::secrets::{SecretKey, SecretVault};
use aphel_sdk::platform::application::security::SecureTokens;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::domain::users::User;
use aphel_sdk::platform::infrastructure::auth::Sha256SecureTokens;
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_sdk::platform::infrastructure::repositories::mysql_users::MySqlUserRepository;

pub(crate) const INCOMING_WEBHOOK_NOT_FOUND: &str = "incoming_webhook_not_found";
pub(crate) const INCOMING_WEBHOOK_REQUEST_NOT_FOUND: &str = "incoming_webhook_request_not_found";
pub(crate) const TOO_MANY_INCOMING_WEBHOOKS: &str = "too_many_incoming_webhooks";

pub(crate) const INCOMING_WEBHOOK_CREATED: &str = "incoming_webhook.created";
pub(crate) const INCOMING_WEBHOOK_UPDATED: &str = "incoming_webhook.updated";
pub(crate) const INCOMING_WEBHOOK_DELETED: &str = "incoming_webhook.deleted";
pub(crate) const INCOMING_WEBHOOK_TOKEN_REGENERATED: &str = "incoming_webhook.token_regenerated";
pub(crate) const INCOMING_WEBHOOK_SECRET_ROTATED: &str = "incoming_webhook.signing_secret_rotated";
pub(crate) const INCOMING_WEBHOOK_SECRET_REMOVED: &str = "incoming_webhook.signing_secret_removed";

pub(crate) const WEBHOOK_TOKEN_PREFIX: &str = "whk_";
pub(crate) const SIGNING_SECRET_PREFIX: &str = "whsec_";
pub(crate) const TOKEN_PREFIX_LENGTH: usize = 12;
pub(crate) const MAX_NAME_LENGTH: usize = 100;
pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 500;
pub(crate) const MAX_WEBHOOKS_PER_ORGANIZATION: usize = 50;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SaveIncomingWebhookCommand {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) actions: Option<Vec<Value>>,
    pub(crate) signature_tolerance_seconds: Option<u32>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CreateIncomingWebhookCommand {
    pub(crate) save: SaveIncomingWebhookCommand,
    pub(crate) generate_signing_secret: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IssuedIncomingWebhook {
    pub(crate) webhook: IncomingWebhook,
    pub(crate) token: Option<Secret>,
    pub(crate) signing_secret: Option<Secret>,
}

pub(crate) fn incoming_webhook_not_found() -> AppError {
    AppError::not_found_code(INCOMING_WEBHOOK_NOT_FOUND, "incoming webhook was not found")
}

fn request_not_found() -> AppError {
    AppError::not_found_code(
        INCOMING_WEBHOOK_REQUEST_NOT_FOUND,
        "incoming webhook request was not found",
    )
}

pub(crate) fn parse_webhook_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| incoming_webhook_not_found())
}

pub(crate) fn signing_secret_key(webhook_id: Uuid) -> SecretKey {
    SecretKey::new(
        SECRET_OWNER_TYPE,
        webhook_id.to_string(),
        SIGNING_SECRET_NAME,
    )
}

fn snapshot(webhook: &IncomingWebhook) -> Value {
    json!({
        "name": webhook.name,
        "description": webhook.description,
        "is_active": webhook.is_active,
        "signature_required": webhook.signature_required,
        "signature_tolerance_seconds": webhook.signature_tolerance_seconds,
        "actions": webhook.actions,
    })
}

fn normalized_description(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn store(ctx: &Ctx<'_>) -> IncomingWebhookStore {
    IncomingWebhookStore::of(ctx)
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<IncomingWebhook>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    store(ctx).list().await
}

pub(crate) async fn get(ctx: &Ctx<'_>, webhook_id: &str) -> Result<IncomingWebhook, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    find(ctx, webhook_id).await
}

pub(crate) async fn targets(ctx: &Ctx<'_>) -> Result<Vec<ActionTarget>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    Ok(FieldCatalog::load(ctx).await?.targets())
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateIncomingWebhookCommand,
) -> Result<IssuedIncomingWebhook, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let now = ctx.now();
    let save = command.save;
    let mut violations = Vec::new();
    let name = validate_name(save.name.as_deref().unwrap_or_default(), &mut violations);
    let description = validate_description(save.description.clone().flatten(), &mut violations);
    let tolerance = validate_tolerance(
        save.signature_tolerance_seconds
            .unwrap_or(DEFAULT_TOLERANCE_SECONDS),
        &mut violations,
    );
    let actions = validate_actions(ctx, save.actions.unwrap_or_default(), &mut violations).await?;
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid incoming webhook",
            violations,
        ));
    }
    let webhooks = store(ctx);
    if webhooks.list().await?.len() >= MAX_WEBHOOKS_PER_ORGANIZATION {
        return Err(AppError::validation_code(
            TOO_MANY_INCOMING_WEBHOOKS,
            format!(
                "an organization can have at most {MAX_WEBHOOKS_PER_ORGANIZATION} incoming webhooks"
            ),
        ));
    }

    let (token, token_hash) = issue_token()?;
    let signing_secret = command
        .generate_signing_secret
        .then(issue_secret)
        .transpose()?;
    let webhook = IncomingWebhook {
        id: Uuid::now_v7(),
        name,
        description,
        token_prefix: token_prefix(&token),
        is_active: save.is_active.unwrap_or(true),
        signature_required: signing_secret.is_some(),
        signature_tolerance_seconds: tolerance,
        actions,
        created_by: scope.user_id().clone(),
        created_by_name: None,
        last_received_at: None,
        created_at: now,
        updated_at: now,
    };

    let mut tx = ctx.begin().await?;
    webhooks.insert(&mut tx, &webhook, &token_hash).await?;
    if let Some(secret) = &signing_secret {
        ctx.runtime()
            .secrets()
            .store(&mut tx, scope, &signing_secret_key(webhook.id), secret)
            .await?;
    }
    let changes = diff_values(&Value::Null, &snapshot(&webhook), &DiffOptions::default())?;
    publish(
        ctx,
        &mut tx,
        &webhook,
        INCOMING_WEBHOOK_CREATED,
        changes,
        save.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    let webhook = reload(ctx, webhook.id).await?;
    Ok(IssuedIncomingWebhook {
        webhook,
        token: Some(token),
        signing_secret,
    })
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    command: SaveIncomingWebhookCommand,
) -> Result<IncomingWebhook, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = find(ctx, webhook_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut next = current.clone();
    if let Some(name) = command.name.as_deref() {
        next.name = validate_name(name, &mut violations);
    }
    if let Some(description) = command.description {
        next.description = validate_description(description, &mut violations);
    }
    if let Some(is_active) = command.is_active {
        next.is_active = is_active;
    }
    if let Some(tolerance) = command.signature_tolerance_seconds {
        next.signature_tolerance_seconds = validate_tolerance(tolerance, &mut violations);
    }
    if let Some(actions) = command.actions {
        next.actions = validate_actions(ctx, actions, &mut violations).await?;
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid incoming webhook",
            violations,
        ));
    }
    let changes = diff_values(
        &snapshot(&current),
        &snapshot(&next),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    let now = ctx.now();
    next.updated_at = now;
    let mut tx = ctx.begin().await?;
    store(ctx).update(&mut tx, &next).await?;
    publish(
        ctx,
        &mut tx,
        &next,
        INCOMING_WEBHOOK_UPDATED,
        changes,
        command.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    reload(ctx, next.id).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let webhook = find(ctx, webhook_id).await?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    ctx.runtime()
        .secrets()
        .remove_owner(&mut tx, scope, SECRET_OWNER_TYPE, &webhook.id.to_string())
        .await?;
    store(ctx).delete(&mut tx, webhook.id).await?;
    let changes = diff_values(&snapshot(&webhook), &Value::Null, &DiffOptions::default())?;
    publish(
        ctx,
        &mut tx,
        &webhook,
        INCOMING_WEBHOOK_DELETED,
        changes,
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn regenerate_token(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    ip: Option<String>,
) -> Result<IssuedIncomingWebhook, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let webhook = find(ctx, webhook_id).await?;
    let (token, token_hash) = issue_token()?;
    let prefix = token_prefix(&token);
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    store(ctx)
        .replace_token(&mut tx, webhook.id, &prefix, &token_hash, now)
        .await?;
    publish(
        ctx,
        &mut tx,
        &webhook,
        INCOMING_WEBHOOK_TOKEN_REGENERATED,
        vec![FieldChange::new(
            "token_prefix",
            json!(webhook.token_prefix),
            json!(prefix),
        )],
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(IssuedIncomingWebhook {
        webhook: reload(ctx, webhook.id).await?,
        token: Some(token),
        signing_secret: None,
    })
}

pub(crate) async fn rotate_signing_secret(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    ip: Option<String>,
) -> Result<IssuedIncomingWebhook, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let webhook = find(ctx, webhook_id).await?;
    let secret = issue_secret()?;
    let now = ctx.now();
    let mut next = webhook.clone();
    next.signature_required = true;
    next.updated_at = now;
    let mut tx = ctx.begin().await?;
    ctx.runtime()
        .secrets()
        .store(&mut tx, scope, &signing_secret_key(webhook.id), &secret)
        .await?;
    store(ctx).update(&mut tx, &next).await?;
    publish(
        ctx,
        &mut tx,
        &next,
        INCOMING_WEBHOOK_SECRET_ROTATED,
        vec![FieldChange::redacted("signing_secret")],
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(IssuedIncomingWebhook {
        webhook: reload(ctx, webhook.id).await?,
        token: None,
        signing_secret: Some(secret),
    })
}

pub(crate) async fn remove_signing_secret(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    ip: Option<String>,
) -> Result<IncomingWebhook, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let webhook = find(ctx, webhook_id).await?;
    if !webhook.signature_required {
        return Ok(webhook);
    }
    let now = ctx.now();
    let mut next = webhook.clone();
    next.signature_required = false;
    next.updated_at = now;
    let mut tx = ctx.begin().await?;
    ctx.runtime()
        .secrets()
        .remove(&mut tx, scope, &signing_secret_key(webhook.id))
        .await?;
    store(ctx).update(&mut tx, &next).await?;
    publish(
        ctx,
        &mut tx,
        &next,
        INCOMING_WEBHOOK_SECRET_REMOVED,
        vec![FieldChange::new(
            "signature_required",
            json!(true),
            json!(false),
        )],
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    reload(ctx, webhook.id).await
}

pub(crate) async fn list_requests(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    status: Option<RequestStatus>,
    window: ListWindow,
) -> Result<Listing<IncomingRequestLog>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let webhook = find(ctx, webhook_id).await?;
    store(ctx).list_requests(webhook.id, status, window).await
}

pub(crate) async fn get_request(
    ctx: &Ctx<'_>,
    webhook_id: &str,
    request_id: &str,
) -> Result<IncomingRequestLog, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let webhook = find(ctx, webhook_id).await?;
    let request_id = Uuid::parse_str(request_id.trim()).map_err(|_| request_not_found())?;
    store(ctx)
        .find_request(webhook.id, request_id)
        .await?
        .ok_or_else(request_not_found)
}

struct Pending {
    request_id: Uuid,
    started: Instant,
    log: IncomingRequestLog,
}

pub(crate) async fn receive(
    ctx: &Ctx<'_>,
    delivery: IncomingDelivery,
) -> Result<ReceiveOutcome, AppError> {
    let started = Instant::now();
    let token = delivery.token.trim();
    if !looks_like_token(token) {
        return Err(incoming_webhook_not_found());
    }
    let webhooks = store(ctx);
    let tokens = Sha256SecureTokens;
    let webhook = webhooks
        .find_by_hash(&tokens.digest(token))
        .await?
        .ok_or_else(incoming_webhook_not_found)?;
    let scope = log_scope(&webhook)?;
    let now = ctx.now();
    let request_id = Uuid::now_v7();
    let (body_text, body_truncated) = match &delivery.body {
        Some(body) => {
            let (text, truncated) = truncated_body(body);
            (Some(text), truncated)
        }
        None => (None, true),
    };
    let mut pending = Pending {
        request_id,
        started,
        log: IncomingRequestLog {
            id: request_id,
            webhook_id: webhook.id,
            received_at: now,
            ip: delivery.ip.clone(),
            content_type: delivery
                .content_type
                .as_ref()
                .map(|value| value.chars().take(255).collect()),
            headers: delivery.headers.clone(),
            body: body_text,
            body_bytes: delivery.body_bytes,
            body_truncated,
            status: RequestStatus::Rejected,
            http_status: 200,
            error_code: None,
            error_message: None,
            results: Vec::new(),
            duration_ms: 0,
        },
    };
    webhooks.touch_received(webhook.id, now).await?;

    let Some(body) = delivery.body.as_deref() else {
        return finish(
            &webhooks,
            &mut pending,
            RequestStatus::Rejected,
            Some(ReceiveFailure::new(
                413,
                WEBHOOK_PAYLOAD_TOO_LARGE,
                "the request body is too large",
            )),
        )
        .await;
    };
    if !webhook.is_active {
        return finish(
            &webhooks,
            &mut pending,
            RequestStatus::Rejected,
            Some(ReceiveFailure::new(
                403,
                INCOMING_WEBHOOK_DISABLED,
                "this incoming webhook is disabled",
            )),
        )
        .await;
    }

    let mut replay_key = delivery
        .delivery_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let value: String = value.chars().take(MAX_DELIVERY_ID_LENGTH).collect();
            tokens.digest(&format!("delivery:{value}"))
        });
    if webhook.signature_required {
        let secret = match ctx
            .runtime()
            .secrets()
            .reveal(&scope, &signing_secret_key(webhook.id))
            .await
        {
            Ok(secret) => secret,
            Err(error) => {
                tracing::error!(error = %error, webhook_id = %webhook.id, "incoming webhook secret unavailable");
                return finish(
                    &webhooks,
                    &mut pending,
                    RequestStatus::Failed,
                    Some(ReceiveFailure::new(
                        500,
                        INTERNAL_ERROR,
                        "internal server error",
                    )),
                )
                .await;
            }
        };
        let verified = match secret {
            Some(secret) => verify_signature(
                secret.expose(),
                delivery.signature.as_deref(),
                body,
                now,
                webhook.signature_tolerance_seconds,
            ),
            None => Err(SignatureFailure::Mismatch),
        };
        match verified {
            Ok(parsed) => {
                if replay_key.is_none() {
                    let signature = parsed
                        .signatures
                        .first()
                        .map(|bytes| encode_hex(bytes))
                        .unwrap_or_default();
                    replay_key =
                        Some(tokens.digest(&format!("signature:{}:{signature}", parsed.timestamp)));
                }
            }
            Err(failure) => {
                return finish(
                    &webhooks,
                    &mut pending,
                    RequestStatus::Rejected,
                    Some(signature_failure(failure)),
                )
                .await;
            }
        }
    }

    let payload = match parse_payload(delivery.content_type.as_deref(), body) {
        Ok(payload) => payload,
        Err(message) => {
            return finish(
                &webhooks,
                &mut pending,
                RequestStatus::Rejected,
                Some(ReceiveFailure::new(422, INVALID_WEBHOOK_PAYLOAD, message)),
            )
            .await;
        }
    };

    if let Some(key) = &replay_key
        && !webhooks.remember_delivery(webhook.id, key, now).await?
    {
        return finish(
            &webhooks,
            &mut pending,
            RequestStatus::Rejected,
            Some(ReceiveFailure::new(
                409,
                WEBHOOK_REPLAYED,
                "this delivery was already received",
            )),
        )
        .await;
    }

    let Some(actor) = actor_scope(ctx, &webhook).await? else {
        forget(&webhooks, &webhook, replay_key.as_deref()).await?;
        return finish(
            &webhooks,
            &mut pending,
            RequestStatus::Failed,
            Some(ReceiveFailure::new(
                403,
                INCOMING_WEBHOOK_OWNER_UNAVAILABLE,
                "the user who created this webhook is no longer active in the organization",
            )),
        )
        .await;
    };
    let actor_ctx = ctx.with_scope(&actor);

    let mut context = ExecutionContext::default();
    let mut failure = None;
    let mut succeeded = 0usize;
    let mut processed = Vec::new();
    for (index, action) in webhook.actions.iter().enumerate() {
        if failure.is_some() {
            pending
                .log
                .results
                .push(outcome(index, action, ActionStatus::Skipped));
            continue;
        }
        match run_action(
            &actor_ctx,
            action,
            &payload,
            &mut context,
            delivery.ip.clone(),
        )
        .await
        {
            Ok((record_type, record_id, operation)) => {
                succeeded += 1;
                processed.push((
                    RecordRef::new(record_type.clone(), record_id),
                    operation.to_owned(),
                ));
                pending.log.results.push(ActionOutcome {
                    operation: Some(operation.to_owned()),
                    record_type: Some(record_type.as_str().to_owned()),
                    record_id: Some(record_id.to_string()),
                    ..outcome(index, action, ActionStatus::Succeeded)
                });
            }
            Err(action_failure) => {
                let receive_failure = match action_failure.kind {
                    FailureKind::Mapping => ReceiveFailure {
                        http_status: 422,
                        code: WEBHOOK_MAPPING_FAILED,
                        message: format!(
                            "action {index} ({}) failed: {}",
                            action.type_name(),
                            action_failure.error.message
                        ),
                        fields: action_failure
                            .error
                            .fields
                            .iter()
                            .map(|field| ActionFieldError {
                                field: format!("actions.{index}.{}", field.field),
                                message: field.message.clone(),
                            })
                            .collect(),
                    },
                    FailureKind::Permission => ReceiveFailure::new(
                        403,
                        INCOMING_WEBHOOK_PERMISSION_DENIED,
                        format!(
                            "the user who created this webhook cannot run action {index} ({})",
                            action.type_name()
                        ),
                    ),
                    FailureKind::Internal => {
                        ReceiveFailure::new(500, INTERNAL_ERROR, "internal server error")
                    }
                };
                pending.log.results.push(ActionOutcome {
                    error: Some(action_failure.error),
                    ..outcome(index, action, ActionStatus::Failed)
                });
                failure = Some(receive_failure);
            }
        }
    }

    publish_processed(&actor_ctx, &webhook, &pending, &payload, processed).await?;
    let status = match (&failure, succeeded) {
        (None, _) => RequestStatus::Succeeded,
        (Some(_), 0) => RequestStatus::Failed,
        (Some(_), _) => RequestStatus::Partial,
    };
    if status == RequestStatus::Failed {
        forget(&webhooks, &webhook, replay_key.as_deref()).await?;
    }
    finish(&webhooks, &mut pending, status, failure).await
}

async fn publish_processed(
    ctx: &Ctx<'_>,
    webhook: &IncomingWebhook,
    pending: &Pending,
    payload: &Value,
    processed: Vec<(RecordRef, String)>,
) -> Result<(), AppError> {
    if processed.is_empty() {
        return Ok(());
    }
    let body = serde_json::to_vec(payload)
        .ok()
        .filter(|bytes| bytes.len() <= MAX_PROCESSED_BODY_BYTES)
        .map(|_| payload.clone())
        .unwrap_or(Value::Null);
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    for (record, operation) in processed {
        let event = DomainEvent::new(
            Actor::from_scope(ctx.scope),
            INCOMING_WEBHOOK_PROCESSED,
            now,
        )
        .with_record(record)
        .with_payload(json!({
            "incoming_webhook_id": webhook.id.to_string(),
            "request_id": pending.request_id.to_string(),
            "operation": operation,
            "body": body,
        }));
        ctx.events().publish(&mut tx, event).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn forget(
    webhooks: &IncomingWebhookStore,
    webhook: &IncomingWebhook,
    replay_key: Option<&str>,
) -> Result<(), AppError> {
    match replay_key {
        Some(key) => webhooks.forget_delivery(webhook.id, key).await,
        None => Ok(()),
    }
}

async fn actor_scope(
    ctx: &Ctx<'_>,
    webhook: &IncomingWebhook,
) -> Result<Option<AccessScope>, AppError> {
    let Some(_user) = MySqlUserRepository::new(ctx.pool().clone())
        .find_by_id(&webhook.created_by)
        .await?
        .filter(User::is_active)
    else {
        return Ok(None);
    };
    Ok(MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&webhook.created_by)
        .await?
        .map(|access| build_access_scope(webhook.created_by.clone(), access)))
}

async fn finish(
    webhooks: &IncomingWebhookStore,
    pending: &mut Pending,
    status: RequestStatus,
    failure: Option<ReceiveFailure>,
) -> Result<ReceiveOutcome, AppError> {
    pending.log.status = status;
    pending.log.duration_ms =
        u64::try_from(pending.started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if let Some(failure) = &failure {
        pending.log.http_status = failure.http_status;
        pending.log.error_code = Some(failure.code.to_owned());
        pending.log.error_message = Some(failure.message.chars().take(1000).collect());
    }
    webhooks.insert_request(&pending.log).await?;
    Ok(ReceiveOutcome {
        request_id: pending.request_id,
        status,
        failure,
    })
}

async fn find(ctx: &Ctx<'_>, webhook_id: &str) -> Result<IncomingWebhook, AppError> {
    let id = parse_webhook_id(webhook_id)?;
    reload(ctx, id).await
}

async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<IncomingWebhook, AppError> {
    store(ctx)
        .find(id)
        .await?
        .ok_or_else(incoming_webhook_not_found)
}

fn issue_token() -> Result<(Secret, String), AppError> {
    let tokens = Sha256SecureTokens;
    let issued = tokens.issue()?;
    let token = Secret::new(format!("{WEBHOOK_TOKEN_PREFIX}{}", issued.token.expose()));
    let hash = tokens.digest(token.expose());
    Ok((token, hash))
}

fn issue_secret() -> Result<Secret, AppError> {
    let issued = Sha256SecureTokens.issue()?;
    Ok(Secret::new(format!(
        "{SIGNING_SECRET_PREFIX}{}",
        issued.token.expose()
    )))
}

fn validate_name(name: &str, violations: &mut Vec<FieldViolation>) -> String {
    let name = name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_NAME_LENGTH} characters"),
        ));
    }
    name
}

fn validate_description(
    description: Option<String>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let description = normalized_description(description);
    if description
        .as_ref()
        .is_some_and(|text| text.chars().count() > MAX_DESCRIPTION_LENGTH)
    {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
        ));
    }
    description
}

fn validate_tolerance(value: u32, violations: &mut Vec<FieldViolation>) -> u32 {
    if !(MIN_TOLERANCE_SECONDS..=MAX_TOLERANCE_SECONDS).contains(&value) {
        violations.push(FieldViolation::new(
            "signature_tolerance_seconds",
            format!("must be between {MIN_TOLERANCE_SECONDS} and {MAX_TOLERANCE_SECONDS}"),
        ));
    }
    value
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    webhook: &IncomingWebhook,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(INCOMING_WEBHOOK_RECORD_TYPE, webhook.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "name": webhook.name,
                    "is_active": webhook.is_active,
                    "signature_required": webhook.signature_required,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

fn token_prefix(token: &Secret) -> String {
    token.expose().chars().take(TOKEN_PREFIX_LENGTH).collect()
}
