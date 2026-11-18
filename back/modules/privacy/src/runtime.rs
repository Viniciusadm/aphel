use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::{
    PRIVACY_REQUEST_RUN_JOB, PRIVACY_RETENTION_CRON, PRIVACY_RETENTION_JOB,
    PRIVACY_RETENTION_RUN_JOB, PRIVACY_RETENTION_SWEEP_CRON, PRIVACY_RETENTION_SWEEP_JOB,
};
use crate::service::requests::{self, GENERIC_FAILURE, RunPrivacyRequest};
use crate::service::retention::{self, RunRetentionRequest};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::custom_objects::RecordTypeCatalog;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler, SharedFileStorage};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::audit::MySqlAuditLog;
use aphel_sdk::platform::infrastructure::events::MySqlEventPublisher;

pub(crate) fn privacy_retention_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(PRIVACY_RETENTION_JOB, PRIVACY_RETENTION_CRON)
        .expect("the privacy retention cron expression is valid")
        .with_max_attempts(2)
}

pub(crate) fn privacy_retention_sweep_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(PRIVACY_RETENTION_SWEEP_JOB, PRIVACY_RETENTION_SWEEP_CRON)
        .expect("the privacy retention sweep cron expression is valid")
        .with_max_attempts(1)
}

#[derive(Clone)]
pub(crate) struct PrivacyRuntime {
    pool: Db,
    catalog: Arc<RecordTypeCatalog>,
    storage: SharedFileStorage,
}

impl PrivacyRuntime {
    pub(crate) fn new(
        pool: Db,
        catalog: Arc<RecordTypeCatalog>,
        storage: SharedFileStorage,
    ) -> Self {
        Self {
            pool,
            catalog,
            storage,
        }
    }

    pub(crate) fn handlers(&self) -> Vec<Arc<dyn JobHandler>> {
        vec![
            Arc::new(PrivacyRequestRunHandler(self.clone())),
            Arc::new(PrivacyRetentionHandler(self.clone())),
            Arc::new(RetentionRunHandler(self.clone())),
            Arc::new(RetentionSweepHandler(self.clone())),
        ]
    }

    async fn ctx<'s>(&self, scope: &'s AccessScope) -> Result<Ctx<'s>, AppError> {
        Ok(Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        )
        .with_storage(self.storage.clone()))
    }
}

pub(crate) struct PrivacyRequestRunHandler(pub(crate) PrivacyRuntime);

#[async_trait]
impl JobHandler for PrivacyRequestRunHandler {
    fn kind(&self) -> &str {
        PRIVACY_REQUEST_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = RunPrivacyRequest::from_payload(&payload)?;
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        match requests::run_request(&ctx, self.0.storage.as_ref(), &request).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if context.attempt >= context.max_attempts
                    && let Err(failure) =
                        requests::fail_request(&ctx, &request, GENERIC_FAILURE, None).await
                {
                    tracing::error!(error = %failure, "failed to mark the privacy request as failed");
                }
                Err(error)
            }
        }
    }
}

pub(crate) struct PrivacyRetentionHandler(pub(crate) PrivacyRuntime);

#[async_trait]
impl JobHandler for PrivacyRetentionHandler {
    fn kind(&self) -> &str {
        PRIVACY_RETENTION_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        let summary = retention::run_scheduled(&ctx, self.0.storage.as_ref()).await?;
        tracing::info!(
            runs = summary.runs,
            expired_exports = summary.expired_exports,
            "privacy retention finished"
        );
        Ok(())
    }
}

pub(crate) struct RetentionRunHandler(pub(crate) PrivacyRuntime);

#[async_trait]
impl JobHandler for RetentionRunHandler {
    fn kind(&self) -> &str {
        PRIVACY_RETENTION_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = RunRetentionRequest::from_payload(&payload)?;
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        match retention::run_queued(&ctx, self.0.storage.as_ref(), &request).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if context.attempt >= context.max_attempts
                    && let Err(failure) = retention::fail_queued_run(&ctx, &request).await
                {
                    tracing::error!(error = %failure, "failed to mark the retention run as failed");
                }
                Err(error)
            }
        }
    }
}

pub(crate) struct RetentionSweepHandler(pub(crate) PrivacyRuntime);

#[async_trait]
impl JobHandler for RetentionSweepHandler {
    fn kind(&self) -> &str {
        PRIVACY_RETENTION_SWEEP_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        let summary = retention::sweep_stale_runs(&ctx).await?;
        if summary.resumed > 0 || summary.abandoned > 0 {
            tracing::warn!(
                resumed = summary.resumed,
                abandoned = summary.abandoned,
                "stale retention runs recovered"
            );
        }
        Ok(())
    }
}
