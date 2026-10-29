use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::dto::{TagResponse, TagSummary, TaggingResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
struct ListInput {
    #[serde(default)]
    record_type: Option<String>,
}

#[derive(Deserialize)]
struct TaggingInput {
    record_type: String,
    record_id: String,
    tag_id: String,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let tags: Vec<TagResponse> = service::list(ctx, input.record_type.as_deref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(json!({ "data": operation_output(tags)? }))
    })
}

async fn tagging_output(
    ctx: &Ctx<'_>,
    input: &TaggingInput,
    changed: bool,
) -> Result<Value, aphel_kernel::errors::AppError> {
    let tags: Vec<TagSummary> = service::record_tags(ctx, &input.record_type, &input.record_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    operation_output(TaggingResponse { changed, tags })
}

fn apply<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: TaggingInput = operation_input(input)?;
        let changed = service::tag_record(
            ctx,
            &input.record_type,
            &input.record_id,
            &input.tag_id,
            None,
        )
        .await?;
        tagging_output(ctx, &input, changed).await
    })
}

fn remove<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: TaggingInput = operation_input(input)?;
        let changed = service::untag_record(
            ctx,
            &input.record_type,
            &input.record_id,
            &input.tag_id,
            None,
        )
        .await?;
        tagging_output(ctx, &input, changed).await
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "tags.list",
            "Tags of the organization with their usage, optionally only those allowed on a record type: { record_type? } -> { data }",
            list,
        )),
        Arc::new(PluginOperation::write(
            "tags.apply",
            "Tag a record the actor can edit: { record_type, record_id, tag_id } -> { changed, tags }",
            apply,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "tags.remove",
            "Remove a tag from a record the actor can edit: { record_type, record_id, tag_id } -> { changed, tags }",
            remove,
        )
        .with_durable_mutations()),
    ]
}
