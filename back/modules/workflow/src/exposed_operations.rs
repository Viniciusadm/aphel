use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde_json::{Value, json};

use crate::http::PipelineResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_output};

fn list<'a>(ctx: &'a Ctx<'_>, _input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let workflow: Vec<PipelineResponse> = service::list_pipelines(ctx)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(json!({ "data": operation_output(workflow)? }))
    })
}

const OP_WORKFLOW_LIST: OperationKey<Value, Value> = operation!("workflow.list");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(
        OP_WORKFLOW_LIST,
        OperationSpec::read("Pipelines visible to the actor with their stages: {} -> { data }"),
        list,
    );
}
