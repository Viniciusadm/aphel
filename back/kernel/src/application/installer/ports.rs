use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::organizations::{OrganizationConfig, OrganizationRegistration};
use crate::application::security::{Clock, PasswordHasher};
use crate::application::shared::{AuditLog, EventPublisher, TransactionManager};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstOrganization {
    pub registration: OrganizationRegistration,
    pub configuration: OrganizationConfig,
    pub email_verified_at: DateTime<Utc>,
}

#[async_trait]
pub trait InstallationRepository: Send + Sync {
    type Tx: Send;

    async fn is_available(&self) -> Result<bool, AppError>;
    async fn claim(&self, tx: &mut Self::Tx) -> Result<bool, AppError>;
    async fn create_first_organization(
        &self,
        tx: &mut Self::Tx,
        organization: &FirstOrganization,
    ) -> Result<(), AppError>;
}

pub trait InstallerPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Installation: InstallationRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Hasher: PasswordHasher;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn installation(&self) -> &Self::Installation;
    fn hasher(&self) -> &Self::Hasher;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
