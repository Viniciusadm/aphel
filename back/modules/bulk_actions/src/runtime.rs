use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{BULK_RUN_JOB, GENERIC_FAILURE, RunBulkRequest, service};
use aphel_kernel::errors::AppError;
use aphel_sdk::Runtime;
use aphel_sdk::exports::system_scope;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};

pub(crate) struct BulkRunHandler {
    state: Runtime,
}

impl BulkRunHandler {
    pub(crate) fn shared(state: Runtime) -> Arc<dyn JobHandler> {
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
        let ctx = self.state.ctx(&scope).await?;
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
