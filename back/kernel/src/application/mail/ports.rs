use std::sync::Arc;

use async_trait::async_trait;

use crate::errors::AppError;

#[derive(Clone, PartialEq, Eq)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub html: String,
    pub text: String,
    pub calendar: Option<CalendarInvite>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for EmailAttachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailAttachment")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field("size", &self.bytes.len())
            .finish()
    }
}

pub const MAIL_ATTACHMENTS_UNSUPPORTED: &str = "mail_attachments_unsupported";

#[derive(Clone, PartialEq, Eq)]
pub struct CalendarInvite {
    pub method: String,
    pub content: String,
}

impl std::fmt::Debug for EmailMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailMessage")
            .field("subject", &self.subject)
            .finish_non_exhaustive()
    }
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError>;

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        if attachments.is_empty() {
            return self.send(message).await;
        }
        Err(AppError::validation_code(
            MAIL_ATTACHMENTS_UNSUPPORTED,
            "this mailer cannot send attachments",
        ))
    }
}

#[async_trait]
impl<M> Mailer for Arc<M>
where
    M: Mailer + ?Sized,
{
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        (**self).send(message).await
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        (**self).send_with_attachments(message, attachments).await
    }
}

pub type SharedMailer = Arc<dyn Mailer>;
