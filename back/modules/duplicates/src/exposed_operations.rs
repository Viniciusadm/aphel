use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::dto::{
    DuplicateCandidateResponse, DuplicateCheckRequest, DuplicateMatchResponse, MergeRequest,
    MergeResponse, match_response,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

#[derive(Deserialize)]
struct MergeInput {
    #[serde(flatten)]
    request: MergeRequest,
    #[serde(default)]
    expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct CandidateId {
    id: String,
}

fn check<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: DuplicateCheckRequest = operation_input(input)?;
        let matches = service::check(ctx, request.into()).await?;
        let matches: Vec<DuplicateMatchResponse> =
            matches.into_iter().map(match_response).collect();
        operation_output(matches)
    })
}

fn merge<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: MergeInput = operation_input(input)?;
        let outcome = service::merge(
            ctx,
            input.request.into_command(input.expected_updated_at, None),
        )
        .await?;
        operation_output(MergeResponse::from(outcome))
    })
}

fn dismiss<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: CandidateId = operation_input(input)?;
        let candidate = service::dismiss(ctx, &input.id, None).await?;
        operation_output(DuplicateCandidateResponse::from(candidate))
    })
}

const OP_DUPLICATES_CHECK: OperationKey<Value, Value> = operation!("duplicates.check");
const OP_DUPLICATES_MERGE: OperationKey<Value, Value> = operation!("duplicates.merge");
const OP_DUPLICATES_DISMISS: OperationKey<Value, Value> = operation!("duplicates.dismiss");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_DUPLICATES_CHECK, OperationSpec::read("Visible records the enabled rules consider possible duplicates of the given values (at most 10); nothing is stored: { record_type, exclude_id?, name?, first_name?, last_name?, email?, phone?, mobile?, company_id?, domain?, website?, values?: { field_key: text } } -> [{ record_id, title, rules }]"), check);
    app.operation(OP_DUPLICATES_MERGE, OperationSpec::write("Merge duplicate_id into primary_id (the duplicate goes to the trash): { record_type, primary_id, duplicate_id, fields?: { field_key: primary|duplicate }, expected_updated_at? (primary's updated_at, ISO-8601) } -> { record_type, survivor_id, merged_id, moved }").durable(), merge);
    app.operation(
        OP_DUPLICATES_DISMISS,
        OperationSpec::write("Mark a duplicate candidate as not a duplicate: { id } -> candidate")
            .durable(),
        dismiss,
    );
}
