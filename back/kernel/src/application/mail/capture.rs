use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::application::mail::{EmailAttachment, EmailMessage, Mailer, SharedMailer};
use crate::errors::AppError;

pub const MAIL_NOT_CONFIGURED: &str = "mail_not_configured";

pub fn is_mail_not_configured(error: &AppError) -> bool {
    matches!(error, AppError::Coded { code, .. } if *code == MAIL_NOT_CONFIGURED)
}

#[derive(Clone)]
pub struct UndeliveredMailCapture {
    inner: SharedMailer,
    undelivered: Arc<Mutex<Vec<EmailMessage>>>,
}

impl UndeliveredMailCapture {
    pub fn new(inner: SharedMailer) -> Self {
        Self {
            inner,
            undelivered: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn shared(&self) -> SharedMailer {
        Arc::new(self.clone())
    }

    pub fn undelivered(&self) -> Vec<EmailMessage> {
        self.undelivered
            .lock()
            .map(|messages| messages.clone())
            .unwrap_or_default()
    }

    pub fn undelivered_link(&self, prefix: &str) -> Option<String> {
        self.undelivered()
            .iter()
            .find_map(|message| find_link(&message.text, prefix))
    }
}

pub fn find_link(text: &str, prefix: &str) -> Option<String> {
    let start = text.find(prefix)?;
    let link: String = text[start..]
        .chars()
        .take_while(|character| !character.is_whitespace() && !matches!(character, '"' | '<' | '>'))
        .collect();
    Some(link)
}

#[async_trait]
impl Mailer for UndeliveredMailCapture {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        match self.inner.send(message.clone()).await {
            Err(error) if is_mail_not_configured(&error) => {
                if let Ok(mut undelivered) = self.undelivered.lock() {
                    undelivered.push(message);
                }
                Err(error)
            }
            other => other,
        }
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        match self
            .inner
            .send_with_attachments(message.clone(), attachments)
            .await
        {
            Err(error) if is_mail_not_configured(&error) => {
                if let Ok(mut undelivered) = self.undelivered.lock() {
                    undelivered.push(message);
                }
                Err(error)
            }
            other => other,
        }
    }
}
