use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{BULK_RUN_JOB, GENERIC_FAILURE, RunBulkRequest, service};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{JobContext, JobHandler};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;
use exports::system_scope;

pub(crate) struct BulkRunHandler {
    state: AppState,
}

impl BulkRunHandler {
    pub(crate) fn shared(state: AppState) -> Arc<dyn JobHandler> {
        Arc::new(Self { state })
    }
}

#[async_trait]
impl JobHandler for BulkRunHandler {
    fn kind(&self) -> &str {
        BULK_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = RunBulkRequest::from_payload(&payload)?;
        let scope = system_scope(request.requested_by.clone())?;
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        match service::run(&ctx, &request).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if context.attempt >= context.max_attempts
                    && let Err(failure) = service::fail(&ctx, &request, GENERIC_FAILURE).await
                {
                    tracing::error!(error = %failure, "failed to mark the bulk operation as failed");
                }
                Err(error)
            }
        }
    }
}
