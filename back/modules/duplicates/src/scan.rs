use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::service;
use crate::{
    DUPLICATES_CLEANUP_CRON, DUPLICATES_CLEANUP_JOB, DUPLICATES_SCAN_CRON, DUPLICATES_SCAN_JOB,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{JobContext, JobHandler};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;

pub(crate) fn duplicates_scan_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(DUPLICATES_SCAN_JOB, DUPLICATES_SCAN_CRON)
        .expect("the duplicates scan cron expression is valid")
        .with_max_attempts(2)
}

pub(crate) fn duplicates_cleanup_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(DUPLICATES_CLEANUP_JOB, DUPLICATES_CLEANUP_CRON)
        .expect("the duplicates cleanup cron expression is valid")
        .with_max_attempts(2)
}

pub(crate) struct DuplicateScanHandler {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl DuplicateScanHandler {
    pub(crate) fn new(pool: MySqlPool, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }
}

#[async_trait]
impl JobHandler for DuplicateScanHandler {
    fn kind(&self) -> &str {
        DUPLICATES_SCAN_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = system_ctx(&scope, &self.pool, &self.catalog).await?;
        let summary = service::scan_all(&ctx).await?;
        tracing::info!(
            records = summary.records,
            candidates = summary.candidates,
            "duplicate scan finished"
        );
        Ok(())
    }
}

async fn system_ctx<'a>(
    scope: &'a AccessScope,
    pool: &MySqlPool,
    catalog: &RecordTypeCatalog,
) -> Result<Ctx<'a>, AppError> {
    Ok(Ctx::from_parts(
        scope,
        pool.clone(),
        catalog.registry().await?,
        Arc::new(MySqlEventPublisher::new()),
        Arc::new(MySqlAuditLog::new()),
    ))
}

pub(crate) struct DuplicateCleanupHandler {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl DuplicateCleanupHandler {
    pub(crate) fn new(pool: MySqlPool, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }
}

#[async_trait]
impl JobHandler for DuplicateCleanupHandler {
    fn kind(&self) -> &str {
        DUPLICATES_CLEANUP_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = system_ctx(&scope, &self.pool, &self.catalog).await?;
        let summary = service::cleanup_all(&ctx).await?;
        tracing::info!(
            removed = summary.removed,
            updated = summary.updated,
            "duplicate candidates cleanup finished"
        );
        Ok(())
    }
}
