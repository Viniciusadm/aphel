use crate::domain::EmailAccess;
use crate::{
    EmailAttachmentView, EmailBody, EmailMessageView, ParticipantView, RecordLinkView, ThreadView,
};
use aphel_kernel::application::access::PERMISSION_DENIED;
use aphel_kernel::application::shared::{ByteStream, RecordRef, RecordType};
use aphel_kernel::errors::AppError;

pub(crate) const EMAIL_THREAD_NOT_FOUND: &str = "email_thread_not_found";
pub(crate) const EMAIL_ATTACHMENT_NOT_FOUND: &str = "email_attachment_not_found";
pub(crate) const EMAIL_THREAD_VISIBILITY_CHANGED: &str = "email_thread.visibility_changed";
pub(crate) const EMAIL_THREAD_RECORD_TYPE: RecordType = RecordType::from_static("email_thread");

pub(crate) fn thread_not_found() -> AppError {
    AppError::not_found_code(EMAIL_THREAD_NOT_FOUND, "e-mail conversation was not found")
}

pub(crate) fn attachment_not_found() -> AppError {
    AppError::not_found_code(EMAIL_ATTACHMENT_NOT_FOUND, "attachment was not found")
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThreadSummary {
    pub(crate) thread: ThreadView,
    pub(crate) access: EmailAccess,
    pub(crate) is_owner: bool,
    pub(crate) participants: Vec<ParticipantView>,
    pub(crate) records: Vec<RecordLinkView>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MessageDetail {
    pub(crate) message: EmailMessageView,
    pub(crate) body: Option<EmailBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThreadDetail {
    pub(crate) summary: ThreadSummary,
    pub(crate) messages: Vec<MessageDetail>,
}

pub(crate) struct AttachmentDownload {
    pub(crate) attachment: EmailAttachmentView,
    pub(crate) content: ByteStream,
}

pub(crate) fn sanitize_email_html(html: &str) -> String {
    ammonia::Builder::default()
        .add_tag_attributes("img", ["width", "height"])
        .add_tag_attributes("td", ["colspan", "rowspan", "align", "valign"])
        .add_tag_attributes("table", ["width", "cellpadding", "cellspacing", "border"])
        .link_rel(Some("noopener noreferrer nofollow"))
        .clean(html)
        .to_string()
}

pub(crate) fn record_access_denied(record: &RecordRef) -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        format!(
            "missing permission to access {} records",
            record.record_type
        ),
    )
}
