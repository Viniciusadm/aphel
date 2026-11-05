use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::RequestListQuery;
use crate::http::{
    ApprovalDecisionRequest, ApprovalRequestResponse, ApprovalStatusSchema,
    ApprovalSubmissionResponse, ApprovalSubmitRequest,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, OperationPage, operation_input, operation_output};

#[derive(Deserialize)]
struct RequestsInput {
    record_type: String,
    record_id: String,
    #[serde(default)]
    status: Option<ApprovalStatusSchema>,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct DecisionInput {
    id: String,
    #[serde(default)]
    comment: Option<String>,
}

impl DecisionInput {
    fn into_parts(self) -> (String, ApprovalDecisionRequest) {
        (
            self.id,
            ApprovalDecisionRequest {
                comment: self.comment,
            },
        )
    }
}

fn require_user_actor(ctx: &Ctx<'_>) -> Result<(), AppError> {
    if ctx.scope.plugin_key().is_some() {
        return Err(AppError::Validation(
            "this operation needs a user actor: call it from a hook or an authenticated route"
                .into(),
        ));
    }
    Ok(())
}

fn requests<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: RequestsInput = operation_input(input)?;
        let listing = service::list_requests(
            ctx,
            RequestListQuery {
                record_type: Some(input.record_type),
                record_id: Some(input.record_id),
                box_name: None,
                status: input.status.map(Into::into),
            },
            input.page.window(),
        )
        .await?;
        let requests: Vec<ApprovalRequestResponse> =
            listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(requests)?, "total": listing.total }))
    })
}

fn submit<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: ApprovalSubmitRequest = operation_input(input)?;
        let (command, policy_key) = request.into_command(None);
        let submission = service::submit(ctx, command, policy_key.as_deref()).await?;
        operation_output(ApprovalSubmissionResponse::from(submission))
    })
}

fn approve<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let (id, request) = operation_input::<DecisionInput>(input)?.into_parts();
        let view = service::approve(ctx, &id, request.into_command(None)).await?;
        operation_output(ApprovalRequestResponse::from(view))
    })
}

fn reject<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let (id, request) = operation_input::<DecisionInput>(input)?.into_parts();
        let view = service::reject(ctx, &id, request.into_command(None)).await?;
        operation_output(ApprovalRequestResponse::from(view))
    })
}

fn cancel<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let (id, request) = operation_input::<DecisionInput>(input)?.into_parts();
        let view = service::cancel(ctx, &id, request.into_command(None)).await?;
        operation_output(ApprovalRequestResponse::from(view))
    })
}

const OP_APPROVALS_REQUESTS: OperationKey<Value, Value> = operation!("approvals.requests");
const OP_APPROVALS_SUBMIT: OperationKey<Value, Value> = operation!("approvals.submit");
const OP_APPROVALS_APPROVE: OperationKey<Value, Value> = operation!("approvals.approve");
const OP_APPROVALS_REJECT: OperationKey<Value, Value> = operation!("approvals.reject");
const OP_APPROVALS_CANCEL: OperationKey<Value, Value> = operation!("approvals.cancel");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_APPROVALS_REQUESTS, OperationSpec::read("Approval requests of a record the actor can read: { record_type, record_id, status?, page?, per_page? } -> { data, total }"), requests);
    app.operation(OP_APPROVALS_SUBMIT, OperationSpec::write("Ask for the approvals a record needs at a moment (needs edit access): { record_type, record_id, moment: on_demand|stage_change|quote_send, target_id?, comment?, policy_key? } -> { outcome, requests }").durable(), submit);
    app.operation(OP_APPROVALS_APPROVE, OperationSpec::write("Approve a request as an approver of its current step (needs a user actor): { id, comment? }").durable(), approve);
    app.operation(OP_APPROVALS_REJECT, OperationSpec::write("Reject a request as an approver of its current step (needs a user actor): { id, comment }").durable(), reject);
    app.operation(
        OP_APPROVALS_CANCEL,
        OperationSpec::write("Cancel a pending request (needs a user actor): { id, comment? }")
            .durable(),
        cancel,
    );
}
