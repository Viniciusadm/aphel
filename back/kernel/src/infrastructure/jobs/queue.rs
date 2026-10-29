use async_trait::async_trait;
use sqlx::types::Json;
use sqlx::{Executor, MySql, MySqlPool};

use crate::application::shared::{EnqueueOutcome, Job, JobQueue};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

pub(crate) const MAX_QUEUE_LENGTH: usize = 64;
pub(crate) const MAX_KIND_LENGTH: usize = 150;
pub(crate) const MAX_UNIQUE_KEY_LENGTH: usize = 191;

#[derive(Clone)]
pub struct MySqlJobQueue {
    pool: MySqlPool,
}

impl MySqlJobQueue {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JobQueue<MySqlTx> for MySqlJobQueue {
    async fn enqueue(&self, tx: &mut MySqlTx, job: Job) -> Result<EnqueueOutcome, AppError> {
        insert_job(&mut **tx, job).await
    }

    async fn enqueue_now(&self, job: Job) -> Result<EnqueueOutcome, AppError> {
        insert_job(&self.pool, job).await
    }
}

pub(crate) async fn insert_job<'e, E>(executor: E, job: Job) -> Result<EnqueueOutcome, AppError>
where
    E: Executor<'e, Database = MySql>,
{
    validate(&job)?;
    let result = sqlx::query(
        "INSERT INTO jobs (id, queue, kind, payload, max_attempts, run_at, unique_key) \
         VALUES (?, ?, ?, ?, ?, COALESCE(?, UTC_TIMESTAMP(6)), ?)",
    )
    .bind(job.id.to_string())
    .bind(&job.queue)
    .bind(&job.kind)
    .bind(Json(&job.payload))
    .bind(job.max_attempts.max(1))
    .bind(job.run_at.map(mapper::naive))
    .bind(job.unique_key.as_deref())
    .execute(executor)
    .await;

    match result {
        Ok(_) => Ok(EnqueueOutcome::Enqueued(job.id)),
        Err(sqlx::Error::Database(error))
            if error.is_unique_violation() && job.unique_key.is_some() =>
        {
            Ok(EnqueueOutcome::Duplicate)
        }
        Err(error) => Err(error.into()),
    }
}

fn validate(job: &Job) -> Result<(), AppError> {
    let within = |value: &str, max: usize| !value.trim().is_empty() && value.len() <= max;
    if !within(&job.queue, MAX_QUEUE_LENGTH) {
        return Err(AppError::Validation(format!(
            "job queue must have between 1 and {MAX_QUEUE_LENGTH} characters"
        )));
    }
    if !within(&job.kind, MAX_KIND_LENGTH) {
        return Err(AppError::Validation(format!(
            "job kind must have between 1 and {MAX_KIND_LENGTH} characters"
        )));
    }
    if job
        .unique_key
        .as_deref()
        .is_some_and(|key| !within(key, MAX_UNIQUE_KEY_LENGTH))
    {
        return Err(AppError::Validation(format!(
            "job unique key must have between 1 and {MAX_UNIQUE_KEY_LENGTH} characters"
        )));
    }
    Ok(())
}
