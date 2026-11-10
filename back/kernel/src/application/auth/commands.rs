use chrono::{DateTime, Duration, Utc};

use crate::application::auth::SessionRecord;
use crate::application::auth::two_factor::TwoFactorStatus;
use crate::domain::access::AccessScope;
use crate::domain::organization_profile::Organization;
use crate::domain::users::User;
use crate::domain::{Secret, SessionId};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientMetadata {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCommand {
    pub email: String,
    pub password: Secret,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetPasswordCommand {
    pub token: Secret,
    pub password: Secret,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptInvitationCommand {
    pub token: Secret,
    pub display_name: Option<String>,
    pub password: Secret,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedSession {
    pub token: Secret,
    pub session_id: SessionId,
    pub expires_at: DateTime<Utc>,
    pub account: AccountOverview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTwoFactor {
    pub token: Secret,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Session(Box<IssuedSession>),
    TwoFactorRequired(PendingTwoFactor),
}

impl LoginOutcome {
    pub fn into_session(self) -> Result<IssuedSession, AppError> {
        match self {
            Self::Session(session) => Ok(*session),
            Self::TwoFactorRequired(_) => Err(AppError::Internal(
                "two-factor authentication is required to finish this sign in".to_owned(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingChallenge {
    pub session: SessionRecord,
    pub user: User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedRequest {
    pub session_id: SessionId,
    pub user: User,
    pub scope: AccessScope,
    pub two_factor: TwoFactorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountOverview {
    pub user: User,
    pub organization: Option<Organization>,
    pub scope: AccessScope,
    pub two_factor: TwoFactorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationPreview {
    pub organization_name: String,
    pub email: String,
    pub requires_account: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthSettings {
    pub app_base_url: crate::application::organization_profile::PublicBaseUrl,
    pub session_ttl: Duration,
    pub email_verification_ttl: Duration,
    pub password_reset_ttl: Duration,
    pub invitation_ttl: Duration,
    pub session_touch_interval: Duration,
    pub two_factor_challenge_ttl: Duration,
    pub two_factor_max_attempts: u32,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            app_base_url: crate::application::organization_profile::PublicBaseUrl::default(),
            session_ttl: Duration::days(30),
            email_verification_ttl: Duration::hours(48),
            password_reset_ttl: Duration::hours(1),
            invitation_ttl: Duration::days(7),
            session_touch_interval: Duration::minutes(1),
            two_factor_challenge_ttl: Duration::minutes(10),
            two_factor_max_attempts: 5,
        }
    }
}

impl AuthSettings {
    pub fn link(&self, path: &str, token: &Secret) -> String {
        format!(
            "{}?token={}",
            self.app_base_url.app_link(path),
            token.expose()
        )
    }
}
