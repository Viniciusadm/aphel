use aphel_kernel::domain::Secret;
use aphel_kernel::errors::{AppError, CodedKind};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Clone, PartialEq, Default)]
pub struct MailboxCredentials(pub Map<String, Value>);

impl MailboxCredentials {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }

    pub fn to_secret(&self) -> Secret {
        Secret::new(Value::Object(self.0.clone()).to_string())
    }

    pub fn from_secret(secret: &Secret) -> Result<Self, AppError> {
        match serde_json::from_str::<Value>(secret.expose()) {
            Ok(Value::Object(values)) => Ok(Self(values)),
            _ => Err(AppError::Internal(
                "stored mailbox credentials are not a JSON object".to_owned(),
            )),
        }
    }
}

impl std::fmt::Debug for MailboxCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailboxCredentials").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
pub trait CredentialSink: Send + Sync {
    async fn persist(
        &self,
        mailbox_id: Uuid,
        credentials: &MailboxCredentials,
    ) -> Result<(), AppError>;
}

#[derive(Clone, Default)]
pub struct CredentialPersistence(Option<std::sync::Arc<dyn CredentialSink>>);

impl CredentialPersistence {
    pub fn new(sink: std::sync::Arc<dyn CredentialSink>) -> Self {
        Self(Some(sink))
    }
    pub async fn persist(
        &self,
        mailbox_id: Uuid,
        credentials: &MailboxCredentials,
    ) -> Result<(), AppError> {
        match &self.0 {
            Some(sink) => sink.persist(mailbox_id, credentials).await,
            None => Ok(()),
        }
    }
}

impl std::fmt::Debug for CredentialPersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CredentialPersistence")
            .field(&self.0.is_some())
            .finish()
    }
}

impl PartialEq for CredentialPersistence {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MailboxAccount {
    pub mailbox_id: Uuid,
    pub user_id: Uuid,
    pub address: String,
    pub settings: Value,
    pub credentials: MailboxCredentials,
    pub persistence: CredentialPersistence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailboxConnectionKind {
    Credentials,
    OAuth { start_path: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MailboxConnectRequest {
    pub user_id: Uuid,
    pub settings: Value,
    pub credentials: MailboxCredentials,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedMailbox {
    pub address: String,
    pub display_name: Option<String>,
    pub settings: Value,
    pub credentials: MailboxCredentials,
}

pub use crate::email::EmailAddress;

#[derive(Clone, PartialEq, Eq)]
pub struct EmailAttachmentContent {
    pub name: String,
    pub mime_type: String,
    pub content: Vec<u8>,
    pub content_id: Option<String>,
    pub inline: bool,
}

impl std::fmt::Debug for EmailAttachmentContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailAttachmentContent")
            .field("name", &self.name)
            .field("mime_type", &self.mime_type)
            .field("size", &self.content.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct FetchedEmail {
    pub provider_message_id: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub subject: String,
    pub from: Option<EmailAddress>,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub reply_to: Vec<EmailAddress>,
    pub date: Option<DateTime<Utc>>,
    pub text: Option<String>,
    pub html: Option<String>,
    pub attachments: Vec<EmailAttachmentContent>,
    pub content_type: Option<String>,
    pub auto_submitted: Option<String>,
    pub report_for_message_id: Option<String>,
    pub sent_by_account: bool,
}

impl std::fmt::Debug for FetchedEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchedEmail")
            .field("provider_message_id", &self.provider_message_id)
            .field("message_id", &self.message_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncRequest {
    pub cursor: Option<Value>,
    pub since: DateTime<Utc>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncBatch {
    pub messages: Vec<FetchedEmail>,
    pub cursor: Value,
    pub has_more: bool,
    pub credentials: Option<MailboxCredentials>,
}

#[derive(Clone, PartialEq)]
pub struct ProviderEmail {
    pub message_id: String,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub reply_to: Option<EmailAddress>,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub attachments: Vec<EmailAttachmentContent>,
    pub headers: Vec<(String, String)>,
}

impl std::fmt::Debug for ProviderEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderEmail")
            .field("message_id", &self.message_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SentEmail {
    pub message_id: String,
    pub provider_message_id: Option<String>,
    pub credentials: Option<MailboxCredentials>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailProviderErrorKind {
    Authentication,
    Connection,
    Rejected,
    Unsupported,
    Internal,
}

impl MailProviderErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Connection => "connection",
            Self::Rejected => "rejected",
            Self::Unsupported => "unsupported",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailProviderError {
    pub kind: MailProviderErrorKind,
    pub message: String,
}

impl MailProviderError {
    pub fn new(kind: MailProviderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn authentication(message: impl Into<String>) -> Self {
        Self::new(MailProviderErrorKind::Authentication, message)
    }

    pub fn connection(message: impl Into<String>) -> Self {
        Self::new(MailProviderErrorKind::Connection, message)
    }

    pub fn rejected(message: impl Into<String>) -> Self {
        Self::new(MailProviderErrorKind::Rejected, message)
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            MailProviderErrorKind::Connection | MailProviderErrorKind::Internal
        )
    }
}

impl std::fmt::Display for MailProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl From<MailProviderError> for AppError {
    fn from(error: MailProviderError) -> Self {
        let code = match error.kind {
            MailProviderErrorKind::Authentication => "mail_provider_authentication",
            MailProviderErrorKind::Connection => "mail_provider_unavailable",
            MailProviderErrorKind::Rejected => "mail_provider_rejected",
            MailProviderErrorKind::Unsupported => "mail_provider_unsupported",
            MailProviderErrorKind::Internal => return AppError::Internal(error.message),
        };
        AppError::Coded {
            kind: CodedKind::Validation,
            code,
            message: error.message,
        }
    }
}

#[async_trait]
pub trait MailboxProvider: Send + Sync {
    fn key(&self) -> &str;

    fn module(&self) -> Option<&str> {
        None
    }

    fn label(&self) -> &str;

    fn connection(&self) -> MailboxConnectionKind;

    async fn connect(
        &self,
        ctx: &crate::Ctx<'_>,
        request: MailboxConnectRequest,
    ) -> Result<ConnectedMailbox, MailProviderError>;

    async fn test(
        &self,
        ctx: &crate::Ctx<'_>,
        account: &MailboxAccount,
    ) -> Result<Option<MailboxCredentials>, MailProviderError>;

    async fn sync_since(
        &self,
        ctx: &crate::Ctx<'_>,
        account: &MailboxAccount,
        request: SyncRequest,
    ) -> Result<SyncBatch, MailProviderError>;

    async fn send(
        &self,
        ctx: &crate::Ctx<'_>,
        account: &MailboxAccount,
        email: &ProviderEmail,
    ) -> Result<SentEmail, MailProviderError>;
}

pub type SharedMailboxProvider = Arc<dyn MailboxProvider>;
