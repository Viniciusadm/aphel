use std::collections::HashMap;

use chrono::Duration;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::outgoing::model::{
    DeliveryAttempt, DeliveryStatus, DeliveryTrigger, DisabledReason, MAX_ERROR_LENGTH,
    OutboundRequest, OutboundResponse, WEBHOOK_ENDPOINT_RECORD_TYPE, WebhookDelivery,
    WebhookEndpoint, retry_delay, truncate_chars,
};
use crate::outgoing::service::{WEBHOOK_ENDPOINT_DISABLED_EVENT, deliver_job, secret_key};
use crate::outgoing::store::{WebhookDeliveryStore, WebhookEndpointStore};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::outbound_http::OutboundSender;
use aphel_sdk::platform::application::public_events::SharedPublicEventRenderer;
use aphel_sdk::platform::application::secrets::SecretVault;
use aphel_sdk::platform::application::security::signature::sign_payload;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EnqueueOutcome, EventPublisher, FieldChange,
    JobQueue, RecordRef,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::outbound_http::OutboundHttpNetwork;

pub(crate) const OUTGOING_WEBHOOKS_CONSUMER: &str = "webhooks.outgoing";
pub(crate) const WEBHOOKS_MAINTENANCE_JOB: &str = "webhooks.maintenance";
const WEBHOOKS_MAINTENANCE_CRON: &str = "0 */10 * * * *";
const STALLED_AFTER_MINUTES: i64 = 10;
const STALLED_BATCH: u32 = 500;
const PURGE_BATCH: u32 = 5000;
const PURGE_MAX_BATCHES: u32 = 20;
pub(crate) const INLINE_DELIVERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub(crate) fn webhooks_maintenance_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(WEBHOOKS_MAINTENANCE_JOB, WEBHOOKS_MAINTENANCE_CRON)
        .expect("the webhooks maintenance cron expression is valid")
        .with_max_attempts(2)
}

pub(crate) async fn fan_out(
    ctx: &Ctx<'_>,
    renderer: &SharedPublicEventRenderer,
    event: &DomainEvent,
) -> Result<(), AppError> {
    let endpoints: Vec<WebhookEndpoint> = WebhookEndpointStore::of(ctx)
        .active()
        .await?
        .into_iter()
        .filter(|endpoint| endpoint.events.includes(&event.event_type))
        .collect();
    if endpoints.is_empty() {
        return Ok(());
    }
    let Some(public) = renderer.render(event).await? else {
        return Ok(());
    };
    let mut bodies: HashMap<Option<UserId>, String> = HashMap::new();
    for endpoint in &endpoints {
        if !bodies.contains_key(&endpoint.created_by) {
            let restricted = renderer
                .restrict(&public, endpoint.created_by.as_ref())
                .await?;
            bodies.insert(
                endpoint.created_by.clone(),
                restricted.to_json().to_string(),
            );
        }
    }
    let now = ctx.now();
    let deliveries = WebhookDeliveryStore::of(ctx);

    let mut tx = ctx.begin().await?;
    for endpoint in endpoints {
        let mut delivery = WebhookDelivery::new(
            endpoint.id,
            event.id,
            event.event_type.clone(),
            DeliveryTrigger::Event,
            bodies
                .get(&endpoint.created_by)
                .cloned()
                .unwrap_or_default(),
            None,
            now,
        );
        delivery.dedupe_key = Some(format!("event:{}:{}", event.id, endpoint.id));
        if deliveries.insert(&mut tx, &delivery).await? {
            ctx.jobs()
                .enqueue(&mut tx, deliver_job(&delivery, now))
                .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

fn payload_id(payload: &Value, key: &str) -> Result<Uuid, AppError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| AppError::Internal(format!("webhook delivery job without a valid {key}")))
}

fn failure(message: impl Into<String>) -> OutboundResponse {
    OutboundResponse {
        status: None,
        body: None,
        duration_ms: 0,
        error: Some(message.into()),
    }
}

pub(crate) async fn deliver(ctx: &Ctx<'_>, payload: Value) -> Result<(), AppError> {
    let network = ctx.outbound_http_network()?;
    let delivery_id = payload_id(&payload, "delivery_id")?;
    let endpoint_id = payload_id(&payload, "endpoint_id")?;
    let Some(delivery) = WebhookDeliveryStore::of(ctx)
        .find(endpoint_id, delivery_id)
        .await?
    else {
        return Ok(());
    };
    if delivery.status != DeliveryStatus::Pending {
        return Ok(());
    }
    let Some(endpoint) = WebhookEndpointStore::of(ctx).find(endpoint_id).await? else {
        return Ok(());
    };
    if !endpoint.is_active {
        return cancel(ctx, &delivery, "the endpoint is disabled").await;
    }
    let response = attempt(ctx, &network, &endpoint, &delivery).await;
    finish(ctx, &network, &endpoint, &delivery, response).await
}

pub(crate) async fn deliver_inline(
    ctx: &Ctx<'_>,
    endpoint: &WebhookEndpoint,
    delivery: &WebhookDelivery,
) -> Result<(), AppError> {
    let network = ctx.outbound_http_network()?;
    if !endpoint.is_active {
        return cancel(ctx, delivery, "the endpoint is disabled").await;
    }
    let response = tokio::time::timeout(
        INLINE_DELIVERY_TIMEOUT,
        attempt(ctx, &network, endpoint, delivery),
    )
    .await
    .unwrap_or_else(|_| {
        failure(format!(
            "timed out after {} s",
            INLINE_DELIVERY_TIMEOUT.as_secs()
        ))
    });
    finish(ctx, &network, endpoint, delivery, response).await
}

async fn attempt(
    ctx: &Ctx<'_>,
    network: &OutboundHttpNetwork,
    endpoint: &WebhookEndpoint,
    delivery: &WebhookDelivery,
) -> OutboundResponse {
    let secret = match ctx
        .runtime()
        .secrets()
        .reveal(ctx.scope, &secret_key(endpoint.id))
        .await
    {
        Ok(Some(secret)) => secret,
        Ok(None) => return failure("the endpoint has no signing secret"),
        Err(error) => {
            tracing::warn!(endpoint_id = %endpoint.id, "webhook signing secret unavailable: {error}");
            return failure("the signing secret is unavailable");
        }
    };
    let timestamp = ctx.now().timestamp();
    let signature = match sign_payload(secret.expose(), timestamp, &delivery.body) {
        Ok(signature) => signature,
        Err(_) => return failure("could not sign the payload"),
    };
    network
        .http_sender()
        .send(&OutboundRequest {
            url: endpoint.url.clone(),
            headers: vec![
                ("X-Aphel-Delivery".to_owned(), delivery.id.to_string()),
                ("X-Aphel-Event".to_owned(), delivery.event_type.clone()),
                ("X-Aphel-Signature".to_owned(), signature),
            ],
            body: delivery.body.clone(),
            ..OutboundRequest::default()
        })
        .await
}

async fn finish(
    ctx: &Ctx<'_>,
    _network: &OutboundHttpNetwork,
    endpoint: &WebhookEndpoint,
    delivery: &WebhookDelivery,
    response: OutboundResponse,
) -> Result<(), AppError> {
    let now = ctx.now();
    let attempts = delivery.attempts + 1;
    let succeeded = response.succeeded();
    let retry = !succeeded && attempts < delivery.max_attempts;
    let status = if succeeded {
        DeliveryStatus::Succeeded
    } else if retry {
        DeliveryStatus::Pending
    } else {
        DeliveryStatus::Failed
    };
    let attempt = DeliveryAttempt {
        status,
        attempts,
        attempted_at: now,
        next_attempt_at: retry.then(|| now + retry_delay(attempts)),
        response_status: response.status,
        response_body: response.body,
        duration_ms: Some(response.duration_ms),
        error: response
            .error
            .map(|error| truncate_chars(&error, MAX_ERROR_LENGTH)),
    };
    let endpoints = WebhookEndpointStore::of(ctx);

    let mut tx = ctx.begin().await?;
    WebhookDeliveryStore::of(ctx)
        .record_attempt(&mut tx, delivery.id, &attempt)
        .await?;
    if succeeded {
        endpoints.record_success(&mut tx, endpoint.id, now).await?;
    } else {
        let counts = !retry && delivery.trigger == DeliveryTrigger::Event;
        let failures = endpoints
            .record_failure(&mut tx, endpoint.id, now, counts)
            .await?;
        if let Some(next_attempt_at) = attempt.next_attempt_at {
            let mut next = delivery.clone();
            next.attempts = attempts;
            ctx.jobs()
                .enqueue(&mut tx, deliver_job(&next, next_attempt_at))
                .await?;
        }
        if counts
            && failures
                >= crate::settings::delivery_settings(&ctx.runtime().configuration())
                    .disable_after_failures
            && endpoints
                .disable(&mut tx, endpoint.id, DisabledReason::Failures, now)
                .await?
        {
            publish_disabled(ctx, &mut tx, &endpoints, endpoint, failures).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn publish_disabled(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    endpoints: &WebhookEndpointStore,
    endpoint: &WebhookEndpoint,
    failures: u32,
) -> Result<(), AppError> {
    let now = ctx.now();
    let recipients: Vec<String> = endpoints
        .settings_managers()
        .await?
        .iter()
        .map(|user| user.as_str().to_owned())
        .collect();
    let record = RecordRef::new(WEBHOOK_ENDPOINT_RECORD_TYPE, endpoint.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(Actor::System, WEBHOOK_ENDPOINT_DISABLED_EVENT, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "endpoint_id": endpoint.id.to_string(),
                    "url": endpoint.url,
                    "consecutive_failures": failures,
                    "reason": DisabledReason::Failures.as_str(),
                    "recipients": recipients,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(Actor::System, WEBHOOK_ENDPOINT_DISABLED_EVENT, now)
                .with_record(record)
                .with_changes(vec![
                    FieldChange::new("is_active", json!(true), json!(false)),
                    FieldChange::new(
                        "disabled_reason",
                        Value::Null,
                        json!(DisabledReason::Failures.as_str()),
                    ),
                ]),
        )
        .await
}

async fn cancel(ctx: &Ctx<'_>, delivery: &WebhookDelivery, message: &str) -> Result<(), AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    WebhookDeliveryStore::of(ctx)
        .record_attempt(
            &mut tx,
            delivery.id,
            &DeliveryAttempt {
                status: DeliveryStatus::Failed,
                attempts: delivery.attempts,
                attempted_at: delivery.last_attempt_at.unwrap_or(now),
                next_attempt_at: None,
                response_status: delivery.response_status,
                response_body: delivery.response_body.clone(),
                duration_ms: delivery.duration_ms,
                error: Some(message.to_owned()),
            },
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn run_maintenance(ctx: &Ctx<'_>, retention_days: u32) -> Result<(), AppError> {
    let now = ctx.now();
    let deliveries = WebhookDeliveryStore::of(ctx);
    let stalled = deliveries
        .stalled(
            now - Duration::minutes(STALLED_AFTER_MINUTES),
            STALLED_BATCH,
        )
        .await?;
    let mut requeued = 0u32;
    for delivery in &stalled {
        if let EnqueueOutcome::Enqueued(_) =
            ctx.jobs().enqueue_now(deliver_job(delivery, now)).await?
        {
            requeued += 1;
        }
    }
    let before = now - Duration::days(i64::from(retention_days));
    let mut purged = 0u64;
    for _ in 0..PURGE_MAX_BATCHES {
        let removed = deliveries.purge(before, PURGE_BATCH).await?;
        purged += removed;
        if removed < u64::from(PURGE_BATCH) {
            break;
        }
    }
    if requeued > 0 || purged > 0 {
        tracing::info!(requeued, purged, "webhooks maintenance finished");
    }
    Ok(())
}
