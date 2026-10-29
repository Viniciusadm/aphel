use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::MySqlPool;

use crate::incoming::IncomingWebhookStore;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{JobContext, JobHandler};
use aphel_kernel::errors::AppError;

pub(crate) const INCOMING_WEBHOOKS_PURGE_JOB: &str = "incoming_webhooks.purge";
pub(crate) const INCOMING_WEBHOOKS_PURGE_CRON: &str = "0 25 * * * *";
pub(crate) const REPLAY_RETENTION_HOURS: i64 = 24;
pub(crate) const REQUEST_LOG_RETENTION_DAYS: i64 = 30;

pub(crate) fn incoming_webhooks_purge_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(INCOMING_WEBHOOKS_PURGE_JOB, INCOMING_WEBHOOKS_PURGE_CRON)
        .expect("the incoming webhooks purge cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) fn incoming_webhooks_purge_handler(pool: MySqlPool) -> Arc<dyn JobHandler> {
    Arc::new(IncomingWebhooksPurgeHandler {
        store: IncomingWebhookStore::new(pool),
    })
}

pub(crate) struct IncomingWebhooksPurgeHandler {
    store: IncomingWebhookStore,
}

#[async_trait]
impl JobHandler for IncomingWebhooksPurgeHandler {
    fn kind(&self) -> &str {
        INCOMING_WEBHOOKS_PURGE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let now = Utc::now();
        let replays = self
            .store
            .purge_replays(now - Duration::hours(REPLAY_RETENTION_HOURS))
            .await?;
        let requests = self
            .store
            .purge_requests(now - Duration::days(REQUEST_LOG_RETENTION_DAYS))
            .await?;
        tracing::info!(replays, requests, "incoming webhooks purge finished");
        Ok(())
    }
}
