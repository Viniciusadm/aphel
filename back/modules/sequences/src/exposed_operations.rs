use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::EnrollmentSource;
use crate::dto::{
    EnrollContactRequest, SequenceEnrollmentResponse, SequenceListParams, SequenceResponse,
};
use crate::model::EnrollCommand;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, OperationPage, operation_input, operation_output};

#[derive(Deserialize)]
struct ListInput {
    #[serde(flatten)]
    params: SequenceListParams,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct EnrollInput {
    sequence_id: String,
    #[serde(flatten)]
    request: EnrollContactRequest,
}

#[derive(Deserialize)]
struct EnrollmentId {
    id: String,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let listing = service::list(ctx, input.params.into_query(), input.page.window()).await?;
        let sequences: Vec<SequenceResponse> = listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(sequences)?, "total": listing.total }))
    })
}

fn enroll<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: EnrollInput = operation_input(input)?;
        let enrollment = service::enroll(
            ctx,
            EnrollCommand {
                sequence_id: input.sequence_id,
                contact_id: input.request.contact_id,
                record_type: input.request.record_type,
                mailbox_id: input.request.mailbox_id,
                source: EnrollmentSource::Manual,
                enroll_as: None,
                ip: None,
            },
        )
        .await?;
        operation_output(SequenceEnrollmentResponse::from(enrollment))
    })
}

fn pause<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: EnrollmentId = operation_input(input)?;
        operation_output(SequenceEnrollmentResponse::from(
            service::pause(ctx, &input.id, None).await?,
        ))
    })
}

fn resume<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: EnrollmentId = operation_input(input)?;
        operation_output(SequenceEnrollmentResponse::from(
            service::resume(ctx, &input.id, None).await?,
        ))
    })
}

fn exit<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: EnrollmentId = operation_input(input)?;
        operation_output(SequenceEnrollmentResponse::from(
            service::remove(ctx, &input.id, None).await?,
        ))
    })
}

const OP_SEQUENCES_LIST: OperationKey<Value, Value> = operation!("sequences.list");
const OP_SEQUENCES_ENROLL: OperationKey<Value, Value> = operation!("sequences.enroll");
const OP_SEQUENCES_PAUSE: OperationKey<Value, Value> = operation!("sequences.pause");
const OP_SEQUENCES_RESUME: OperationKey<Value, Value> = operation!("sequences.resume");
const OP_SEQUENCES_EXIT: OperationKey<Value, Value> = operation!("sequences.exit");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_SEQUENCES_LIST, OperationSpec::read("Sequences with their steps and enrollment counts: { active?, search?, page?, per_page? } -> { data, total }"), list);
    app.operation(
        OP_SEQUENCES_ENROLL,
        OperationSpec::write(
            "Enroll a contact in a sequence: { sequence_id, contact_id, mailbox_id? }",
        )
        .durable(),
        enroll,
    );
    app.operation(
        OP_SEQUENCES_PAUSE,
        OperationSpec::write("Pause an active enrollment: { id }").durable(),
        pause,
    );
    app.operation(
        OP_SEQUENCES_RESUME,
        OperationSpec::write("Resume a paused enrollment: { id }").durable(),
        resume,
    );
    app.operation(
        OP_SEQUENCES_EXIT,
        OperationSpec::write("Remove a contact from a sequence (ends a live enrollment): { id }")
            .durable(),
        exit,
    );
}
