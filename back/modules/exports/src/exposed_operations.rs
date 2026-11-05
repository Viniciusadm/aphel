use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde_json::Value;

use crate::dto::{ExportJobResponse, ExportRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

fn request<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: ExportRequest = operation_input(input)?;
        let job = service::request(ctx, request.into_command(None)?).await?;
        operation_output(ExportJobResponse::from(job))
    })
}

const OP_EXPORTS_REQUEST: OperationKey<Value, Value> = operation!("exports.request");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_EXPORTS_REQUEST, OperationSpec::write("Queue a CSV or XLSX export of records the actor can read (at most 100 000); the requester is notified when the file is ready: { record_type, format: csv|xlsx, filter?, sort?, search?, archived?: exclude|include|only, columns? } -> { id, record_type, format, status, columns, file_id, row_count, failure_reason, requested_by, created_at, .. }").durable(), request);
}
