use async_trait::async_trait;
use serde_json::Value;

use crate::IMPORT_RUN_JOB;
use crate::service::{self, GENERIC_FAILURE, RunImportRequest, RunProgress};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;
use aphel_sdk::exports::system_scope;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};

pub(crate) struct ImportRunHandler {
    state: AppState,
}

impl ImportRunHandler {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl JobHandler for ImportRunHandler {
    fn kind(&self) -> &str {
        IMPORT_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = RunImportRequest::from_payload(&payload)?;
        let scope = system_scope(request.requested_by.clone())?;
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        match service::run(&ctx, &request).await {
            Ok(RunProgress::Finished) => Ok(()),
            Ok(RunProgress::Continue { processed }) => {
                service::continue_later(&ctx, &request, processed).await
            }
            Err(error) => {
                if context.attempt >= context.max_attempts
                    && let Err(failure) = service::fail(&ctx, &request, GENERIC_FAILURE).await
                {
                    tracing::error!(error = %failure, "failed to mark the import as failed");
                }
                Err(error)
            }
        }
    }
}
