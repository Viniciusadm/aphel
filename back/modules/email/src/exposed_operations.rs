use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::dto::email_outbound::SendEmailRequest;
use crate::dto::email_sync::{
    EmailMessageResponse, EmailThreadDetailResponse, LinkEmailThreadRequest,
};
use crate::service::{compose, linking};
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

#[derive(Deserialize)]
struct LinkInput {
    thread_id: String,
    #[serde(flatten)]
    request: LinkEmailThreadRequest,
}

fn send<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: SendEmailRequest = operation_input(input)?;
        let email = compose::send(ctx, request.into_command(None)).await?;
        operation_output(EmailMessageResponse::from(email))
    })
}

fn link<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: LinkInput = operation_input(input)?;
        let detail = linking::link(
            ctx,
            &input.thread_id,
            &input.request.record_type,
            &input.request.record_id,
            None,
        )
        .await?;
        operation_output(EmailThreadDetailResponse::from(detail))
    })
}

fn unlink<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: LinkInput = operation_input(input)?;
        linking::unlink(
            ctx,
            &input.thread_id,
            &input.request.record_type,
            &input.request.record_id,
            None,
        )
        .await?;
        Ok(json!({ "unlinked": true }))
    })
}

const OP_EMAIL_SEND: OperationKey<Value, Value> = operation!("email.send");
const OP_EMAIL_LINK: OperationKey<Value, Value> = operation!("email.link");
const OP_EMAIL_UNLINK: OperationKey<Value, Value> = operation!("email.unlink");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_EMAIL_SEND, OperationSpec::write("Send (or schedule) an e-mail from one of the acting user's mailboxes through the delivery queue: { to: [{ address, name? }], cc?, bcc?, mailbox_id?, subject?, body_html?, body_text?, template_id?, record?: { record_type, record_id }, reply_to_message_id?, attachment_file_ids?, scheduled_at?, tracking? }").durable(), send);
    app.operation(OP_EMAIL_LINK, OperationSpec::write("Link an e-mail conversation to a record by hand: { thread_id, record_type, record_id }").durable(), link);
    app.operation(OP_EMAIL_UNLINK, OperationSpec::write("Remove the link between an e-mail conversation and a record: { thread_id, record_type, record_id }").durable(), unlink);
}
