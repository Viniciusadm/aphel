use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::outgoing::delivery::{
    OUTGOING_WEBHOOKS_CONSUMER, WEBHOOKS_MAINTENANCE_JOB, deliver, fan_out, run_maintenance,
    webhooks_maintenance_schedule,
};
use crate::outgoing::renderer::ApiPublicEventRenderer;
use crate::outgoing::service::WEBHOOK_DELIVER_JOB;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_sdk::Runtime;
use aphel_sdk::platform::application::public_events::{SharedPublicEventRenderer, is_public_event};
use aphel_sdk::platform::application::shared::{
    DomainEvent, EventConsumer, JobContext, JobHandler,
};
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) fn outgoing_webhook_worker(env: &WorkerEnv) -> WorkerParts {
    let state = env.runtime();
    if let Err(error) = state.outbound_http_network() {
        tracing::error!("outgoing webhooks disabled: {error}");
        return WorkerParts::default();
    }
    let renderer = ApiPublicEventRenderer::shared(env.pool.clone());
    WorkerParts::default()
        .consumer(Arc::new(WebhookFanOutConsumer {
            state: state.clone(),
            renderer,
        }))
        .handler(Arc::new(WebhookDeliverHandler {
            state: state.clone(),
        }))
        .handler(Arc::new(WebhooksMaintenanceHandler { state }))
        .schedule(webhooks_maintenance_schedule())
}

pub(crate) struct WebhookFanOutConsumer {
    state: Runtime,
    renderer: SharedPublicEventRenderer,
}

#[async_trait]
impl EventConsumer for WebhookFanOutConsumer {
    fn name(&self) -> &str {
        OUTGOING_WEBHOOKS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        is_public_event(event_type) || aphel_sdk::platform::modules::app().contributions::<dyn aphel_sdk::platform::application::public_events::RuntimePublicEventSource>().iter().any(|source| source.recognizes(event_type))
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.state.ctx(&scope).await?;
        fan_out(&ctx, &self.renderer, event).await
    }
}

pub(crate) struct WebhookDeliverHandler {
    state: Runtime,
}

#[async_trait]
impl JobHandler for WebhookDeliverHandler {
    fn kind(&self) -> &str {
        WEBHOOK_DELIVER_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.state.ctx(&scope).await?;
        deliver(&ctx, payload).await
    }
}

pub(crate) struct WebhooksMaintenanceHandler {
    state: Runtime,
}

#[async_trait]
impl JobHandler for WebhooksMaintenanceHandler {
    fn kind(&self) -> &str {
        WEBHOOKS_MAINTENANCE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let retention_days =
            crate::settings::delivery_settings(&self.state.configuration()).retention_days;
        let scope = AccessScope::system();
        let ctx = self.state.ctx(&scope).await?;
        run_maintenance(&ctx, retention_days).await
    }
}
