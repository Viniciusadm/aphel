use std::sync::Arc;

use serde_json::Value;

use crate::http::{TimelinePageResponse, TimelineParams, mask_timeline};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

fn page<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let params: TimelineParams = operation_input(input)?;
        let mut page = service::page(ctx, params.into()).await?;
        mask_timeline(&ctx.state(), ctx.scope, &mut page.items).await?;
        operation_output(TimelinePageResponse::from(page))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(PluginOperation::read(
        "timeline.page",
        "Timeline of a record the actor can read, newest first: { record_type, record_id, cursor?, kinds?: \"activity,note,deal_stage,deal_change,record_change,attachment,event\", per_page? } -> { data, meta: { next_cursor } }",
        page,
    ))]
}
