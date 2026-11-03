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

fn kinds<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let params: TimelineParams = operation_input(input)?;
        let subject = aphel_kernel::application::shared::RecordRef::parse(
            &params.record_type,
            &params.record_id,
        )?;
        service::authorize(ctx, &subject).await?;
        operation_output(
            crate::domain::kinds(ctx)?
                .into_iter()
                .map(|kind| serde_json::json!({"key":kind.key,"labels":kind.labels}))
                .collect::<Vec<_>>(),
        )
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "timeline.page",
            "Owner-authorized record timeline, newest first: { record_type, record_id, cursor?, kinds?: comma-separated contributed keys, per_page? } -> { data, meta: { next_cursor } }",
            page,
        )),
        Arc::new(PluginOperation::read(
            "timeline.kinds",
            "Available timeline kinds and localized labels: { record_type, record_id }",
            kinds,
        )),
    ]
}
