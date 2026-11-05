use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::{CreateNoteCommand, ListNotesQuery};
use crate::http::{CreateNoteRequest, NoteResponse, UpdateNoteRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, OperationPage, operation_input, operation_output};

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

const OP_NOTES_LIST: OperationKey<Value, Value> = operation!("notes.list");
const OP_NOTES_GET: OperationKey<Value, Value> = operation!("notes.get");
const OP_NOTES_CREATE: OperationKey<Value, Value> = operation!("notes.create");
const OP_NOTES_UPDATE: OperationKey<Value, Value> = operation!("notes.update");
const OP_NOTES_PIN: OperationKey<Value, Value> = operation!("notes.pin");
const OP_NOTES_DELETE: OperationKey<Value, Value> = operation!("notes.delete");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_NOTES_LIST, OperationSpec::read("Notes of a record visible to the actor: { record_type, record_id, page?, per_page? } -> { data, total }"), list);
    app.operation(OP_NOTES_GET, OperationSpec::read("One note: { id }"), get);
    app.operation(OP_NOTES_CREATE, OperationSpec::write("Create a note on a record: { record_type, record_id, body, visibility?: team|private, pinned? }").durable(), create);
    app.operation(
        OP_NOTES_UPDATE,
        OperationSpec::write("Edit a note: { id, body?, visibility?, pinned? }").durable(),
        update,
    );
    app.operation(
        OP_NOTES_PIN,
        OperationSpec::write("Pin or unpin a note: { id, pinned }").durable(),
        pin,
    );
    app.operation(
        OP_NOTES_DELETE,
        OperationSpec::write("Delete a note: { id }").durable(),
        delete,
    );
}
