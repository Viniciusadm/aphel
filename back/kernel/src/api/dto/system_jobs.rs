use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::system::{FailedJob, JobKindCounts, QueueOverview};

#[derive(Debug, Serialize, ToSchema)]
pub struct QueueStatsResponse {
    #[schema(example = "default")]
    pub queue: String,
    pub pending: u64,
    pub scheduled: u64,
    pub running: u64,
    pub failed: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobKindStatsResponse {
    #[schema(example = "default")]
    pub queue: String,
    #[schema(example = "outbox.dispatch")]
    pub kind: String,
    pub pending: u64,
    pub scheduled: u64,
    pub running: u64,
    pub failed: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobStatsResponse {
    pub pending: u64,
    pub scheduled: u64,
    pub running: u64,
    pub failed: u64,
    #[schema(format = DateTime)]
    pub oldest_pending_run_at: Option<String>,
    pub oldest_pending_age_seconds: Option<i64>,
    pub queues: Vec<QueueStatsResponse>,
    pub kinds: Vec<JobKindStatsResponse>,
    #[schema(format = DateTime)]
    pub generated_at: String,
}

impl From<JobKindCounts> for JobKindStatsResponse {
    fn from(counts: JobKindCounts) -> Self {
        Self {
            queue: counts.queue,
            kind: counts.kind,
            pending: counts.ready,
            scheduled: counts.scheduled,
            running: counts.running,
            failed: counts.failed,
        }
    }
}

impl From<QueueOverview> for JobStatsResponse {
    fn from(overview: QueueOverview) -> Self {
        let mut queues: BTreeMap<String, QueueStatsResponse> = BTreeMap::new();
        for entry in &overview.by_kind {
            let queue = queues
                .entry(entry.queue.clone())
                .or_insert_with(|| QueueStatsResponse {
                    queue: entry.queue.clone(),
                    pending: 0,
                    scheduled: 0,
                    running: 0,
                    failed: 0,
                });
            queue.pending += entry.ready;
            queue.scheduled += entry.scheduled;
            queue.running += entry.running;
            queue.failed += entry.failed;
        }
        Self {
            pending: overview.totals.ready,
            scheduled: overview.totals.scheduled,
            running: overview.totals.running,
            failed: overview.totals.failed,
            oldest_pending_run_at: overview.oldest_ready_at.map(timestamp),
            oldest_pending_age_seconds: overview.oldest_ready_age_seconds,
            queues: queues.into_values().collect(),
            kinds: overview.by_kind.into_iter().map(Into::into).collect(),
            generated_at: timestamp(overview.generated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FailedJobResponse {
    pub id: String,
    pub queue: String,
    #[schema(example = "webhook.deliver")]
    pub kind: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    #[schema(format = DateTime)]
    pub failed_at: String,
    #[schema(format = DateTime)]
    pub created_at: String,
}

impl From<FailedJob> for FailedJobResponse {
    fn from(job: FailedJob) -> Self {
        Self {
            id: job.id.to_string(),
            queue: job.queue,
            kind: job.kind,
            attempts: job.attempts,
            max_attempts: job.max_attempts,
            last_error: job.last_error,
            failed_at: timestamp(job.failed_at),
            created_at: timestamp(job.created_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FailedJobsQuery {
    #[param(example = "webhook.deliver")]
    pub kind: Option<String>,
}

pub fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
