use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::compose::html::sanitize_html;
use crate::compose::{MAX_BODY_BYTES, MAX_SUBJECT_LENGTH};
use crate::dto::email_outbound::SendEmailRequest;
use crate::dto::email_sync::{
    EmailMessageResponse, EmailThreadDetailResponse, LinkEmailThreadRequest,
};
use crate::hooks::OutgoingMessage;
use crate::service::{compose, linking};
use crate::{EmailAddress, EmailBody};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, OperationFuture, PluginHook, PluginHookKind, PluginOperation, forward_plugin_hook,
    operation_input, operation_output,
};

pub(crate) const OUTGOING: &str = "email.outgoing";
const EMAIL_RECORD_TYPE: &str = "email";

#[derive(Deserialize)]
struct LinkInput {
    thread_id: String,
    #[serde(flatten)]
    request: LinkEmailThreadRequest,
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

fn send<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
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

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::write(
            "email.send",
            "Send (or schedule) an e-mail from one of the acting user's mailboxes (needs a user actor): { to: [{ address, name? }], cc?, bcc?, mailbox_id?, subject?, body_html?, body_text?, template_id?, record?: { record_type, record_id }, reply_to_message_id?, attachment_file_ids?, scheduled_at?, tracking? }",
            send,
        )),
        Arc::new(PluginOperation::write(
            "email.link",
            "Link an e-mail conversation to a record by hand: { thread_id, record_type, record_id }",
            link,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "email.unlink",
            "Remove the link between an e-mail conversation and a record: { thread_id, record_type, record_id }",
            unlink,
        )
        .with_durable_mutations()),
    ]
}

fn addresses_json(addresses: &[EmailAddress]) -> Value {
    Value::Array(
        addresses
            .iter()
            .map(|address| json!({ "address": address.address, "name": address.name }))
            .collect(),
    )
}

fn outgoing_json(message: &OutgoingMessage) -> Value {
    json!({
        "mailbox_id": message.mailbox_id,
        "sender_id": message.sender_id.as_str(),
        "from": { "address": message.from.address, "name": message.from.name },
        "to": addresses_json(&message.to),
        "cc": addresses_json(&message.cc),
        "bcc": addresses_json(&message.bcc),
        "subject": message.subject,
        "body": { "text": message.body.text, "html": message.body.html },
        "record": message.record.as_ref().map(|record| json!({
            "record_type": record.record_type.as_str(),
            "record_id": record.record_id,
        })),
        "thread_id": message.thread_id,
        "scheduled": message.scheduled,
        "tracking": message.tracking,
    })
}

fn invalid_patch(message: &str) -> AppError {
    AppError::Validation(format!("invalid {OUTGOING} patch: {message}"))
}

fn patch_subject(value: &Value) -> Result<String, AppError> {
    let subject = value
        .as_str()
        .ok_or_else(|| invalid_patch("subject must be a string"))?
        .trim();
    if subject.is_empty() {
        return Err(invalid_patch("subject must not be empty"));
    }
    if subject.contains(['\r', '\n']) {
        return Err(invalid_patch("subject must be a single line"));
    }
    if subject.chars().count() > MAX_SUBJECT_LENGTH {
        return Err(invalid_patch("subject is too long"));
    }
    Ok(subject.to_owned())
}

fn patch_body_part(value: &Value, field: &str) -> Result<Option<String>, AppError> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.clone())),
        _ => Err(invalid_patch(&format!(
            "body.{field} must be a string or null"
        ))),
    }
}

fn apply_patch(message: &mut OutgoingMessage, patch: &Value) -> Result<(), AppError> {
    let Some(patch) = patch.as_object() else {
        return Ok(());
    };
    if let Some(subject) = patch.get("subject") {
        message.subject = patch_subject(subject)?;
    }
    let Some(body) = patch.get("body") else {
        return Ok(());
    };
    let body = body
        .as_object()
        .ok_or_else(|| invalid_patch("body must be an object with text and/or html"))?;
    let mut next = EmailBody {
        text: message.body.text.clone(),
        html: message.body.html.clone(),
    };
    if let Some(text) = body.get("text") {
        next.text = patch_body_part(text, "text")?;
    }
    if let Some(html) = body.get("html") {
        next.html = patch_body_part(html, "html")?.map(|html| sanitize_html(&html));
    }
    if next.text.is_none() && next.html.is_none() {
        return Err(invalid_patch("body must keep text or html"));
    }
    let size =
        next.text.as_ref().map_or(0, String::len) + next.html.as_ref().map_or(0, String::len);
    if size > MAX_BODY_BYTES {
        return Err(invalid_patch("body is too large"));
    }
    message.body = next;
    Ok(())
}

pub(crate) async fn outgoing(
    ctx: &mut HookCtx<'_>,
    mut message: OutgoingMessage,
) -> Result<OutgoingMessage, AppError> {
    let record_type = message
        .record
        .as_ref()
        .map_or(EMAIL_RECORD_TYPE.to_owned(), |record| {
            record.record_type.as_str().to_owned()
        });
    let record_id = message.record.as_ref().map_or_else(
        || {
            message
                .thread_id
                .map(|id| id.to_string())
                .unwrap_or_default()
        },
        |record| record.record_id.to_string(),
    );
    let patches = forward_plugin_hook(ctx, OUTGOING, &record_type, record_id, || {
        outgoing_json(&message)
    })
    .await?;
    for patch in &patches {
        apply_patch(&mut message, patch)?;
    }
    Ok(message)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![Arc::new(PluginHook::new(
        OUTGOING,
        EMAIL_RECORD_TYPE,
        PluginHookKind::Filter,
        "Before an e-mail is saved and sent; record_type is the linked record's type (or email when unlinked); values: { mailbox_id, sender_id, from, to, cc, bcc, subject, body: { text, html }, record, thread_id, scheduled, tracking }. Return { subject?, body?: { text?, html? } } to change them (html is sanitized)",
    ))]
}
