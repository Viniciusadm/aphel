use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::application::access::AccessRepository;
use crate::application::auth::two_factor::TwoFactorGate;
use crate::application::mail::Mailer;
use crate::application::organization_profile::OrganizationRepository;
use crate::application::security::{Clock, LoginMonitor, PasswordHasher, SecureTokens};
use crate::domain::users::User;
use crate::domain::{NonEmptyString, SessionId, UserId};
use crate::errors::AppError;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn set_password(&self, id: &UserId, password_hash: &str) -> Result<(), AppError>;
    async fn mark_email_verified(&self, id: &UserId, at: DateTime<Utc>) -> Result<(), AppError>;
    async fn record_login(&self, id: &UserId, at: DateTime<Utc>) -> Result<(), AppError>;
    async fn complete_account(
        &self,
        id: &UserId,
        display_name: &NonEmptyString,
        password_hash: &str,
        verified_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn activate(&self, id: &UserId, joined_at: DateTime<Utc>) -> Result<(), AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionStage {
    #[default]
    Full,
    TwoFactorPending,
}

impl SessionStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::TwoFactorPending => "two_factor_pending",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "full" => Some(Self::Full),
            "two_factor_pending" => Some(Self::TwoFactorPending),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSession {
    pub id: SessionId,
    pub user_id: UserId,
    pub token_hash: String,
    pub stage: SessionStage,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: SessionId,
    pub user_id: UserId,
    pub stage: SessionStage,
    pub two_factor_attempts: u32,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl SessionRecord {
    pub fn is_valid_at(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: NewSession) -> Result<(), AppError>;
    async fn find_by_token_hash(&self, token_hash: &str)
    -> Result<Option<SessionRecord>, AppError>;
    async fn touch(&self, id: &SessionId, at: DateTime<Utc>) -> Result<(), AppError>;
    async fn revoke(&self, id: &SessionId, at: DateTime<Utc>) -> Result<(), AppError>;
    async fn record_two_factor_failure(&self, id: &SessionId) -> Result<u32, AppError>;
    async fn revoke_all_for_user(
        &self,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserTokenPurpose {
    EmailVerification,
    PasswordReset,
    Invitation,
}

impl UserTokenPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
            Self::Invitation => "invitation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "email_verification" => Some(Self::EmailVerification),
            "password_reset" => Some(Self::PasswordReset),
            "invitation" => Some(Self::Invitation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUserToken {
    pub id: String,
    pub user_id: UserId,
    pub purpose: UserTokenPurpose,
    pub token_hash: String,
    pub payload: Option<Value>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserTokenRecord {
    pub id: String,
    pub user_id: UserId,
    pub purpose: UserTokenPurpose,
    pub payload: Option<Value>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl UserTokenRecord {
    pub fn is_usable_at(&self, now: DateTime<Utc>) -> bool {
        self.used_at.is_none() && self.expires_at > now
    }
}

#[async_trait]
pub trait UserTokenRepository: Send + Sync {
    async fn create(&self, token: NewUserToken) -> Result<(), AppError>;
    async fn find_by_hash(
        &self,
        purpose: UserTokenPurpose,
        token_hash: &str,
    ) -> Result<Option<UserTokenRecord>, AppError>;
    async fn mark_used(&self, id: &str, at: DateTime<Utc>) -> Result<bool, AppError>;
    async fn invalidate_for_user(
        &self,
        user_id: &UserId,
        purpose: UserTokenPurpose,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

pub trait AuthPorts: Send + Sync {
    type Users: UserRepository;
    type Sessions: SessionRepository;
    type Tokens: UserTokenRepository;
    type OrganizationProfile: OrganizationRepository;
    type Access: AccessRepository;
    type Hasher: PasswordHasher;
    type Secrets: SecureTokens;
    type Mailer: Mailer;
    type Clock: Clock;
    type Monitor: LoginMonitor;
    type TwoFactor: TwoFactorGate;

    fn users(&self) -> &Self::Users;
    fn sessions(&self) -> &Self::Sessions;
    fn tokens(&self) -> &Self::Tokens;
    fn organization_profile(&self) -> &Self::OrganizationProfile;
    fn access(&self) -> &Self::Access;
    fn hasher(&self) -> &Self::Hasher;
    fn secrets(&self) -> &Self::Secrets;
    fn mailer(&self) -> &Self::Mailer;
    fn clock(&self) -> &Self::Clock;
    fn monitor(&self) -> &Self::Monitor;
    fn two_factor(&self) -> &Self::TwoFactor;
}
