use std::sync::Arc;

use serde_json::{Value, json};

use crate::http::PipelineResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_output};

fn list<'a>(ctx: &'a Ctx<'_>, _input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let pipelines: Vec<PipelineResponse> = service::list_pipelines(ctx)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(json!({ "data": operation_output(pipelines)? }))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(PluginOperation::read(
        "pipelines.list",
        "Pipelines visible to the actor with their stages: {} -> { data }",
        list,
    ))]
}
