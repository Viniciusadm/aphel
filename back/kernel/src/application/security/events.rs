use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::UserId;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityEventKind {
    LoginSucceeded,
    LoginFailed,
    LoginLocked,
    TwoFactorEnabled,
    TwoFactorDisabled,
    TwoFactorChallengeFailed,
    RecoveryCodeUsed,
    RecoveryCodesRegenerated,
    PasswordChanged,
    SessionRevoked,
    ApiTokenCreated,
    ApiTokenRevoked,
    PermissionChanged,
}

impl SecurityEventKind {
    pub const ALL: [Self; 13] = [
        Self::LoginSucceeded,
        Self::LoginFailed,
        Self::LoginLocked,
        Self::TwoFactorEnabled,
        Self::TwoFactorDisabled,
        Self::TwoFactorChallengeFailed,
        Self::RecoveryCodeUsed,
        Self::RecoveryCodesRegenerated,
        Self::PasswordChanged,
        Self::SessionRevoked,
        Self::ApiTokenCreated,
        Self::ApiTokenRevoked,
        Self::PermissionChanged,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginSucceeded => "login_succeeded",
            Self::LoginFailed => "login_failed",
            Self::LoginLocked => "login_locked",
            Self::TwoFactorEnabled => "two_factor_enabled",
            Self::TwoFactorDisabled => "two_factor_disabled",
            Self::TwoFactorChallengeFailed => "two_factor_challenge_failed",
            Self::RecoveryCodeUsed => "recovery_code_used",
            Self::RecoveryCodesRegenerated => "recovery_codes_regenerated",
            Self::PasswordChanged => "password_changed",
            Self::SessionRevoked => "session_revoked",
            Self::ApiTokenCreated => "api_token_created",
            Self::ApiTokenRevoked => "api_token_revoked",
            Self::PermissionChanged => "permission_changed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSecurityEvent {
    pub id: Uuid,
    pub kind: SecurityEventKind,
    pub user_id: Option<UserId>,
    pub subject: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Value,
    pub occurred_at: DateTime<Utc>,
}

impl NewSecurityEvent {
    pub fn new(kind: SecurityEventKind, occurred_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::now_v7(),
            kind,
            user_id: None,
            subject: None,
            ip: None,
            user_agent: None,
            metadata: json!({}),
            occurred_at,
        }
    }

    pub fn user(mut self, user_id: &UserId) -> Self {
        self.user_id = Some(user_id.clone());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn client(mut self, ip: Option<&str>, user_agent: Option<&str>) -> Self {
        self.ip = ip.map(str::to_owned);
        self.user_agent = user_agent.map(str::to_owned);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[async_trait]
pub trait SecurityEventRecorder: Send + Sync {
    async fn record(&self, event: NewSecurityEvent) -> Result<(), AppError>;
}

pub type SharedSecurityEventRecorder = Arc<dyn SecurityEventRecorder>;

#[async_trait]
impl<T> SecurityEventRecorder for Arc<T>
where
    T: SecurityEventRecorder + ?Sized,
{
    async fn record(&self, event: NewSecurityEvent) -> Result<(), AppError> {
        (**self).record(event).await
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopSecurityEventRecorder;

#[async_trait]
impl SecurityEventRecorder for NoopSecurityEventRecorder {
    async fn record(&self, _event: NewSecurityEvent) -> Result<(), AppError> {
        Ok(())
    }
}

pub async fn record_quietly<R>(recorder: &R, event: NewSecurityEvent)
where
    R: SecurityEventRecorder + ?Sized,
{
    let kind = event.kind;
    if let Err(error) = recorder.record(event).await {
        tracing::warn!(%error, kind = kind.as_str(), "failed to record security event");
    }
}
