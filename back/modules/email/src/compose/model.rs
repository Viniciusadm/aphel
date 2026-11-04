use uuid::Uuid;

use crate::domain::address_domain;
use crate::{EmailAddress, EmailBody, EmailMessageView};
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const MAX_RECIPIENTS: usize = 50;
pub(crate) const MAX_SUBJECT_LENGTH: usize = 998;
pub(crate) const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_ATTACHMENTS: usize = 20;
pub(crate) const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
pub(crate) const MAX_SCHEDULE_DAYS: i64 = 365;
pub(crate) const MAX_FAILURE_REASON_LENGTH: usize = 500;

pub(crate) const EMAIL_NOT_FOUND: &str = "email_not_found";
pub(crate) const MAILBOX_NOT_FOUND: &str = "mailbox_not_found";
pub(crate) const NO_MAILBOX_AVAILABLE: &str = "no_mailbox_available";
pub(crate) const MAILBOX_UNAVAILABLE: &str = "mailbox_unavailable";
pub(crate) const EMAIL_NOT_CANCELLABLE: &str = "email_not_cancellable";
pub(crate) const EMAIL_ATTACHMENTS_TOO_LARGE: &str = "email_attachments_too_large";
pub(crate) const EMAIL_SCHEDULED: &str = "email.scheduled";
pub(crate) const EMAIL_CANCELLED: &str = "email.cancelled";
pub(crate) const EMAIL_FAILED: &str = "email.failed";
pub(crate) const SEND_SCHEDULED_JOB: &str = "email.send_scheduled";
pub(crate) const BODY_FILE_NAME: &str = "email-body.json";
const DEFAULT_MESSAGE_DOMAIN: &str = "aphel.local";

pub use aphel_sdk::email::{RecordInput, SendEmailCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct ComposedEmail {
    pub message: EmailMessageView,
    pub(crate) body: EmailBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnerEmailRequest {
    pub(crate) owner_id: String,
    pub(crate) record: RecordInput,
    pub(crate) to: Vec<EmailAddress>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    pub(crate) template_id: Option<String>,
    pub(crate) tracking: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SenderProfile {
    pub(crate) display_name: String,
    pub(crate) email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachmentFile {
    pub(crate) file_id: Uuid,
    pub(crate) name: String,
    pub(crate) mime_type: String,
    pub(crate) size: u64,
}

pub(crate) enum DeliveryOutcome {
    Skipped,
    Sent(Box<EmailMessageView>),
    Retry(AppError),
    Failed(AppError),
}

pub(crate) fn email_not_found() -> AppError {
    AppError::not_found_code(EMAIL_NOT_FOUND, "e-mail was not found")
}

pub(crate) fn mailbox_not_found() -> AppError {
    AppError::not_found_code(MAILBOX_NOT_FOUND, "mailbox was not found")
}

pub(crate) fn invalid_email(field: &str, message: impl Into<String>) -> AppError {
    AppError::invalid_fields("invalid e-mail", vec![FieldViolation::new(field, message)])
}

pub(crate) fn generate_message_id(address: &str) -> String {
    let domain = address_domain(address).unwrap_or(DEFAULT_MESSAGE_DOMAIN);
    format!("{}@{domain}", Uuid::now_v7().simple())
}
