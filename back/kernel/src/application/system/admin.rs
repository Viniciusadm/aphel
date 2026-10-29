use chrono::Duration;
use serde_json::json;
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::security::Clock;
use crate::application::shared::TransactionManager;
use crate::application::system::health::age_seconds;
use crate::application::system::model::{
    FailedJob, FailedJobFilter, FailureSource, InstalledModuleInfo, InstanceHeartbeat,
    JobStateTotals, MigrationStatus, PageWindow, QueueOverview, RecentFailures,
    SYSTEM_JOB_DISCARDED, SYSTEM_JOB_RETRIED, SystemAuditEntry,
};
use crate::application::system::ports::{
    FailedJobRepository, MigrationInspector, ModuleInventory, QueueMonitor, SystemAdminPorts,
    SystemAuditLog,
};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, Permission};
use crate::errors::AppError;

pub const FAILURE_WINDOW_DAYS: i64 = 7;
pub const FAILURES_PER_SOURCE: u64 = 50;
pub const JOB_NOT_FOUND: &str = "job_not_found";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemActor {
    pub scope: AccessScope,
    pub user_id: UserId,
    pub ip: Option<String>,
}

pub struct SystemAdminUseCases<P> {
    ports: P,
}

impl<P> SystemAdminUseCases<P>
where
    P: SystemAdminPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    async fn require_admin(&self, actor: &SystemActor) -> Result<(), AppError> {
        actor.scope.require(&Permission::SYSTEM_MANAGE).map(|_| ())
    }

    pub async fn queue_overview(&self, actor: &SystemActor) -> Result<QueueOverview, AppError> {
        self.require_admin(actor).await?;
        let now = self.ports.clock().now();
        let by_kind = self.ports.monitor().job_counts(now).await?;
        let oldest_ready_at = self.ports.monitor().oldest_ready_job(now).await?;
        Ok(QueueOverview {
            totals: JobStateTotals::sum(&by_kind),
            by_kind,
            oldest_ready_at,
            oldest_ready_age_seconds: oldest_ready_at.map(|run_at| age_seconds(now, run_at)),
            generated_at: now,
        })
    }

    pub async fn failed_jobs(
        &self,
        actor: &SystemActor,
        filter: &FailedJobFilter,
        window: PageWindow,
    ) -> Result<(Vec<FailedJob>, u64), AppError> {
        self.require_admin(actor).await?;
        self.ports.monitor().failed_jobs(filter, window).await
    }

    pub async fn retry_job(&self, actor: &SystemActor, job_id: &str) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let id = parse_job_id(job_id)?;
        let now = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        let job = self.locked_failed_job(&mut tx, id).await?;
        self.ports.failed_jobs().requeue(&mut tx, id, now).await?;
        self.ports
            .audit()
            .record(&mut tx, audit_entry(SYSTEM_JOB_RETRIED, actor, &job, now))
            .await?;
        self.ports.transactions().commit(tx).await
    }

    pub async fn discard_job(&self, actor: &SystemActor, job_id: &str) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let id = parse_job_id(job_id)?;
        let now = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        let job = self.locked_failed_job(&mut tx, id).await?;
        self.ports.failed_jobs().delete(&mut tx, id).await?;
        self.ports
            .audit()
            .record(&mut tx, audit_entry(SYSTEM_JOB_DISCARDED, actor, &job, now))
            .await?;
        self.ports.transactions().commit(tx).await
    }

    async fn locked_failed_job(
        &self,
        tx: &mut <P::Transactions as TransactionManager>::Tx,
        id: Uuid,
    ) -> Result<FailedJob, AppError> {
        self.ports
            .failed_jobs()
            .lock_failed(tx, id)
            .await?
            .ok_or_else(job_not_found)
    }

    pub async fn recent_failures(&self, actor: &SystemActor) -> Result<RecentFailures, AppError> {
        self.require_admin(actor).await?;
        let since = self.ports.clock().now() - Duration::days(FAILURE_WINDOW_DAYS);
        let entries = self
            .ports
            .monitor()
            .recent_failures(since, FAILURES_PER_SOURCE)
            .await?;
        let mut failures = RecentFailures::default();
        for entry in entries {
            match entry.source {
                FailureSource::Automation => failures.automations.push(entry),
                FailureSource::Webhook => failures.webhooks.push(entry),
                FailureSource::Integration => failures.integrations.push(entry),
            }
        }
        Ok(failures)
    }

    pub async fn heartbeats(
        &self,
        actor: &SystemActor,
    ) -> Result<Vec<InstanceHeartbeat>, AppError> {
        self.require_admin(actor).await?;
        self.ports.monitor().heartbeats().await
    }

    pub async fn migrations(&self, actor: &SystemActor) -> Result<MigrationStatus, AppError> {
        self.require_admin(actor).await?;
        self.ports.migrations().status().await
    }

    pub async fn modules(&self, actor: &SystemActor) -> Result<Vec<InstalledModuleInfo>, AppError> {
        self.require_admin(actor).await?;
        self.ports.modules().installed().await
    }
}

fn parse_job_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| job_not_found())
}

fn job_not_found() -> AppError {
    AppError::not_found_code(JOB_NOT_FOUND, "failed job was not found")
}

fn audit_entry(
    action: &'static str,
    actor: &SystemActor,
    job: &FailedJob,
    now: chrono::DateTime<chrono::Utc>,
) -> SystemAuditEntry {
    SystemAuditEntry {
        id: Uuid::now_v7(),
        action,
        actor_user_id: Some(actor.user_id.as_str().to_owned()),
        subject_type: Some("job"),
        subject_id: Some(job.id.to_string()),
        metadata: json!({
            "kind": job.kind,
            "queue": job.queue,
            "attempts": job.attempts,
            "failed_at": job.failed_at.to_rfc3339(),
        }),
        ip: actor.ip.clone(),
        occurred_at: now,
    }
}
