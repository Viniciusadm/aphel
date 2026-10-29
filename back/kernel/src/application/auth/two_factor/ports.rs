use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::auth::ClientMetadata;
use crate::application::auth::two_factor::{TwoFactorPolicy, TwoFactorStatus};
use crate::application::security::{Clock, PasswordHasher, SecureTokens, SecurityEventRecorder};
use crate::application::shared::{AuditLog, EventPublisher, TransactionManager};
use crate::domain::access::AccessScope;
use crate::domain::users::User;
use crate::domain::{Secret, UserId};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoFactorRecord {
    pub user_id: UserId,
    pub secret: Secret,
    pub enabled_at: Option<DateTime<Utc>>,
    pub last_used_step: Option<u64>,
}

impl TwoFactorRecord {
    pub fn is_enabled(&self) -> bool {
        self.enabled_at.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoFactorRequirement {
    pub enabled: bool,
    pub policy: TwoFactorPolicy,
}

#[async_trait]
pub trait TwoFactorRepository: Send + Sync {
    async fn find(&self, user_id: &UserId) -> Result<Option<TwoFactorRecord>, AppError>;
    async fn requirement(&self, user_id: &UserId) -> Result<TwoFactorRequirement, AppError>;
    async fn save_pending(
        &self,
        user_id: &UserId,
        secret: &Secret,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn enable(
        &self,
        user_id: &UserId,
        step: u64,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn claim_step(&self, user_id: &UserId, step: u64) -> Result<bool, AppError>;
    async fn consume_recovery_code(
        &self,
        user_id: &UserId,
        code_hash: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn replace_recovery_codes(
        &self,
        user_id: &UserId,
        code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn remaining_recovery_codes(&self, user_id: &UserId) -> Result<u32, AppError>;
    async fn disable(&self, user_id: &UserId) -> Result<(), AppError>;
}

#[async_trait]
pub trait TwoFactorPolicyRepository: Send + Sync {
    type Tx: Send;

    async fn find(&self) -> Result<TwoFactorPolicy, AppError>;
    async fn save(
        &self,
        tx: &mut Self::Tx,
        updated_by: &UserId,
        policy: TwoFactorPolicy,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn members_without_two_factor(&self, policy: TwoFactorPolicy) -> Result<u64, AppError>;
}

pub trait OneTimePasswords: Send + Sync {
    fn generate_secret(&self) -> Result<Secret, AppError>;
    fn provisioning_uri(
        &self,
        secret: &Secret,
        account: &str,
        issuer: &str,
    ) -> Result<String, AppError>;
    fn qr_code_svg(&self, uri: &str) -> Result<String, AppError>;
    fn matching_step(
        &self,
        secret: &Secret,
        code: &str,
        at: DateTime<Utc>,
    ) -> Result<Option<u64>, AppError>;
    fn recovery_code(&self) -> Result<Secret, AppError>;
}

#[async_trait]
pub trait TwoFactorGate: Send + Sync {
    async fn status(
        &self,
        user_id: &UserId,
        scope: &AccessScope,
    ) -> Result<TwoFactorStatus, AppError>;
    async fn is_enabled(&self, user_id: &UserId) -> Result<bool, AppError>;
    async fn verify_challenge(
        &self,
        user: &User,
        code: &str,
        client: &ClientMetadata,
    ) -> Result<bool, AppError>;
}

pub type TwoFactorTx<P> = <<P as TwoFactorPorts>::Transactions as TransactionManager>::Tx;

pub trait TwoFactorPorts: Send + Sync {
    type Store: TwoFactorRepository;
    type Passwords: OneTimePasswords;
    type Hasher: PasswordHasher;
    type Digests: SecureTokens;
    type Clock: Clock;
    type Recorder: SecurityEventRecorder;
    type Transactions: TransactionManager;
    type Policies: TwoFactorPolicyRepository<Tx = TwoFactorTx<Self>>;
    type Events: EventPublisher<TwoFactorTx<Self>>;
    type Audit: AuditLog<TwoFactorTx<Self>>;

    fn store(&self) -> &Self::Store;
    fn passwords(&self) -> &Self::Passwords;
    fn hasher(&self) -> &Self::Hasher;
    fn digests(&self) -> &Self::Digests;
    fn clock(&self) -> &Self::Clock;
    fn recorder(&self) -> &Self::Recorder;
    fn transactions(&self) -> &Self::Transactions;
    fn policies(&self) -> &Self::Policies;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
}
