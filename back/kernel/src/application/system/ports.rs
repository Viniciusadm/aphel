use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::security::Clock;
use crate::application::shared::TransactionManager;
use crate::application::system::model::{
    FailedJob, FailedJobFilter, FailureEntry, InstalledModuleInfo, InstanceHeartbeat,
    JobKindCounts, MigrationStatus, PageWindow, SystemAuditEntry,
};
use crate::errors::AppError;

#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn database(&self) -> Result<(), AppError>;
    async fn storage(&self) -> Result<(), AppError>;
    async fn latest_heartbeat(&self, role: &str) -> Result<Option<DateTime<Utc>>, AppError>;
    async fn oldest_due_schedule(&self) -> Result<Option<DateTime<Utc>>, AppError>;
    async fn oldest_ready_job(&self, now: DateTime<Utc>)
    -> Result<Option<DateTime<Utc>>, AppError>;
}

#[async_trait]
pub trait QueueMonitor: Send + Sync {
    async fn job_counts(&self, now: DateTime<Utc>) -> Result<Vec<JobKindCounts>, AppError>;
    async fn oldest_ready_job(&self, now: DateTime<Utc>)
    -> Result<Option<DateTime<Utc>>, AppError>;
    async fn failed_jobs(
        &self,
        filter: &FailedJobFilter,
        window: PageWindow,
    ) -> Result<(Vec<FailedJob>, u64), AppError>;
    async fn recent_failures(
        &self,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError>;
    async fn heartbeats(&self) -> Result<Vec<InstanceHeartbeat>, AppError>;
}

#[async_trait]
pub trait FailedJobRepository: Send + Sync {
    type Tx: Send;

    async fn lock_failed(&self, tx: &mut Self::Tx, id: Uuid)
    -> Result<Option<FailedJob>, AppError>;
    async fn requeue(
        &self,
        tx: &mut Self::Tx,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, id: Uuid) -> Result<(), AppError>;
}

#[async_trait]
pub trait SystemAuditLog<Tx: Send>: Send + Sync {
    async fn record(&self, tx: &mut Tx, entry: SystemAuditEntry) -> Result<(), AppError>;
}

#[async_trait]
pub trait MigrationInspector: Send + Sync {
    async fn status(&self) -> Result<MigrationStatus, AppError>;
}

#[async_trait]
pub trait ModuleInventory: Send + Sync {
    async fn installed(&self) -> Result<Vec<InstalledModuleInfo>, AppError>;
}

pub trait SystemAdminPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Monitor: QueueMonitor;
    type FailedJobs: FailedJobRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Audit: SystemAuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Migrations: MigrationInspector;
    type Modules: ModuleInventory;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn monitor(&self) -> &Self::Monitor;
    fn failed_jobs(&self) -> &Self::FailedJobs;
    fn audit(&self) -> &Self::Audit;
    fn migrations(&self) -> &Self::Migrations;
    fn modules(&self) -> &Self::Modules;
    fn clock(&self) -> &Self::Clock;
}
