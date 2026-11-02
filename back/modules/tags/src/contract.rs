use aphel_contract_tagging as tagging;
use aphel_contract_tagging::{GetTag, ListTags, TagInfo, TagList, TagRecord, TagSummary, Tagging};
use aphel_sdk::automation::{ADD_TAG, REMOVE_TAG};
use aphel_sdk::module::{AppBuilder, AutomationBinding, BoxFuture, OperationSpec, ReferenceCheck};
use aphel_sdk::{AppError, Ctx};
use serde_json::json;

use crate::dto::TagResponse;
use crate::service;

const NOT_ON_TAGS: &[&str] = &["activity"];

fn config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": { "tag_id": { "type": "string", "format": "uuid" } },
        "required": ["tag_id"],
        "additionalProperties": false,
    })
}

fn binding(action_key: &'static str, label: &'static str) -> AutomationBinding {
    AutomationBinding {
        action_key,
        label,
        config_schema: config_schema(),
        reference: Some(ReferenceCheck {
            field: "tag_id",
            exists_operation: "tagging.get",
        }),
        excluded_record_types: NOT_ON_TAGS,
        record_permission: Some("update"),
    }
}

async fn summaries(ctx: &Ctx<'_>, input: &TagRecord) -> Result<Vec<TagSummary>, AppError> {
    Ok(
        service::record_tags(ctx, &input.record_type, &input.record_id)
            .await?
            .into_iter()
            .map(|tag| {
                let summary = crate::dto::TagSummary::from(tag);
                TagSummary {
                    id: summary.id,
                    name: summary.name,
                    color: summary.color,
                }
            })
            .collect(),
    )
}

fn apply<'a>(ctx: &'a Ctx<'_>, input: TagRecord) -> BoxFuture<'a, Result<Tagging, AppError>> {
    Box::pin(async move {
        let changed = service::tag_record(
            ctx,
            &input.record_type,
            &input.record_id,
            &input.tag_id,
            None,
        )
        .await?;
        Ok(Tagging {
            changed,
            tags: summaries(ctx, &input).await?,
        })
    })
}

fn remove<'a>(ctx: &'a Ctx<'_>, input: TagRecord) -> BoxFuture<'a, Result<Tagging, AppError>> {
    Box::pin(async move {
        let changed = service::untag_record(
            ctx,
            &input.record_type,
            &input.record_id,
            &input.tag_id,
            None,
        )
        .await?;
        Ok(Tagging {
            changed,
            tags: summaries(ctx, &input).await?,
        })
    })
}

fn info(response: TagResponse) -> TagInfo {
    TagInfo {
        id: response.id,
        name: response.name,
        color: response.color,
        record_types: response.record_types,
        usage_count: response.usage_count,
        created_by: response.created_by,
        created_at: response.created_at,
        updated_at: response.updated_at,
    }
}

fn list<'a>(ctx: &'a Ctx<'_>, input: ListTags) -> BoxFuture<'a, Result<TagList, AppError>> {
    Box::pin(async move {
        let tags = service::list(ctx, input.record_type.as_deref()).await?;
        Ok(TagList {
            data: tags
                .into_iter()
                .map(|view| info(TagResponse::from(view)))
                .collect(),
        })
    })
}

fn get<'a>(ctx: &'a Ctx<'_>, input: GetTag) -> BoxFuture<'a, Result<TagInfo, AppError>> {
    Box::pin(async move { Ok(info(TagResponse::from(service::get(ctx, &input.id).await?))) })
}

pub(crate) fn register(app: &mut AppBuilder) {
    app.contract(tagging::DESCRIPTOR);
    app.operation(
        tagging::APPLY,
        OperationSpec::write("Tag a record the actor can edit: { record_type, record_id, tag_id } -> { changed, tags }")
            .idempotent()
            .durable()
            .automatable(binding(ADD_TAG, "Adicionar etiqueta")),
        apply,
    );
    app.operation(
        tagging::REMOVE,
        OperationSpec::write("Remove a tag from a record the actor can edit: { record_type, record_id, tag_id } -> { changed, tags }")
            .idempotent()
            .durable()
            .automatable(binding(REMOVE_TAG, "Remover etiqueta")),
        remove,
    );
    app.operation(
        tagging::LIST,
        OperationSpec::read("Tags of the organization with their usage, optionally only those allowed on a record type: { record_type? } -> { data }"),
        list,
    );
    app.operation(
        tagging::GET,
        OperationSpec::read("One tag by id: { id } -> tag"),
        get,
    );
}
