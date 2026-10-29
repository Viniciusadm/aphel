use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::files::FileRepository;
use crate::application::security::Clock;
use crate::application::shared::{AuditLog, EventPublisher, FileStorage, TransactionManager};
use crate::domain::access::{PermissionGrant, RoleKey};
use crate::domain::organizations::Organization;
use crate::domain::{Email, NonEmptyString, RoleId, UserId};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOrganization {
    pub name: NonEmptyString,
    pub locale: String,
    pub timezone: String,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOwner {
    pub id: UserId,
    pub email: Email,
    pub display_name: NonEmptyString,
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRole {
    pub id: RoleId,
    pub key: RoleKey,
    pub name: String,
    pub is_system: bool,
    pub grants: Vec<PermissionGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationRegistration {
    pub organization: NewOrganization,
    pub owner: NewOwner,
    pub roles: Vec<NewRole>,
    pub owner_role_id: RoleId,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationChanges {
    pub name: NonEmptyString,
    pub primary_color: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub currency: String,
    pub logo_file_id: Option<String>,
}

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn find(&self) -> Result<Option<Organization>, AppError>;
}

#[async_trait]
pub trait OrganizationSettingsRepository: Send + Sync {
    type Tx: Send;

    async fn find(&self) -> Result<Option<Organization>, AppError>;
    async fn update(
        &self,
        tx: &mut Self::Tx,
        changes: &OrganizationChanges,
    ) -> Result<(), AppError>;
}

pub type OrganizationTx<P> = <<P as OrganizationPorts>::Transactions as TransactionManager>::Tx;

pub trait OrganizationPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Organizations: OrganizationSettingsRepository<
        Tx = <Self::Transactions as TransactionManager>::Tx,
    >;
    type Files: FileRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Storage: FileStorage + ?Sized;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn organizations(&self) -> &Self::Organizations;
    fn files(&self) -> &Self::Files;
    fn storage(&self) -> &Self::Storage;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
