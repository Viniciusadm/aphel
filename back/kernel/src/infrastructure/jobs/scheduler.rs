use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::application::jobs::{CronSchedule, ScheduledJobDefinition};
use crate::application::shared::{EnqueueOutcome, Job};
use crate::errors::AppError;
use crate::infrastructure::jobs::queue::insert_job;
use crate::infrastructure::jobs::shutdown::Shutdown;
use crate::infrastructure::repositories::mapper;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TickReport {
    pub(crate) enqueued: usize,
    pub(crate) duplicates: usize,
    pub(crate) disabled: usize,
}

#[derive(Clone)]
pub(crate) struct JobScheduler {
    pool: MySqlPool,
}

impl JobScheduler {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) async fn ensure(
        &self,
        definitions: &[ScheduledJobDefinition],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for definition in definitions {
            let next_run_at = definition.cron.next_after(now).ok_or_else(|| {
                AppError::Validation(format!(
                    "cron expression {:?} never fires",
                    definition.cron.expression()
                ))
            })?;
            sqlx::query(
                "INSERT INTO scheduled_jobs (id, kind, cron, payload, queue, max_attempts, next_run_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?) AS incoming \
                 ON DUPLICATE KEY UPDATE \
                   next_run_at = IF(scheduled_jobs.cron = incoming.cron, scheduled_jobs.next_run_at, incoming.next_run_at), \
                   cron = incoming.cron, \
                   payload = incoming.payload, \
                   queue = incoming.queue, \
                   max_attempts = incoming.max_attempts",
            )
            .bind(Uuid::now_v7().to_string())
            .bind(&definition.kind)
            .bind(definition.cron.expression())
            .bind(Json(&definition.payload))
            .bind(&definition.queue)
            .bind(definition.max_attempts)
            .bind(mapper::naive(next_run_at))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn tick(&self, now: DateTime<Utc>) -> Result<TickReport, AppError> {
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT id, kind, cron, payload, queue, max_attempts, next_run_at FROM scheduled_jobs \
             WHERE enabled = 1 AND next_run_at <= ? \
             ORDER BY next_run_at FOR UPDATE SKIP LOCKED",
        )
        .bind(mapper::naive(now))
        .fetch_all(&mut *transaction)
        .await?;

        let mut report = TickReport::default();
        for row in rows {
            let id: String = row.try_get("id")?;
            let kind: String = row.try_get("kind")?;
            let expression: String = row.try_get("cron")?;
            let due_at = mapper::utc(row.try_get::<NaiveDateTime, _>("next_run_at")?);

            let Some(next_run_at) = CronSchedule::parse(&expression)
                .ok()
                .and_then(|schedule| schedule.next_after(now))
            else {
                warn!(kind = %kind, cron = %expression, "disabling scheduled job with an invalid cron expression");
                sqlx::query("UPDATE scheduled_jobs SET enabled = 0 WHERE id = ?")
                    .bind(&id)
                    .execute(&mut *transaction)
                    .await?;
                report.disabled += 1;
                continue;
            };

            let Json(payload): Json<Value> = row.try_get("payload")?;
            let job = Job::new(kind.clone(), payload)
                .on_queue(row.try_get::<String, _>("queue")?)
                .with_max_attempts(row.try_get("max_attempts")?)
                .with_unique_key(ScheduledJobDefinition::unique_key_for(&kind, due_at));
            match insert_job(&mut *transaction, job).await? {
                EnqueueOutcome::Enqueued(_) => report.enqueued += 1,
                EnqueueOutcome::Duplicate => report.duplicates += 1,
            }

            sqlx::query("UPDATE scheduled_jobs SET last_run_at = ?, next_run_at = ? WHERE id = ?")
                .bind(mapper::naive(now))
                .bind(mapper::naive(next_run_at))
                .bind(&id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(report)
    }

    pub(crate) async fn run(&self, interval: Duration, mut shutdown: Shutdown) {
        info!(
            interval_ms = interval.as_millis() as u64,
            "job scheduler started"
        );
        loop {
            if let Err(failure) = self.tick(Utc::now()).await {
                error!("job scheduler tick failed: {failure}");
            }
            tokio::select! {
                () = shutdown.wait() => break,
                () = tokio::time::sleep(interval) => {}
            }
        }
        info!("job scheduler stopped");
    }
}
