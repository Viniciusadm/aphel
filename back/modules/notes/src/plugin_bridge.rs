use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::{CreateNoteCommand, ListNotesQuery};
use crate::http::{CreateNoteRequest, NoteResponse, UpdateNoteRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};

#[derive(Deserialize)]
struct ListInput {
    record_type: String,
    record_id: String,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct NoteId {
    id: String,
}

#[derive(Deserialize)]
struct UpdateInput {
    id: String,
    #[serde(flatten)]
    changes: UpdateNoteRequest,
}

#[derive(Deserialize)]
struct PinInput {
    id: String,
    pinned: bool,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let listing = service::list(
            ctx,
            ListNotesQuery {
                record_type: input.record_type,
                record_id: input.record_id,
            },
            input.page.window(),
        )
        .await?;
        let notes: Vec<NoteResponse> = listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(notes)?, "total": listing.total }))
    })
}

fn get<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: NoteId = operation_input(input)?;
        operation_output(NoteResponse::from(service::get(ctx, &input.id).await?))
    })
}

fn create<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: CreateNoteRequest = operation_input(input)?;
        let command: CreateNoteCommand = request.into_command(None);
        operation_output(NoteResponse::from(service::create(ctx, command).await?))
    })
}

fn update<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: UpdateInput = operation_input(input)?;
        let note = service::update(ctx, &input.id, input.changes.into_command(None, None)).await?;
        operation_output(NoteResponse::from(note))
    })
}

fn pin<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: PinInput = operation_input(input)?;
        let note = service::set_pinned(ctx, &input.id, input.pinned, None).await?;
        operation_output(NoteResponse::from(note))
    })
}

fn delete<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: NoteId = operation_input(input)?;
        service::delete(ctx, &input.id, None).await?;
        Ok(json!({ "deleted": true }))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "notes.list",
            "Notes of a record visible to the actor: { record_type, record_id, page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(PluginOperation::read("notes.get", "One note: { id }", get)),
        Arc::new(PluginOperation::write(
            "notes.create",
            "Create a note on a record: { record_type, record_id, body, visibility?: team|private, pinned? }",
            create,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "notes.update",
            "Edit a note: { id, body?, visibility?, pinned? }",
            update,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "notes.pin",
            "Pin or unpin a note: { id, pinned }",
            pin,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "notes.delete",
            "Delete a note: { id }",
            delete,
        )
        .with_durable_mutations()),
    ]
}
