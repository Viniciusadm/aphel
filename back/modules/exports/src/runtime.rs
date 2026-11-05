use async_trait::async_trait;
use serde_json::Value;

use crate::{EXPORT_RUN_JOB, RunExportRequest, service};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};

const GENERIC_FAILURE: &str = "the export could not be completed";

pub use aphel_sdk::exports::system_scope;

pub struct ExportRunHandler {
    state: AppState,
}

impl ExportRunHandler {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl JobHandler for ExportRunHandler {
    fn kind(&self) -> &str {
        EXPORT_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = RunExportRequest::from_payload(&payload)?;
        let scope = system_scope(request.requested_by.clone())?;
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        match service::run(&ctx, &request).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if context.attempt >= context.max_attempts
                    && let Err(failure) = service::fail(&ctx, &request, GENERIC_FAILURE).await
                {
                    tracing::error!(error = %failure, "failed to mark the export as failed");
                }
                Err(error)
            }
        }
    }
}
