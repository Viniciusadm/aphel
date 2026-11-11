use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::application::jobs::{JobRegistry, RetryPolicy};
use crate::application::shared::{JobContext, JobHandler};
use crate::config::WorkerConfig;
use crate::errors::AppError;
use crate::infrastructure::jobs::shutdown::Shutdown;
use crate::infrastructure::jobs::signal::{jobs_available, wake_workers};

pub(crate) const MAX_ERROR_LENGTH: usize = 2000;
pub(crate) const LOCK_GRACE_PERIOD: Duration = Duration::from_secs(60);
const MAX_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(60);
const MAX_IDLE_POLL_INTERVAL: Duration = Duration::from_secs(5);
const HELPER_IDLE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopRole {
    Poller,
    Helper,
}
const LOST_LOCK_ERROR: &str =
    "the worker holding this job stopped before finishing it (lock expired)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerSettings {
    pub(crate) worker_id: String,
    pub(crate) concurrency: usize,
    pub(crate) poll_interval: Duration,
    pub(crate) job_timeout: Duration,
    pub(crate) lock_timeout: Duration,
    pub(crate) retry: RetryPolicy,
    pub(crate) queues: Vec<String>,
}

impl WorkerSettings {
    pub(crate) fn from_config(config: &WorkerConfig) -> Self {
        Self {
            worker_id: generate_worker_id(),
            concurrency: config.concurrency.max(1),
            poll_interval: config.poll_interval,
            job_timeout: config.job_timeout,
            lock_timeout: config.job_timeout + LOCK_GRACE_PERIOD,
            retry: RetryPolicy::default(),
            queues: Vec::new(),
        }
    }
}

pub(crate) fn generate_worker_id() -> String {
    let suffix = Uuid::now_v7().simple().to_string();
    format!(
        "worker-{}-{}",
        std::process::id(),
        &suffix[suffix.len() - 12..]
    )
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReservedJob {
    pub(crate) id: Uuid,
    pub(crate) queue: String,
    pub(crate) kind: String,
    pub(crate) payload: Value,
    pub(crate) attempt: u32,
    pub(crate) max_attempts: u32,
}

impl ReservedJob {
    pub(crate) fn context(&self) -> JobContext {
        JobContext {
            job_id: self.id,
            queue: self.queue.clone(),
            kind: self.kind.clone(),
            attempt: self.attempt,
            max_attempts: self.max_attempts,
        }
    }

    fn from_row(row: &MySqlRow) -> Result<Self, AppError> {
        let id: String = row.try_get("id")?;
        let attempts: u32 = row.try_get("attempts")?;
        let Json(payload): Json<Value> = row.try_get("payload")?;
        Ok(Self {
            id: Uuid::parse_str(&id)
                .map_err(|_| AppError::Persistence(format!("invalid job id {id:?}")))?,
            queue: row.try_get("queue")?,
            kind: row.try_get("kind")?,
            payload,
            attempt: attempts + 1,
            max_attempts: row.try_get("max_attempts")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JobOutcome {
    Completed,
    Retrying { delay: Duration, error: String },
    Failed { error: String },
}

#[derive(Clone)]
pub struct JobWorker {
    pool: MySqlPool,
    registry: Arc<JobRegistry>,
    settings: Arc<WorkerSettings>,
}

impl JobWorker {
    pub(crate) fn new(pool: MySqlPool, registry: JobRegistry, settings: WorkerSettings) -> Self {
        Self {
            pool,
            registry: Arc::new(registry),
            settings: Arc::new(settings),
        }
    }

    pub(crate) async fn run(&self, shutdown: Shutdown) -> Result<(), AppError> {
        info!(
            worker_id = %self.settings.worker_id,
            concurrency = self.settings.concurrency,
            handlers = ?self.registry.kinds().collect::<Vec<_>>(),
            "job worker started"
        );

        let mut tasks = JoinSet::new();
        for index in 0..self.settings.concurrency {
            let worker = self.clone();
            let shutdown = shutdown.clone();
            let role = if index == 0 {
                LoopRole::Poller
            } else {
                LoopRole::Helper
            };
            tasks.spawn(crate::tenancy::bind(async move {
                worker.process_until(shutdown, role).await
            }));
        }
        let worker = self.clone();
        let maintenance_shutdown = shutdown.clone();
        tasks.spawn(crate::tenancy::bind(async move {
            worker.recover_until(maintenance_shutdown).await
        }));

        while let Some(result) = tasks.join_next().await {
            if let Err(join_error) = result {
                error!("job worker task stopped unexpectedly: {join_error}");
            }
        }

        info!(worker_id = %self.settings.worker_id, "job worker stopped");
        Ok(())
    }

    pub(crate) async fn run_shared(
        &self,
        shutdown: Shutdown,
        slots: Arc<Semaphore>,
    ) -> Result<(), AppError> {
        info!(
            worker_id = %self.settings.worker_id,
            handlers = ?self.registry.kinds().collect::<Vec<_>>(),
            "job worker started on the shared slots"
        );
        let worker = self.clone();
        let maintenance_shutdown = shutdown.clone();
        let maintenance =
            crate::tenancy::spawn(async move { worker.recover_until(maintenance_shutdown).await });
        self.process_shared(shutdown, slots).await;
        if let Err(join_error) = maintenance.await {
            error!("job lock recovery stopped unexpectedly: {join_error}");
        }
        info!(worker_id = %self.settings.worker_id, "job worker stopped");
        Ok(())
    }

    async fn process_shared(&self, mut shutdown: Shutdown, slots: Arc<Semaphore>) {
        let busiest_wait = self.settings.poll_interval.max(MAX_IDLE_POLL_INTERVAL);
        let mut idle_wait = self.settings.poll_interval;
        let signal = jobs_available().unwrap_or_default();
        let mut running = JoinSet::new();
        loop {
            while let Some(result) = running.try_join_next() {
                if let Err(join_error) = result {
                    error!("job task stopped unexpectedly: {join_error}");
                }
            }
            if shutdown.is_triggered() {
                break;
            }
            let woken = signal.notified();
            tokio::pin!(woken);
            woken.as_mut().enable();
            let permit = tokio::select! {
                () = shutdown.wait() => break,
                permit = slots.clone().acquire_owned() => match permit {
                    Ok(permit) => permit,
                    Err(_) => break,
                },
            };
            match self.reserve().await {
                Ok(Some(job)) => {
                    idle_wait = self.settings.poll_interval;
                    let worker = self.clone();
                    running.spawn(crate::tenancy::bind(async move {
                        let _permit = permit;
                        if let Err(failure) = worker.execute(job).await {
                            error!("job worker failed to process the queue: {failure}");
                        }
                    }));
                    continue;
                }
                Ok(None) => drop(permit),
                Err(failure) => {
                    drop(permit);
                    error!("job worker failed to process the queue: {failure}");
                }
            }
            tokio::select! {
                () = shutdown.wait() => break,
                () = &mut woken => idle_wait = self.settings.poll_interval,
                () = tokio::time::sleep(idle_wait) => {
                    idle_wait = (idle_wait * 2).min(busiest_wait);
                }
            }
        }
        while let Some(result) = running.join_next().await {
            if let Err(join_error) = result {
                error!("job task stopped unexpectedly: {join_error}");
            }
        }
    }

    pub(crate) async fn reserve(&self) -> Result<Option<ReservedJob>, AppError> {
        let mut query = QueryBuilder::<MySql>::new(
            "SELECT id, queue, kind, payload, attempts, max_attempts FROM jobs \
             WHERE finished_at IS NULL AND failed_at IS NULL AND locked_at IS NULL \
             AND run_at <= UTC_TIMESTAMP(6)",
        );
        if !self.settings.queues.is_empty() {
            query.push(" AND queue IN (");
            let mut queues = query.separated(", ");
            for queue in &self.settings.queues {
                queues.push_bind(queue);
            }
            queues.push_unseparated(")");
        }
        query.push(" ORDER BY run_at, id LIMIT 1 FOR UPDATE SKIP LOCKED");

        let mut transaction = self.pool.begin().await?;
        let Some(row) = query.build().fetch_optional(&mut *transaction).await? else {
            transaction.commit().await?;
            return Ok(None);
        };
        let job = ReservedJob::from_row(&row)?;
        sqlx::query(
            "UPDATE jobs SET locked_at = UTC_TIMESTAMP(6), locked_by = ?, attempts = attempts + 1 \
             WHERE id = ?",
        )
        .bind(&self.settings.worker_id)
        .bind(job.id.to_string())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        debug!(job_id = %job.id, kind = %job.kind, attempt = job.attempt, "job reserved");
        Ok(Some(job))
    }

    pub(crate) async fn execute(&self, job: ReservedJob) -> Result<JobOutcome, AppError> {
        let result = match self.registry.get(&job.kind) {
            Some(handler) => self.invoke(handler, &job).await,
            None => Err(AppError::Internal(format!(
                "no job handler is registered for kind {}",
                job.kind
            ))),
        };

        match result {
            Ok(()) => {
                self.complete(&job).await?;
                debug!(job_id = %job.id, kind = %job.kind, "job finished");
                Ok(JobOutcome::Completed)
            }
            Err(failure) => self.fail(&job, &failure).await,
        }
    }

    pub(crate) async fn recover_expired_locks(&self) -> Result<u64, AppError> {
        let lock_timeout = duration_micros(self.settings.lock_timeout);
        let failed = sqlx::query(
            "UPDATE jobs SET failed_at = UTC_TIMESTAMP(6), locked_at = NULL, locked_by = NULL, \
             unique_key = NULL, last_error = ? \
             WHERE finished_at IS NULL AND failed_at IS NULL AND locked_at IS NOT NULL \
             AND locked_at < UTC_TIMESTAMP(6) - INTERVAL ? MICROSECOND \
             AND attempts >= max_attempts",
        )
        .bind(LOST_LOCK_ERROR)
        .bind(lock_timeout)
        .execute(&self.pool)
        .await?
        .rows_affected();
        let released = sqlx::query(
            "UPDATE jobs SET run_at = UTC_TIMESTAMP(6), locked_at = NULL, locked_by = NULL, \
             last_error = ? \
             WHERE finished_at IS NULL AND failed_at IS NULL AND locked_at IS NOT NULL \
             AND locked_at < UTC_TIMESTAMP(6) - INTERVAL ? MICROSECOND \
             AND attempts < max_attempts",
        )
        .bind(LOST_LOCK_ERROR)
        .bind(lock_timeout)
        .execute(&self.pool)
        .await?
        .rows_affected();

        let recovered = failed + released;
        if recovered > 0 {
            warn!(released, failed, "recovered jobs with expired locks");
        }
        Ok(recovered)
    }

    async fn process_until(&self, mut shutdown: Shutdown, role: LoopRole) {
        let busiest_wait = self.settings.poll_interval.max(MAX_IDLE_POLL_INTERVAL);
        let mut idle_wait = self.settings.poll_interval;
        let signal = jobs_available().unwrap_or_default();
        loop {
            if shutdown.is_triggered() {
                break;
            }
            let woken = signal.notified();
            tokio::pin!(woken);
            woken.as_mut().enable();
            match self.reserve().await {
                Ok(Some(job)) => {
                    wake_workers();
                    idle_wait = self.settings.poll_interval;
                    if let Err(failure) = self.execute(job).await {
                        error!("job worker failed to process the queue: {failure}");
                    }
                    continue;
                }
                Ok(None) => {}
                Err(failure) => error!("job worker failed to process the queue: {failure}"),
            }
            let wait = match role {
                LoopRole::Poller => idle_wait,
                LoopRole::Helper => HELPER_IDLE_INTERVAL,
            };
            tokio::select! {
                () = shutdown.wait() => break,
                () = &mut woken => idle_wait = self.settings.poll_interval,
                () = tokio::time::sleep(wait) => {
                    idle_wait = (idle_wait * 2).min(busiest_wait);
                }
            }
        }
    }

    async fn recover_until(&self, mut shutdown: Shutdown) {
        let interval = self
            .settings
            .lock_timeout
            .min(MAX_MAINTENANCE_INTERVAL)
            .max(Duration::from_secs(1));
        loop {
            if let Err(failure) = self.recover_expired_locks().await {
                error!("failed to recover jobs with expired locks: {failure}");
            }
            tokio::select! {
                () = shutdown.wait() => break,
                () = tokio::time::sleep(interval) => {}
            }
        }
    }

    async fn invoke(
        &self,
        handler: Arc<dyn JobHandler>,
        job: &ReservedJob,
    ) -> Result<(), AppError> {
        let context = job.context();
        let payload = job.payload.clone();
        let mut task =
            crate::tenancy::spawn(async move { handler.handle(&context, payload).await });
        match tokio::time::timeout(self.settings.job_timeout, &mut task).await {
            Ok(Ok(result)) => result,
            Ok(Err(join_error)) if join_error.is_panic() => {
                Err(AppError::Internal("job handler panicked".to_owned()))
            }
            Ok(Err(_)) => Err(AppError::Internal("job handler was cancelled".to_owned())),
            Err(_) => {
                task.abort();
                Err(AppError::Internal(format!(
                    "job timed out after {} ms",
                    self.settings.job_timeout.as_millis()
                )))
            }
        }
    }

    async fn complete(&self, job: &ReservedJob) -> Result<(), AppError> {
        let updated = sqlx::query(
            "UPDATE jobs SET finished_at = UTC_TIMESTAMP(6), locked_at = NULL, locked_by = NULL, \
             unique_key = NULL WHERE id = ? AND locked_by = ?",
        )
        .bind(job.id.to_string())
        .bind(&self.settings.worker_id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if updated == 0 {
            warn!(job_id = %job.id, kind = %job.kind, "job finished after its lock was lost");
        }
        Ok(())
    }

    async fn fail(&self, job: &ReservedJob, failure: &AppError) -> Result<JobOutcome, AppError> {
        let error = truncate_error(&failure.to_string());
        if job.attempt >= job.max_attempts {
            sqlx::query(
                "UPDATE jobs SET failed_at = UTC_TIMESTAMP(6), locked_at = NULL, locked_by = NULL, \
                 unique_key = NULL, last_error = ? WHERE id = ? AND locked_by = ?",
            )
            .bind(&error)
            .bind(job.id.to_string())
            .bind(&self.settings.worker_id)
            .execute(&self.pool)
            .await?;
            error!(
                job_id = %job.id,
                kind = %job.kind,
                attempt = job.attempt,
                "job failed permanently: {error}"
            );
            return Ok(JobOutcome::Failed { error });
        }

        let delay = self.settings.retry.delay_after(job.attempt);
        sqlx::query(
            "UPDATE jobs SET run_at = UTC_TIMESTAMP(6) + INTERVAL ? MICROSECOND, locked_at = NULL, \
             locked_by = NULL, last_error = ? WHERE id = ? AND locked_by = ?",
        )
        .bind(duration_micros(delay))
        .bind(&error)
        .bind(job.id.to_string())
        .bind(&self.settings.worker_id)
        .execute(&self.pool)
        .await?;
        warn!(
            job_id = %job.id,
            kind = %job.kind,
            attempt = job.attempt,
            retry_in_ms = delay.as_millis() as u64,
            "job failed and will be retried: {error}"
        );
        Ok(JobOutcome::Retrying { delay, error })
    }
}

pub(crate) fn truncate_error(message: &str) -> String {
    if message.chars().count() <= MAX_ERROR_LENGTH {
        return message.to_owned();
    }
    let mut truncated: String = message.chars().take(MAX_ERROR_LENGTH - 1).collect();
    truncated.push('…');
    truncated
}

fn duration_micros(duration: Duration) -> i64 {
    i64::try_from(duration.as_micros()).unwrap_or(i64::MAX)
}
