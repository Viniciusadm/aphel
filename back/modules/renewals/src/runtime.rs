use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::{RENEWALS_SCAN_CRON, RENEWALS_SCAN_JOB, RenewalScanOutcome};
use crate::service;
use crate::store::RenewalStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::custom_objects::RecordTypeCatalog;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::infrastructure::audit::MySqlAuditLog;
use aphel_sdk::platform::infrastructure::events::MySqlEventPublisher;
use aphel_sdk::platform::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};

pub(crate) fn renewals_scan_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(RENEWALS_SCAN_JOB, RENEWALS_SCAN_CRON)
        .expect("the renewals scan cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) struct RenewalScanHandler {
    pool: Db,
    catalog: Arc<RecordTypeCatalog>,
}

impl RenewalScanHandler {
    pub(crate) fn new(pool: Db, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }

    async fn run(&self) -> Result<RenewalScanOutcome, AppError> {
        let store = RenewalStore::new(self.pool.clone());
        let mut outcome = RenewalScanOutcome::default();
        let settings = store.enabled().await?;
        if settings.is_empty() {
            return Ok(outcome);
        }
        let registry = self.catalog.registry().await?;
        let events: SharedEventPublisher = Arc::new(MySqlEventPublisher::new());
        let audit: SharedAuditLog = Arc::new(MySqlAuditLog::new());
        let timezone = store.timezone().await?;
        let today = chrono::Utc::now().with_timezone(&timezone).date_naive();
        for setting in settings {
            let Some(user_id) = setting.updated_by.as_ref() else {
                tracing::warn!(pipeline_id = %setting.pipeline_id, "renewal settings without an author were skipped");
                continue;
            };
            let Some(scope) = store.member_scope(user_id).await? else {
                tracing::warn!(pipeline_id = %setting.pipeline_id, "the author of the renewal settings is no longer an active user");
                continue;
            };
            let ctx = Ctx::from_parts(
                &scope,
                self.pool.clone(),
                registry.clone(),
                events.clone(),
                audit.clone(),
            );
            for deal_id in store.due_deals(&ctx, &setting, today).await? {
                match service::renew(&ctx, &setting, deal_id).await {
                    Ok(true) => outcome.created += 1,
                    Ok(false) => {}
                    Err(error) => {
                        outcome.failed += 1;
                        tracing::warn!(%deal_id, %error, "could not create the renewal of a deal");
                    }
                }
            }
        }
        Ok(outcome)
    }
}

#[async_trait]
impl JobHandler for RenewalScanHandler {
    fn kind(&self) -> &str {
        RENEWALS_SCAN_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let outcome = self.run().await?;
        if outcome.created > 0 || outcome.failed > 0 {
            tracing::info!(
                created = outcome.created,
                failed = outcome.failed,
                "deal renewals processed"
            );
        }
        Ok(())
    }
}
