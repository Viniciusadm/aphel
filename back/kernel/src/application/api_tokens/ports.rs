use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::access::AccessRepository;
use crate::application::auth::UserRepository;
use crate::application::security::{Clock, SecureTokens};
use crate::application::shared::{AuditLog, EventPublisher, TransactionManager};
use crate::domain::UserId;
use crate::domain::api_tokens::ApiTokenScopes;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiToken {
    pub id: Uuid,
    pub created_by: UserId,
    pub created_by_name: Option<String>,
    pub name: String,
    pub token_prefix: String,
    pub scopes: ApiTokenScopes,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_used_ip: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ApiToken {
    pub fn is_usable_at(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at.is_none_or(|expires_at| expires_at > now)
    }
}

#[async_trait]
pub trait ApiTokenRepository: Send + Sync {
    type Tx: Send;

    async fn insert(
        &self,
        tx: &mut Self::Tx,
        token: &ApiToken,
        token_hash: &str,
    ) -> Result<(), AppError>;
    async fn list(&self, created_by: Option<&UserId>) -> Result<Vec<ApiToken>, AppError>;
    async fn find(&self, id: Uuid) -> Result<Option<ApiToken>, AppError>;
    async fn count_usable(&self, created_by: &UserId, now: DateTime<Utc>) -> Result<u64, AppError>;
    async fn revoke(
        &self,
        tx: &mut Self::Tx,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, AppError>;
    async fn record_use(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
        ip: Option<&str>,
    ) -> Result<(), AppError>;
}

pub trait ApiTokenPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Tokens: ApiTokenRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Users: UserRepository;
    type Access: AccessRepository;
    type Secrets: SecureTokens;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn tokens(&self) -> &Self::Tokens;
    fn users(&self) -> &Self::Users;
    fn access(&self) -> &Self::Access;
    fn secrets(&self) -> &Self::Secrets;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
