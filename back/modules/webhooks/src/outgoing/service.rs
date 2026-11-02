use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::outgoing::delivery::deliver_inline;
use crate::outgoing::model::{
    ALL_EVENTS, DeliveryListFilter, DeliveryStatus, DeliveryTrigger, DisabledReason,
    EventSubscription, MAX_DESCRIPTION_LENGTH, MAX_ENDPOINTS_PER_ORGANIZATION, MAX_URL_LENGTH,
    SIGNING_SECRET_NAME, SIGNING_SECRET_OWNER, SIGNING_SECRET_PREFIX, TEST_EVENT_TYPE,
    WEBHOOK_DELIVERY_RECORD_TYPE, WEBHOOK_ENDPOINT_RECORD_TYPE, WebhookDelivery, WebhookEndpoint,
    WebhookEndpointView,
};
use crate::outgoing::store::{WebhookDeliveryStore, WebhookEndpointStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::outbound_http::{OutboundUrlGuard, UrlRejection};
use aphel_kernel::application::public_events::is_public_event;
use aphel_kernel::application::secrets::{SecretKey, SecretVault};
use aphel_kernel::application::security::SecureTokens;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, Job, JobQueue,
    RecordRef, ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::Secret;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::auth::Sha256SecureTokens;

pub(crate) const WEBHOOK_ENDPOINT_NOT_FOUND: &str = "webhook_endpoint_not_found";
pub(crate) const WEBHOOK_DELIVERY_NOT_FOUND: &str = "webhook_delivery_not_found";
pub(crate) const WEBHOOK_URL_NOT_ALLOWED: &str = "webhook_url_not_allowed";
pub(crate) const WEBHOOK_ENDPOINT_DISABLED: &str = "webhook_endpoint_disabled";
pub(crate) const TOO_MANY_WEBHOOK_ENDPOINTS: &str = "too_many_webhook_endpoints";

pub(crate) const WEBHOOK_ENDPOINT_CREATED: &str = "webhook_endpoint.created";
pub(crate) const WEBHOOK_ENDPOINT_UPDATED: &str = "webhook_endpoint.updated";
pub(crate) const WEBHOOK_ENDPOINT_DELETED: &str = "webhook_endpoint.deleted";
pub(crate) const WEBHOOK_ENDPOINT_SECRET_ROTATED: &str = "webhook_endpoint.secret_rotated";
pub(crate) const WEBHOOK_ENDPOINT_TESTED: &str = "webhook_endpoint.tested";
pub(crate) const WEBHOOK_ENDPOINT_DISABLED_EVENT: &str = "webhook_endpoint.disabled";
pub(crate) const WEBHOOK_DELIVERY_REDELIVERED: &str = "webhook_delivery.redelivered";

pub(crate) const WEBHOOK_DELIVER_JOB: &str = "webhook.deliver";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateWebhookEndpointCommand {
    pub(crate) url: String,
    pub(crate) description: Option<String>,
    pub(crate) events: Vec<String>,
    pub(crate) is_active: Option<bool>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateWebhookEndpointCommand {
    pub(crate) url: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) events: Option<Vec<String>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_version: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebhookEndpointWithSecret {
    pub(crate) view: WebhookEndpointView,
    pub(crate) secret: Secret,
}

pub(crate) fn webhook_endpoint_not_found() -> AppError {
    AppError::not_found_code(WEBHOOK_ENDPOINT_NOT_FOUND, "webhook endpoint was not found")
}

fn webhook_delivery_not_found() -> AppError {
    AppError::not_found_code(WEBHOOK_DELIVERY_NOT_FOUND, "webhook delivery was not found")
}

fn parse_id(value: &str, not_found: fn() -> AppError) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| not_found())
}

pub(crate) fn secret_key(endpoint_id: Uuid) -> SecretKey {
    SecretKey::new(
        SIGNING_SECRET_OWNER,
        endpoint_id.to_string(),
        SIGNING_SECRET_NAME,
    )
}

pub(crate) fn deliver_job(delivery: &WebhookDelivery, run_at: DateTime<Utc>) -> Job {
    Job::new(
        WEBHOOK_DELIVER_JOB,
        json!({
            "delivery_id": delivery.id.to_string(),
            "endpoint_id": delivery.endpoint_id.to_string(),
        }),
    )
    .run_at(run_at)
    .with_max_attempts(3)
    .with_unique_key(format!(
        "webhook_delivery:{}:{}",
        delivery.id, delivery.attempts
    ))
}

fn clean_description(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn endpoint_snapshot(endpoint: &WebhookEndpoint) -> Value {
    json!({
        "url": endpoint.url,
        "description": endpoint.description,
        "events": endpoint.events.to_strings(),
        "is_active": endpoint.is_active,
    })
}

fn validate_events(values: &[String], violations: &mut Vec<FieldViolation>) -> EventSubscription {
    let mut events: Vec<String> = Vec::new();
    for value in values {
        let value = value.trim();
        if value != ALL_EVENTS && !is_public_event(value) {
            violations.push(FieldViolation::new(
                "events",
                format!("{value} is not a public event type; see GET /api/v1/events/catalog"),
            ));
            continue;
        }
        if !events.iter().any(|existing| existing == value) {
            events.push(value.to_owned());
        }
    }
    if values.is_empty() {
        violations.push(FieldViolation::new(
            "events",
            "must list at least one event type or \"*\" for all",
        ));
    }
    EventSubscription::from_strings(events)
}

fn validate_description(value: &Option<String>, violations: &mut Vec<FieldViolation>) {
    if value
        .as_ref()
        .is_some_and(|text| text.chars().count() > MAX_DESCRIPTION_LENGTH)
    {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
        ));
    }
}

pub(crate) fn test_event_body(
    scope: &AccessScope,
    endpoint: &WebhookEndpoint,
    event_id: Uuid,
    now: DateTime<Utc>,
) -> String {
    let actor = Actor::from_scope(scope);
    json!({
        "id": event_id.to_string(),
        "type": TEST_EVENT_TYPE,
        "actor": { "type": actor.kind(), "id": actor.id() },
        "occurred_at": now.to_rfc3339_opts(SecondsFormat::Millis, true),
        "data": {
            "endpoint_id": endpoint.id.to_string(),
            "message": "Evento de teste enviado pelo Aphel.",
        },
    })
    .to_string()
}

async fn validate_url(ctx: &Ctx<'_>, raw: &str) -> Result<String, AppError> {
    let url = raw.trim().to_owned();
    if url.is_empty() || url.len() > MAX_URL_LENGTH {
        return Err(AppError::invalid_fields(
            "invalid webhook endpoint",
            vec![FieldViolation::new(
                "url",
                format!("must have between 1 and {MAX_URL_LENGTH} characters"),
            )],
        ));
    }
    let network = ctx.outbound_http_network()?;
    match network.guard().check(&url).await {
        Ok(()) => Ok(url),
        Err(UrlRejection::Invalid(message)) => Err(AppError::invalid_fields(
            "invalid webhook endpoint",
            vec![FieldViolation::new("url", message)],
        )),
        Err(UrlRejection::NotAllowed(message)) => Err(AppError::invalid_fields_code(
            WEBHOOK_URL_NOT_ALLOWED,
            "the webhook URL points to an address that is not allowed",
            vec![FieldViolation::new("url", message)],
        )),
    }
}

async fn views(
    ctx: &Ctx<'_>,
    endpoints: Vec<WebhookEndpoint>,
) -> Result<Vec<WebhookEndpointView>, AppError> {
    let ids: Vec<String> = endpoints
        .iter()
        .map(|endpoint| endpoint.id.to_string())
        .collect();
    let configured = if ids.is_empty() {
        Vec::new()
    } else {
        ctx.state()
            .secrets
            .configured(ctx.scope, SIGNING_SECRET_OWNER, &ids)
            .await?
    };
    Ok(endpoints
        .into_iter()
        .map(|endpoint| {
            let id = endpoint.id.to_string();
            let secret_configured = configured
                .iter()
                .any(|key| key.owner_id == id && key.name == SIGNING_SECRET_NAME);
            WebhookEndpointView {
                endpoint,
                secret_configured,
            }
        })
        .collect())
}

async fn load(ctx: &Ctx<'_>, id: &str) -> Result<WebhookEndpoint, AppError> {
    let id = parse_id(id, webhook_endpoint_not_found)?;
    WebhookEndpointStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(webhook_endpoint_not_found)
}

fn new_secret() -> Result<Secret, AppError> {
    let token = Sha256SecureTokens.issue()?.token;
    Ok(Secret::new(format!(
        "{SIGNING_SECRET_PREFIX}{}",
        token.expose()
    )))
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<WebhookEndpointView>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let endpoints = WebhookEndpointStore::of(ctx).list().await?;
    views(ctx, endpoints).await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<WebhookEndpointView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let endpoint = load(ctx, id).await?;
    Ok(views(ctx, vec![endpoint]).await?.remove(0))
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateWebhookEndpointCommand,
) -> Result<WebhookEndpointWithSecret, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let description = clean_description(command.description);
    validate_description(&description, &mut violations);
    aphel_kernel::application::public_events::refresh_public_events(ctx.pool()).await?;
    let events = validate_events(&command.events, &mut violations);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid webhook endpoint",
            violations,
        ));
    }
    let url = validate_url(ctx, &command.url).await?;
    let endpoints = WebhookEndpointStore::of(ctx);
    if endpoints.count().await? >= MAX_ENDPOINTS_PER_ORGANIZATION {
        return Err(AppError::validation_code(
            TOO_MANY_WEBHOOK_ENDPOINTS,
            format!(
                "an organization can have at most {MAX_ENDPOINTS_PER_ORGANIZATION} webhook endpoints"
            ),
        ));
    }

    let now = ctx.now();
    let is_active = command.is_active.unwrap_or(true);
    let endpoint = WebhookEndpoint {
        id: Uuid::now_v7(),
        url,
        description,
        events,
        is_active,
        disabled_reason: (!is_active).then_some(DisabledReason::Manual),
        disabled_at: (!is_active).then_some(now),
        consecutive_failures: 0,
        last_delivery_at: None,
        last_success_at: None,
        created_by: Some(scope.user_id().clone()),
        created_by_name: None,
        created_at: now,
        updated_at: now,
    };
    let secret = new_secret()?;

    let mut tx = ctx.begin().await?;
    endpoints.insert(&mut tx, &endpoint).await?;
    ctx.state()
        .secrets
        .store(&mut tx, scope, &secret_key(endpoint.id), &secret)
        .await?;
    let changes = diff_values(
        &Value::Null,
        &endpoint_snapshot(&endpoint),
        &DiffOptions::default(),
    )?;
    publish(
        ctx,
        &mut tx,
        &endpoint,
        WEBHOOK_ENDPOINT_CREATED,
        changes,
        command.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(WebhookEndpointWithSecret {
        view: WebhookEndpointView {
            endpoint,
            secret_configured: true,
        },
        secret,
    })
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateWebhookEndpointCommand,
) -> Result<WebhookEndpointView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = load(ctx, id).await?;
    ensure_version(command.expected_version, current.updated_at)?;

    let mut violations = Vec::new();
    let mut updated = current.clone();
    if let Some(description) = command.description {
        updated.description = clean_description(description);
        validate_description(&updated.description, &mut violations);
    }
    if let Some(events) = &command.events {
        aphel_kernel::application::public_events::refresh_public_events(ctx.pool()).await?;
        updated.events = validate_events(events, &mut violations);
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid webhook endpoint",
            violations,
        ));
    }
    if let Some(url) = &command.url {
        updated.url = validate_url(ctx, url).await?;
    }
    let now = ctx.now();
    if let Some(is_active) = command.is_active
        && is_active != current.is_active
    {
        updated.is_active = is_active;
        if is_active {
            updated.disabled_reason = None;
            updated.disabled_at = None;
            updated.consecutive_failures = 0;
        } else {
            updated.disabled_reason = Some(DisabledReason::Manual);
            updated.disabled_at = Some(now);
        }
    }

    let changes = diff_values(
        &endpoint_snapshot(&current),
        &endpoint_snapshot(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(views(ctx, vec![current]).await?.remove(0));
    }
    updated.updated_at = now;

    let mut tx = ctx.begin().await?;
    WebhookEndpointStore::of(ctx)
        .update(&mut tx, &updated)
        .await?;
    publish(
        ctx,
        &mut tx,
        &updated,
        WEBHOOK_ENDPOINT_UPDATED,
        changes,
        command.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(views(ctx, vec![updated]).await?.remove(0))
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let endpoint = load(ctx, id).await?;
    let now = ctx.now();
    let changes = diff_values(
        &endpoint_snapshot(&endpoint),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ctx.state()
        .secrets
        .remove_owner(
            &mut tx,
            scope,
            SIGNING_SECRET_OWNER,
            &endpoint.id.to_string(),
        )
        .await?;
    WebhookEndpointStore::of(ctx)
        .delete(&mut tx, endpoint.id)
        .await?;
    publish(
        ctx,
        &mut tx,
        &endpoint,
        WEBHOOK_ENDPOINT_DELETED,
        changes,
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn rotate_secret(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<WebhookEndpointWithSecret, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut endpoint = load(ctx, id).await?;
    let now = ctx.now();
    let secret = new_secret()?;
    endpoint.updated_at = now;
    let mut tx = ctx.begin().await?;
    ctx.state()
        .secrets
        .store(&mut tx, scope, &secret_key(endpoint.id), &secret)
        .await?;
    WebhookEndpointStore::of(ctx)
        .update(&mut tx, &endpoint)
        .await?;
    publish(
        ctx,
        &mut tx,
        &endpoint,
        WEBHOOK_ENDPOINT_SECRET_ROTATED,
        vec![FieldChange::redacted("signing_secret")],
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(WebhookEndpointWithSecret {
        view: WebhookEndpointView {
            endpoint,
            secret_configured: true,
        },
        secret,
    })
}

pub(crate) async fn send_test(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<WebhookDelivery, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let endpoint = load(ctx, id).await?;
    let now = ctx.now();
    let event_id = Uuid::now_v7();
    let delivery = WebhookDelivery::new(
        endpoint.id,
        event_id,
        TEST_EVENT_TYPE,
        DeliveryTrigger::Test,
        test_event_body(scope, &endpoint, event_id, now),
        Some(scope.user_id().clone()),
        now,
    );
    ctx.outbound_http_network()?;
    let deliveries = WebhookDeliveryStore::of(ctx);
    let mut tx = ctx.begin().await?;
    deliveries.insert(&mut tx, &delivery).await?;
    publish(
        ctx,
        &mut tx,
        &endpoint,
        WEBHOOK_ENDPOINT_TESTED,
        vec![FieldChange::new(
            "delivery_id",
            Value::Null,
            json!(delivery.id.to_string()),
        )],
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    let system = AccessScope::system();
    deliver_inline(&ctx.with_scope(&system), &endpoint, &delivery).await?;
    Ok(deliveries
        .find(endpoint.id, delivery.id)
        .await?
        .unwrap_or(delivery))
}

pub(crate) async fn list_deliveries(
    ctx: &Ctx<'_>,
    endpoint_id: &str,
    filter: DeliveryListFilter,
) -> Result<(Vec<WebhookDelivery>, u64), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let endpoint = load(ctx, endpoint_id).await?;
    WebhookDeliveryStore::of(ctx)
        .list(endpoint.id, filter)
        .await
}

pub(crate) async fn get_delivery(
    ctx: &Ctx<'_>,
    endpoint_id: &str,
    delivery_id: &str,
) -> Result<WebhookDelivery, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let endpoint = load(ctx, endpoint_id).await?;
    let delivery_id = parse_id(delivery_id, webhook_delivery_not_found)?;
    WebhookDeliveryStore::of(ctx)
        .find(endpoint.id, delivery_id)
        .await?
        .ok_or_else(webhook_delivery_not_found)
}

pub(crate) async fn redeliver(
    ctx: &Ctx<'_>,
    endpoint_id: &str,
    delivery_id: &str,
    ip: Option<String>,
) -> Result<WebhookDelivery, AppError> {
    let scope = ctx.scope;
    let original = get_delivery(ctx, endpoint_id, delivery_id).await?;
    let endpoint = WebhookEndpointStore::of(ctx)
        .find(original.endpoint_id)
        .await?
        .ok_or_else(webhook_endpoint_not_found)?;
    if !endpoint.is_active {
        return Err(AppError::conflict_code(
            WEBHOOK_ENDPOINT_DISABLED,
            "the webhook endpoint is disabled; enable it before resending",
        ));
    }
    let now = ctx.now();
    let mut delivery = WebhookDelivery::new(
        endpoint.id,
        original.event_id,
        original.event_type.clone(),
        DeliveryTrigger::Redelivery,
        original.body.clone(),
        Some(scope.user_id().clone()),
        now,
    );
    delivery.redelivery_of = Some(original.id);
    let mut tx = ctx.begin().await?;
    WebhookDeliveryStore::of(ctx)
        .insert(&mut tx, &delivery)
        .await?;
    ctx.jobs()
        .enqueue(&mut tx, deliver_job(&delivery, now))
        .await?;
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(WEBHOOK_DELIVERY_RECORD_TYPE, delivery.id);
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), WEBHOOK_DELIVERY_REDELIVERED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "endpoint_id": endpoint.id.to_string(),
                    "event_id": original.event_id.to_string(),
                    "event_type": original.event_type,
                    "redelivery_of": original.id.to_string(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, WEBHOOK_DELIVERY_REDELIVERED, now)
                .with_record(record)
                .with_changes(vec![FieldChange::new(
                    "redelivery_of",
                    Value::Null,
                    json!(original.id.to_string()),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    Ok(delivery)
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    endpoint: &WebhookEndpoint,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(WEBHOOK_ENDPOINT_RECORD_TYPE, endpoint.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "url": endpoint.url,
                    "events": endpoint.events.to_strings(),
                    "is_active": endpoint.is_active,
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

pub(crate) fn delivery_status_filter(
    value: Option<&str>,
) -> Result<Option<DeliveryStatus>, AppError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => DeliveryStatus::parse(value).map(Some).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid query",
                vec![FieldViolation::new(
                    "status",
                    "must be pending, succeeded or failed",
                )],
            )
        }),
    }
}
