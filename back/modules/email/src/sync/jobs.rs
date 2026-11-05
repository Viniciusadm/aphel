use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::linking::EmailEngagementConsumer;
use crate::service;
use crate::sync::{
    EMAIL_SYNC_JOB, EMAIL_SYNC_MAILBOX_JOB, SyncMailboxRequest, SyncSettings, email_sync_schedule,
};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::WorkerParts;
use aphel_sdk::Runtime;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const SYNC_INTERVAL_ENV: &str = "EMAIL_SYNC_INTERVAL_MINUTES";
pub(crate) const SYNC_BATCH_SIZE_ENV: &str = "EMAIL_SYNC_BATCH_SIZE";
pub(crate) const SYNC_MAX_BATCHES_ENV: &str = "EMAIL_SYNC_MAX_BATCHES";

fn env_number<T: std::str::FromStr + PartialOrd>(key: &str, min: T, max: T) -> Option<T> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<T>().ok())
        .filter(|value| *value >= min && *value <= max)
}

pub(crate) fn sync_settings_from_env() -> SyncSettings {
    let defaults = SyncSettings::default();
    SyncSettings {
        interval_minutes: env_number(SYNC_INTERVAL_ENV, 1, 1440)
            .unwrap_or(defaults.interval_minutes),
        batch_size: env_number(SYNC_BATCH_SIZE_ENV, 1, 500).unwrap_or(defaults.batch_size),
        max_batches: env_number(SYNC_MAX_BATCHES_ENV, 1, 100).unwrap_or(defaults.max_batches),
        dispatch_limit: defaults.dispatch_limit,
    }
}

pub(crate) struct EmailSyncDispatchHandler {
    state: Runtime,
    settings: SyncSettings,
}

#[async_trait]
impl JobHandler for EmailSyncDispatchHandler {
    fn kind(&self) -> &str {
        EMAIL_SYNC_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.state.ctx(&scope).await?;
        let queued = service::sync::dispatch(&ctx, self.settings).await?;
        if queued > 0 {
            tracing::debug!(queued, "queued mailbox synchronizations");
        }
        Ok(())
    }
}

pub(crate) struct EmailSyncMailboxHandler {
    state: Runtime,
    settings: SyncSettings,
}

fn payload_text<'a>(payload: &'a Value, key: &str) -> Result<&'a str, AppError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation(format!("e-mail sync payload is missing {key}")))
}

#[async_trait]
impl JobHandler for EmailSyncMailboxHandler {
    fn kind(&self) -> &str {
        EMAIL_SYNC_MAILBOX_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let mailbox_id = Uuid::parse_str(payload_text(&payload, "mailbox_id")?)
            .map_err(|_| AppError::Validation("invalid mailbox id".to_owned()))?;
        let owner = UserId::new(payload_text(&payload, "user_id")?)?;
        let scope = AccessScope::system();
        let ctx = self.state.ctx(&scope).await?;
        let outcome = service::sync::sync_mailbox(
            &ctx,
            self.settings,
            &SyncMailboxRequest { mailbox_id, owner },
        )
        .await?;
        tracing::debug!(
            %mailbox_id,
            imported = outcome.imported,
            skipped = outcome.skipped,
            "mailbox synchronized"
        );
        Ok(())
    }
}

pub(crate) fn worker(state: Runtime) -> WorkerParts {
    let settings = sync_settings_from_env();
    WorkerParts::default()
        .handler(Arc::new(EmailSyncDispatchHandler {
            state: state.clone(),
            settings,
        }))
        .handler(Arc::new(EmailSyncMailboxHandler {
            state: state.clone(),
            settings,
        }))
        .consumer(Arc::new(EmailEngagementConsumer::new(state)))
        .schedule(email_sync_schedule())
}
