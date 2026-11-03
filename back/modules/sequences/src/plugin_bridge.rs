use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::EnrollmentSource;
use crate::dto::{
    EnrollContactRequest, SequenceEnrollmentResponse, SequenceListParams, SequenceResponse,
};
use crate::hooks::RunningStep;
use crate::model::EnrollCommand;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, OperationFuture, OperationPage, PluginHook, PluginHookKind, PluginOperation,
    forward_plugin_hook, operation_input, operation_output,
};

pub(crate) const STEP_RUNNING: &str = "sequence.step_running";

fn step_json(step: &RunningStep) -> Value {
    json!({
        "enrollment_id": step.enrollment_id,
        "sequence_id": step.sequence_id,
        "contact_id": step.contact_id,
        "enrolled_by": step.enrolled_by.as_str(),
        "position": step.position,
        "kind": step.kind.as_str(),
        "template_id": step.template_id,
    })
}

pub(crate) async fn step_running(
    ctx: &mut HookCtx<'_>,
    step: &RunningStep,
) -> Result<(), AppError> {
    forward_plugin_hook(ctx, STEP_RUNNING, "people.person", step.contact_id, || {
        step_json(step)
    })
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![Arc::new(PluginHook::new(
        STEP_RUNNING,
        "people.person",
        PluginHookKind::Veto,
        "Before a sequence step runs for an enrolled contact; values: { enrollment_id, sequence_id, contact_id, enrolled_by, position, kind, template_id }. A definitive error removes the contact from the sequence, a transient one retries the step",
    ))]
}

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

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "sequences.list",
            "Sequences with their steps and enrollment counts: { active?, search?, page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(
            PluginOperation::write(
                "sequences.enroll",
                "Enroll a contact in a sequence: { sequence_id, contact_id, mailbox_id? }",
                enroll,
            )
            .with_durable_mutations(),
        ),
        Arc::new(
            PluginOperation::write(
                "sequences.pause",
                "Pause an active enrollment: { id }",
                pause,
            )
            .with_durable_mutations(),
        ),
        Arc::new(
            PluginOperation::write(
                "sequences.resume",
                "Resume a paused enrollment: { id }",
                resume,
            )
            .with_durable_mutations(),
        ),
        Arc::new(
            PluginOperation::write(
                "sequences.exit",
                "Remove a contact from a sequence (ends a live enrollment): { id }",
                exit,
            )
            .with_durable_mutations(),
        ),
    ]
}
