use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde_json::Value;

use crate::http::{TimelinePageResponse, TimelineParams, mask_timeline};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

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
        let subject = aphel_sdk::platform::application::shared::RecordRef::parse(
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

const OP_TIMELINE_PAGE: OperationKey<Value, Value> = operation!("timeline.page");
const OP_TIMELINE_KINDS: OperationKey<Value, Value> = operation!("timeline.kinds");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_TIMELINE_PAGE, OperationSpec::read("Owner-authorized record timeline, newest first: { record_type, record_id, cursor?, kinds?: comma-separated contributed keys, per_page? } -> { data, meta: { next_cursor } }"), page);
    app.operation(
        OP_TIMELINE_KINDS,
        OperationSpec::read(
            "Available timeline kinds and localized labels: { record_type, record_id }",
        ),
        kinds,
    );
}
