use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::model::{
    ConnectedMailbox, MailboxAccount, MailboxConnectRequest, MailboxConnectionKind,
    MailboxCredentials, ProviderEmail, SentEmail, SyncBatch, SyncRequest,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, CodedKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailProviderErrorKind {
    Authentication,
    Connection,
    Rejected,
    Unsupported,
    Internal,
}

impl MailProviderErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
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

    pub(crate) fn authentication(message: impl Into<String>) -> Self {
        Self::new(MailProviderErrorKind::Authentication, message)
    }

    pub(crate) fn connection(message: impl Into<String>) -> Self {
        Self::new(MailProviderErrorKind::Connection, message)
    }

    pub(crate) fn rejected(message: impl Into<String>) -> Self {
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
        request: MailboxConnectRequest,
    ) -> Result<ConnectedMailbox, MailProviderError>;

    async fn test(
        &self,
        account: &MailboxAccount,
    ) -> Result<Option<MailboxCredentials>, MailProviderError>;

    async fn sync_since(
        &self,
        account: &MailboxAccount,
        request: SyncRequest,
    ) -> Result<SyncBatch, MailProviderError>;

    async fn send(
        &self,
        account: &MailboxAccount,
        email: &ProviderEmail,
    ) -> Result<SentEmail, MailProviderError>;
}

pub type SharedMailboxProvider = Arc<dyn MailboxProvider>;

#[derive(Clone, Default)]
pub struct MailboxProviderRegistry {
    providers: Vec<SharedMailboxProvider>,
}

impl MailboxProviderRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, provider: SharedMailboxProvider) {
        self.providers
            .retain(|existing| existing.key() != provider.key());
        self.providers.push(provider);
    }

    pub fn find(&self, key: &str) -> Option<SharedMailboxProvider> {
        self.providers
            .iter()
            .find(|provider| provider.key() == key)
            .cloned()
    }

    pub(crate) fn available_for(&self, scope: &AccessScope) -> Vec<SharedMailboxProvider> {
        self.providers
            .iter()
            .filter(|provider| {
                provider
                    .module()
                    .is_none_or(|module| scope.module_enabled(module))
            })
            .cloned()
            .collect()
    }

    pub(crate) fn require_for(
        &self,
        scope: &AccessScope,
        key: &str,
    ) -> Result<SharedMailboxProvider, AppError> {
        self.available_for(scope)
            .into_iter()
            .find(|provider| provider.key() == key)
            .ok_or_else(|| AppError::Validation(format!("mail provider {key} is not available")))
    }
}

impl std::fmt::Debug for MailboxProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.providers
                    .iter()
                    .map(|provider| provider.key().to_owned()),
            )
            .finish()
    }
}

pub(crate) fn mailbox_settings_or_empty(settings: Option<Value>) -> Value {
    match settings {
        Some(Value::Object(values)) => Value::Object(values),
        _ => Value::Object(Default::default()),
    }
}
