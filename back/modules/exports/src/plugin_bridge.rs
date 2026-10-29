use std::sync::Arc;

use serde_json::Value;

use crate::dto::{ExportJobResponse, ExportRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

fn request<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: ExportRequest = operation_input(input)?;
        let job = service::request(ctx, request.into_command(None)?).await?;
        operation_output(ExportJobResponse::from(job))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(PluginOperation::write(
        "exports.request",
        "Queue a CSV or XLSX export of records the actor can read (at most 100 000); the requester is notified when the file is ready: { record_type, format: csv|xlsx, filter?, sort?, search?, archived?: exclude|include|only, columns? } -> { id, record_type, format, status, columns, file_id, row_count, failure_reason, requested_by, created_at, .. }",
        request,
    ))]
}
