use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{RequestResponse, RevisionResponse, StatusResponse};
use crate::service::{self, CaptureInput, RequestInput, RestoreInput, WithdrawInput};
use aphel_contract_publication::{
    OP_CANCEL, OP_CAPTURE, OP_REQUEST, OP_RESTORE, OP_RETRY, OP_REVISION, OP_REVISIONS, OP_STATUS,
    OP_WITHDRAW,
};
use aphel_kernel::Ctx;
use aphel_kernel::modules::{
    AppBuilder, OperationFuture, OperationSpec, operation_input, operation_output,
};
use aphel_sdk::{OperationKey, operation};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureArgs {
    record_type: String,
    record_id: String,
    expected_version: Option<DateTime<Utc>>,
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestArgs {
    revision_id: String,
    at: Option<DateTime<Utc>>,
    idempotency_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WithdrawArgs {
    record_type: String,
    record_id: String,
    idempotency_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdArgs {
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreArgs {
    revision_id: String,
    expected_version: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordArgs {
    record_type: String,
    record_id: String,
    page: Option<u32>,
    per_page: Option<u32>,
}

fn capture<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: CaptureArgs = operation_input(input)?;
        let revision = service::capture(
            ctx,
            CaptureInput {
                record_type: args.record_type,
                record_id: args.record_id,
                expected_version: args.expected_version,
                note: args.note,
            },
        )
        .await?;
        operation_output(RevisionResponse::of(revision, false))
    })
}

fn request<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: RequestArgs = operation_input(input)?;
        let request = service::request(
            ctx,
            RequestInput {
                revision_id: args.revision_id,
                at: args.at,
                idempotency_key: args.idempotency_key,
            },
        )
        .await?;
        operation_output(RequestResponse::from(request))
    })
}

fn withdraw<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: WithdrawArgs = operation_input(input)?;
        let request = service::withdraw(
            ctx,
            WithdrawInput {
                record_type: args.record_type,
                record_id: args.record_id,
                idempotency_key: args.idempotency_key,
            },
        )
        .await?;
        operation_output(RequestResponse::from(request))
    })
}

fn cancel<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: IdArgs = operation_input(input)?;
        operation_output(RequestResponse::from(service::cancel(ctx, &args.id).await?))
    })
}

fn retry<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: IdArgs = operation_input(input)?;
        operation_output(RequestResponse::from(service::retry(ctx, &args.id).await?))
    })
}

fn restore<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: RestoreArgs = operation_input(input)?;
        service::restore(
            ctx,
            RestoreInput {
                revision_id: args.revision_id,
                expected_version: args.expected_version,
            },
        )
        .await
    })
}

fn status<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: RecordArgs = operation_input(input)?;
        let view = service::status(ctx, &args.record_type, &args.record_id).await?;
        operation_output(StatusResponse::from(view))
    })
}

fn revisions<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: RecordArgs = operation_input(input)?;
        let per_page = args.per_page.unwrap_or(20).clamp(1, 100);
        let page = args.page.unwrap_or(1).max(1);
        let (items, total) = service::revisions(
            ctx,
            &args.record_type,
            &args.record_id,
            per_page,
            (page - 1) * per_page,
        )
        .await?;
        let data = items
            .into_iter()
            .map(|revision| RevisionResponse::of(revision, false))
            .collect::<Vec<_>>();
        Ok(json!({ "data": operation_output(data)?, "total": total }))
    })
}

fn revision<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let args: IdArgs = operation_input(input)?;
        operation_output(RevisionResponse::of(
            service::revision(ctx, &args.id).await?,
            true,
        ))
    })
}

const CAPTURE: OperationKey<Value, Value> = operation!(OP_CAPTURE);
const REQUEST: OperationKey<Value, Value> = operation!(OP_REQUEST);
const WITHDRAW: OperationKey<Value, Value> = operation!(OP_WITHDRAW);
const CANCEL: OperationKey<Value, Value> = operation!(OP_CANCEL);
const RETRY: OperationKey<Value, Value> = operation!(OP_RETRY);
const RESTORE: OperationKey<Value, Value> = operation!(OP_RESTORE);
const STATUS: OperationKey<Value, Value> = operation!(OP_STATUS);
const REVISIONS: OperationKey<Value, Value> = operation!(OP_REVISIONS);
const REVISION: OperationKey<Value, Value> = operation!(OP_REVISION);

pub(crate) fn register(app: &mut AppBuilder) {
    app.operation(CAPTURE, OperationSpec::write("Capture the saved draft of a publishable record as a revision: { record_type, record_id, expected_version, note? } -> revision").durable(), capture);
    app.operation(REQUEST, OperationSpec::write("Publish a revision now or at a time (UTC), subject to approval policies: { revision_id, at?, idempotency_key } -> request").durable(), request);
    app.operation(WITHDRAW, OperationSpec::write("Withdraw the published revision of a record: { record_type, record_id, idempotency_key } -> request").durable(), withdraw);
    app.operation(
        CANCEL,
        OperationSpec::write("Cancel an open publication request: { id } -> request").durable(),
        cancel,
    );
    app.operation(
        RETRY,
        OperationSpec::write("Retry a failed publication request: { id } -> request").durable(),
        retry,
    );
    app.operation(RESTORE, OperationSpec::write("Copy a revision back into the draft fields: { revision_id, expected_version } -> record").durable(), restore);
    app.operation(
        STATUS,
        OperationSpec::read("Publication status of a record: { record_type, record_id } -> status"),
        status,
    );
    app.operation(REVISIONS, OperationSpec::read("Revisions of a record without payloads: { record_type, record_id, page?, per_page? } -> { data, total }"), revisions);
    app.operation(
        REVISION,
        OperationSpec::read("One revision with its payload: { id } -> revision"),
        revision,
    );
}
