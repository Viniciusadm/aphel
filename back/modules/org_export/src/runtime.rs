use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::service::{self, RunOrgExportRequest};
use crate::{GENERIC_FAILURE, ORG_EXPORT_RUN_JOB};
use aphel_kernel::Ctx;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::shared::{JobContext, JobHandler, SharedFileStorage};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;

pub(crate) struct OrgExportRunHandler {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
    storage: SharedFileStorage,
}

impl OrgExportRunHandler {
    pub(crate) fn new(
        pool: MySqlPool,
        catalog: Arc<RecordTypeCatalog>,
        storage: SharedFileStorage,
    ) -> Self {
        Self {
            pool,
            catalog,
            storage,
        }
    }
}

#[async_trait]
impl JobHandler for OrgExportRunHandler {
    fn kind(&self) -> &str {
        ORG_EXPORT_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = RunOrgExportRequest::from_payload(&payload)?;
        let scope = AccessScope::system();
        let ctx = Ctx::from_parts(
            &scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        )
        .with_storage(self.storage.clone());
        match service::run(&ctx, self.storage.as_ref(), &request).await {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::error!(error = %error, "organization export failed");
                if context.attempt >= context.max_attempts
                    && let Err(failure) = service::fail(&ctx, &request, GENERIC_FAILURE).await
                {
                    tracing::error!(error = %failure, "failed to mark the organization export as failed");
                }
                Err(error)
            }
        }
    }
}
