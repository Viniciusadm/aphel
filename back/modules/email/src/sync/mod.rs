pub(crate) mod jobs;
pub(crate) mod model;

pub(crate) use model::*;

use serde_json::json;
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::Job;

pub(crate) const EMAIL_SYNC_JOB: &str = "email.sync";
pub(crate) const EMAIL_SYNC_MAILBOX_JOB: &str = "email.sync.mailbox";
pub(crate) const EMAIL_SYNC_CRON: &str = "0 * * * * *";
pub(crate) const DEFAULT_SYNC_INTERVAL_MINUTES: u32 = 5;
pub(crate) const DEFAULT_SYNC_BATCH_SIZE: usize = 50;
pub(crate) const DEFAULT_SYNC_MAX_BATCHES: u32 = 10;
pub(crate) const DEFAULT_SYNC_DISPATCH_LIMIT: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyncSettings {
    pub(crate) interval_minutes: u32,
    pub(crate) batch_size: usize,
    pub(crate) max_batches: u32,
    pub(crate) dispatch_limit: u32,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            interval_minutes: DEFAULT_SYNC_INTERVAL_MINUTES,
            batch_size: DEFAULT_SYNC_BATCH_SIZE,
            max_batches: DEFAULT_SYNC_MAX_BATCHES,
            dispatch_limit: DEFAULT_SYNC_DISPATCH_LIMIT,
        }
    }
}

pub(crate) fn email_sync_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(EMAIL_SYNC_JOB, EMAIL_SYNC_CRON)
        .expect("the e-mail sync cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) fn sync_unique_key(mailbox_id: Uuid) -> String {
    format!("{EMAIL_SYNC_MAILBOX_JOB}:{mailbox_id}")
}

pub(crate) fn sync_mailbox_job(mailbox_id: Uuid, owner: &UserId) -> Job {
    Job::new(
        EMAIL_SYNC_MAILBOX_JOB,
        json!({
            "mailbox_id": mailbox_id,
            "user_id": owner.as_str(),
        }),
    )
    .with_max_attempts(3)
    .with_unique_key(sync_unique_key(mailbox_id))
}
