use serde_json::{Value, json};
use uuid::Uuid;

use crate::compose::html::sanitize_html;
use crate::compose::{MAX_BODY_BYTES, MAX_SUBJECT_LENGTH};
use crate::domain::{EmailDirection, EmailKind};
use crate::{EmailAddress, EmailBody, KEY};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{Hook, HookRef, PluginHookArgs, PluginHookFilter};
use aphel_sdk::platform::application::shared::RecordRef;

pub(crate) const OUTGOING: &str = "email.outgoing";
pub(crate) const EMAIL_RECORD_TYPE: &str = "email";

fn addresses_json(addresses: &[EmailAddress]) -> Value {
    Value::Array(
        addresses
            .iter()
            .map(|address| json!({ "address": address.address, "name": address.name }))
            .collect(),
    )
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

impl PluginHookArgs for OutgoingMessage {
    fn plugin_record_type(&self) -> Option<String> {
        Some(
            self.record
                .as_ref()
                .map_or(EMAIL_RECORD_TYPE.to_owned(), |record| {
                    record.record_type.as_str().to_owned()
                }),
        )
    }

    fn plugin_record_id(&self) -> String {
        self.record.as_ref().map_or_else(
            || self.thread_id.map(|id| id.to_string()).unwrap_or_default(),
            |record| record.record_id.to_string(),
        )
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "mailbox_id": self.mailbox_id,
            "sender_id": self.sender_id.as_str(),
            "from": { "address": self.from.address, "name": self.from.name },
            "to": addresses_json(&self.to),
            "cc": addresses_json(&self.cc),
            "bcc": addresses_json(&self.bcc),
            "subject": self.subject,
            "body": { "text": self.body.text, "html": self.body.html },
            "record": self.record.as_ref().map(|record| json!({
                "record_type": record.record_type.as_str(),
                "record_id": record.record_id,
            })),
            "thread_id": self.thread_id,
            "scheduled": self.scheduled,
            "tracking": self.tracking,
        })
    }
}

impl PluginHookFilter for OutgoingMessage {
    fn apply_plugin_patches(mut self, patches: &[Value]) -> Result<Self, AppError> {
        for patch in patches {
            apply_patch(&mut self, patch)?;
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutgoingMessage {
    pub(crate) mailbox_id: Uuid,
    pub(crate) sender_id: UserId,
    pub(crate) from: EmailAddress,
    pub(crate) to: Vec<EmailAddress>,
    pub(crate) cc: Vec<EmailAddress>,
    pub(crate) bcc: Vec<EmailAddress>,
    pub(crate) subject: String,
    pub(crate) body: EmailBody,
    pub(crate) record: Option<RecordRef>,
    pub(crate) thread_id: Option<Uuid>,
    pub(crate) scheduled: bool,
    pub(crate) tracking: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OutgoingEmail;

impl Hook for OutgoingEmail {
    type Args = OutgoingMessage;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkingEmail {
    pub(crate) mailbox_id: Uuid,
    pub(crate) owner_id: UserId,
    pub(crate) thread_id: Uuid,
    pub(crate) message_id: String,
    pub(crate) direction: EmailDirection,
    pub(crate) kind: EmailKind,
    pub(crate) from: Option<String>,
    pub(crate) participants: Vec<String>,
    pub(crate) contact_ids: Vec<Uuid>,
    pub(crate) records: Vec<RecordRef>,
}

#[derive(Debug, Clone, Copy)]
pub struct InboundLinking;

impl Hook for InboundLinking {
    type Args = LinkingEmail;
}

pub(crate) async fn apply_linking(
    ctx: &Ctx<'_>,
    email: LinkingEmail,
) -> Result<LinkingEmail, AppError> {
    apply(InboundLinking, ctx, email).await
}

async fn apply<K: HookRef>(hook: K, ctx: &Ctx<'_>, args: K::Args) -> Result<K::Args, AppError> {
    if !ctx.app().hooks().has_filters_except(hook, None) {
        return Ok(args);
    }
    let mut tx = ctx.begin().await?;
    let result = ctx.apply_hook(hook, &mut tx, KEY, args).await;
    tx.rollback().await?;
    result
}
