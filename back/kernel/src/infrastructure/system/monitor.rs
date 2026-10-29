use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{AssertSqlSafe, MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::system::model::{
    FailedJob, FailedJobFilter, FailureEntry, FailureSource, InstanceHeartbeat, JobKindCounts,
    PageWindow,
};
use crate::application::system::ports::{FailedJobRepository, QueueMonitor};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::system::probe;

const MAX_ERROR_CHARS: usize = 500;

const FAILED_JOB_COLUMNS: &str = "j.id, j.queue, j.kind, \
     j.attempts, j.max_attempts, j.last_error, j.failed_at, j.created_at";

#[derive(Clone)]
pub struct MySqlQueueMonitor {
    pool: MySqlPool,
}

impl MySqlQueueMonitor {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn count(row: &MySqlRow, column: &str) -> Result<u64, AppError> {
    let value: Option<i64> = row.try_get(column)?;
    Ok(value.unwrap_or(0).max(0) as u64)
}

fn truncate(value: Option<String>) -> Option<String> {
    value.map(|text| {
        if text.chars().count() > MAX_ERROR_CHARS {
            let mut short: String = text.chars().take(MAX_ERROR_CHARS).collect();
            short.push('…');
            short
        } else {
            text
        }
    })
}

fn failed_job(row: &MySqlRow) -> Result<FailedJob, AppError> {
    let id: String = row.try_get("id")?;
    Ok(FailedJob {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("jobs.id", &id))?,
        queue: row.try_get("queue")?,
        kind: row.try_get("kind")?,
        attempts: row.try_get("attempts")?,
        max_attempts: row.try_get("max_attempts")?,
        last_error: truncate(row.try_get("last_error")?),
        failed_at: mapper::utc(row.try_get::<NaiveDateTime, _>("failed_at")?),
        created_at: mapper::utc(row.try_get::<NaiveDateTime, _>("created_at")?),
    })
}

pub async fn job_counts(
    pool: &MySqlPool,
    now: DateTime<Utc>,
) -> Result<Vec<JobKindCounts>, AppError> {
    let now = mapper::naive(now);
    let rows = sqlx::query(
        "SELECT queue, kind, \
           CAST(SUM(failed_at IS NULL AND locked_at IS NULL AND run_at <= ?) AS SIGNED) AS ready, \
           CAST(SUM(failed_at IS NULL AND locked_at IS NULL AND run_at > ?) AS SIGNED) AS scheduled, \
           CAST(SUM(failed_at IS NULL AND locked_at IS NOT NULL) AS SIGNED) AS running, \
           CAST(SUM(failed_at IS NOT NULL) AS SIGNED) AS failed \
         FROM jobs WHERE finished_at IS NULL GROUP BY queue, kind ORDER BY queue, kind",
    )
    .bind(now)
    .bind(now)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(JobKindCounts {
                queue: row.try_get("queue")?,
                kind: row.try_get("kind")?,
                ready: count(row, "ready")?,
                scheduled: count(row, "scheduled")?,
                running: count(row, "running")?,
                failed: count(row, "failed")?,
            })
        })
        .collect()
}

pub async fn heartbeats(pool: &MySqlPool) -> Result<Vec<InstanceHeartbeat>, AppError> {
    let rows = sqlx::query(
        "SELECT instance_id, role, hostname, version, started_at, last_seen_at \
         FROM system_heartbeats ORDER BY last_seen_at DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(InstanceHeartbeat {
                instance_id: row.try_get("instance_id")?,
                role: row.try_get("role")?,
                hostname: row.try_get("hostname")?,
                version: row.try_get("version")?,
                started_at: mapper::utc(row.try_get::<NaiveDateTime, _>("started_at")?),
                last_seen_at: mapper::utc(row.try_get::<NaiveDateTime, _>("last_seen_at")?),
            })
        })
        .collect()
}

fn failure(row: &MySqlRow, source: FailureSource) -> Result<FailureEntry, AppError> {
    let status: Option<u16> = match source {
        FailureSource::Webhook => row.try_get("response_status")?,
        _ => None,
    };
    Ok(FailureEntry {
        source,
        reference_id: row.try_get("reference_id")?,
        kind: row.try_get("kind")?,
        error: truncate(row.try_get("error")?),
        response_status: status,
        occurred_at: mapper::utc(row.try_get::<NaiveDateTime, _>("occurred_at")?),
    })
}

#[async_trait]
impl QueueMonitor for MySqlQueueMonitor {
    async fn job_counts(&self, now: DateTime<Utc>) -> Result<Vec<JobKindCounts>, AppError> {
        job_counts(&self.pool, now).await
    }

    async fn oldest_ready_job(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        probe::oldest_ready_job(&self.pool, now).await
    }

    async fn failed_jobs(
        &self,
        filter: &FailedJobFilter,
        window: PageWindow,
    ) -> Result<(Vec<FailedJob>, u64), AppError> {
        let mut total = QueryBuilder::<MySql>::new(
            "SELECT COUNT(*) FROM jobs j WHERE j.finished_at IS NULL AND j.failed_at IS NOT NULL",
        );
        if let Some(kind) = &filter.kind {
            total.push(" AND j.kind = ").push_bind(kind);
        }
        let total: i64 = total.build_query_scalar().fetch_one(&self.pool).await?;

        let mut query = QueryBuilder::<MySql>::new(format!(
            "SELECT {FAILED_JOB_COLUMNS} FROM jobs j \
             WHERE j.finished_at IS NULL AND j.failed_at IS NOT NULL"
        ));
        if let Some(kind) = &filter.kind {
            query.push(" AND j.kind = ").push_bind(kind);
        }
        query
            .push(" ORDER BY j.failed_at DESC, j.id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let rows = query.build().fetch_all(&self.pool).await?;
        let jobs = rows.iter().map(failed_job).collect::<Result<_, _>>()?;
        Ok((jobs, total.max(0) as u64))
    }

    async fn recent_failures(
        &self,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError> {
        let since = mapper::naive(since);
        let mut entries = Vec::new();
        let automations = sqlx::query(
            "SELECT r.automation_id AS reference_id, \
               r.trigger_type AS kind, r.error, COALESCE(r.finished_at, r.started_at) AS occurred_at \
             FROM automation_runs r \
             WHERE r.status = 'failed' AND r.started_at >= ? \
             ORDER BY r.started_at DESC LIMIT ?",
        )
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        for row in &automations {
            entries.push(failure(row, FailureSource::Automation)?);
        }
        let webhooks = sqlx::query(
            "SELECT d.endpoint_id AS reference_id, \
               d.event_type AS kind, d.error, d.response_status, \
               COALESCE(d.completed_at, d.last_attempt_at, d.created_at) AS occurred_at \
             FROM webhook_deliveries d \
             WHERE d.status = 'failed' AND d.created_at >= ? \
             ORDER BY d.created_at DESC LIMIT ?",
        )
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        for row in &webhooks {
            entries.push(failure(row, FailureSource::Webhook)?);
        }
        let integrations = sqlx::query(
            "SELECT i.id AS reference_id, \
               i.provider_key AS kind, i.last_error AS error, i.last_error_at AS occurred_at \
             FROM integrations i \
             WHERE i.status = 'error' AND i.last_error_at IS NOT NULL AND i.last_error_at >= ? \
             ORDER BY i.last_error_at DESC LIMIT ?",
        )
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        for row in &integrations {
            entries.push(failure(row, FailureSource::Integration)?);
        }
        Ok(entries)
    }

    async fn heartbeats(&self) -> Result<Vec<InstanceHeartbeat>, AppError> {
        heartbeats(&self.pool).await
    }
}

#[derive(Clone, Default)]
pub struct MySqlFailedJobRepository;

#[async_trait]
impl FailedJobRepository for MySqlFailedJobRepository {
    type Tx = MySqlTx;

    async fn lock_failed(&self, tx: &mut MySqlTx, id: Uuid) -> Result<Option<FailedJob>, AppError> {
        let row = sqlx::query(AssertSqlSafe(format!(
            "SELECT {FAILED_JOB_COLUMNS} FROM jobs j \
             WHERE j.id = ? AND j.finished_at IS NULL AND j.failed_at IS NOT NULL FOR UPDATE OF j"
        )))
        .bind(id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        row.as_ref().map(failed_job).transpose()
    }

    async fn requeue(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE jobs SET failed_at = NULL, attempts = 0, run_at = ?, locked_at = NULL, locked_by = NULL \
             WHERE id = ? AND failed_at IS NOT NULL",
        )
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM jobs WHERE id = ? AND failed_at IS NOT NULL")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
