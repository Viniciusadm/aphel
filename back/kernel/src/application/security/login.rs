use async_trait::async_trait;
use serde_json::json;

use crate::application::auth::ClientMetadata;
use crate::application::security::{NewSecurityEvent, SecurityEventKind, SecurityEventRecorder};
use crate::domain::SessionId;
use crate::domain::users::User;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginFailure {
    UnknownAccount,
    WrongPassword,
    InactiveAccount,
    InvalidTwoFactorCode,
}

impl LoginFailure {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnknownAccount => "unknown_account",
            Self::WrongPassword => "wrong_password",
            Self::InactiveAccount => "inactive_account",
            Self::InvalidTwoFactorCode => "invalid_two_factor_code",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAttempt {
    pub email: String,
    pub client: ClientMetadata,
}

impl LoginAttempt {
    pub fn new(email: &str, client: &ClientMetadata) -> Self {
        Self {
            email: email.trim().to_ascii_lowercase(),
            client: client.clone(),
        }
    }
}

#[async_trait]
pub trait LoginMonitor: Send + Sync {
    async fn ensure_allowed(&self, attempt: &LoginAttempt) -> Result<(), AppError>;
    async fn failed(
        &self,
        attempt: &LoginAttempt,
        user: Option<&User>,
        reason: LoginFailure,
    ) -> Result<(), AppError>;
    async fn succeeded(
        &self,
        user: &User,
        session_id: &SessionId,
        client: &ClientMetadata,
    ) -> Result<(), AppError>;
    async fn password_changed(&self, user: &User, client: &ClientMetadata) -> Result<(), AppError>;
}

pub struct RecordingLoginMonitor<R> {
    recorder: R,
}

impl<R> RecordingLoginMonitor<R> {
    pub fn new(recorder: R) -> Self {
        Self { recorder }
    }
}

#[async_trait]
impl<R> LoginMonitor for RecordingLoginMonitor<R>
where
    R: SecurityEventRecorder,
{
    async fn ensure_allowed(&self, _attempt: &LoginAttempt) -> Result<(), AppError> {
        Ok(())
    }

    async fn failed(
        &self,
        attempt: &LoginAttempt,
        user: Option<&User>,
        reason: LoginFailure,
    ) -> Result<(), AppError> {
        let mut event = NewSecurityEvent::new(SecurityEventKind::LoginFailed, chrono::Utc::now())
            .subject(&attempt.email)
            .client(
                attempt.client.ip.as_deref(),
                attempt.client.user_agent.as_deref(),
            )
            .metadata(json!({ "reason": reason.as_str() }));
        if let Some(user) = user {
            event = event.user(&user.id);
        }
        self.recorder.record(event).await
    }

    async fn succeeded(
        &self,
        user: &User,
        session_id: &SessionId,
        client: &ClientMetadata,
    ) -> Result<(), AppError> {
        self.recorder
            .record(
                NewSecurityEvent::new(SecurityEventKind::LoginSucceeded, chrono::Utc::now())
                    .user(&user.id)
                    .subject(user.email.as_str())
                    .client(client.ip.as_deref(), client.user_agent.as_deref())
                    .metadata(json!({ "session_id": session_id.as_str() })),
            )
            .await
    }

    async fn password_changed(&self, user: &User, client: &ClientMetadata) -> Result<(), AppError> {
        self.recorder
            .record(
                NewSecurityEvent::new(SecurityEventKind::PasswordChanged, chrono::Utc::now())
                    .user(&user.id)
                    .client(client.ip.as_deref(), client.user_agent.as_deref()),
            )
            .await
    }
}
