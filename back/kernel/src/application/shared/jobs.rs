use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::AppError;

pub const DEFAULT_QUEUE: &str = "default";
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: Uuid,
    pub queue: String,
    pub kind: String,
    pub payload: Value,
    pub run_at: Option<DateTime<Utc>>,
    pub max_attempts: u32,
    pub unique_key: Option<String>,
}

impl Job {
    pub fn new(kind: impl Into<String>, payload: Value) -> Self {
        Self {
            id: Uuid::now_v7(),
            queue: DEFAULT_QUEUE.to_owned(),
            kind: kind.into(),
            payload,
            run_at: None,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            unique_key: None,
        }
    }

    pub fn on_queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = queue.into();
        self
    }

    pub fn run_at(mut self, run_at: DateTime<Utc>) -> Self {
        self.run_at = Some(run_at);
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }

    pub fn with_unique_key(mut self, unique_key: impl Into<String>) -> Self {
        self.unique_key = Some(unique_key.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    Enqueued(Uuid),
    Duplicate,
}

#[async_trait]
pub trait JobQueue<Tx: Send>: Send + Sync {
    async fn enqueue(&self, tx: &mut Tx, job: Job) -> Result<EnqueueOutcome, AppError>;
    async fn enqueue_now(&self, job: Job) -> Result<EnqueueOutcome, AppError>;
}

#[async_trait]
impl<Tx, Q> JobQueue<Tx> for Arc<Q>
where
    Tx: Send,
    Q: JobQueue<Tx> + ?Sized,
{
    async fn enqueue(&self, tx: &mut Tx, job: Job) -> Result<EnqueueOutcome, AppError> {
        (**self).enqueue(tx, job).await
    }

    async fn enqueue_now(&self, job: Job) -> Result<EnqueueOutcome, AppError> {
        (**self).enqueue_now(job).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobContext {
    pub job_id: Uuid,
    pub queue: String,
    pub kind: String,
    pub attempt: u32,
    pub max_attempts: u32,
}

#[async_trait]
pub trait JobHandler: Send + Sync {
    fn kind(&self) -> &str;
    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError>;
}
