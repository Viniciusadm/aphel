use crate::application::ConfigReferenceKind;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::application::ConfigSectionRegistry;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{AuditLog, EventPublisher, TransactionManager};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;

pub const SAVED_VIEW_ITEM: ConfigReferenceKind = ConfigReferenceKind::new("views.view");

pub use aphel_sdk::configuration::ConfigKeyRepository;

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigPackageInstall {
    pub id: Uuid,
    pub name: String,
    pub format_version: u32,
    pub template_key: Option<String>,
    pub package: Value,
    pub summary: Value,
    pub installed_by: Option<UserId>,
    pub installed_by_name: Option<String>,
    pub installed_at: DateTime<Utc>,
}

#[async_trait]
pub trait ConfigInstallRepository: Send + Sync {
    type Tx: Send;

    async fn insert(
        &self,
        tx: &mut Self::Tx,
        install: &ConfigPackageInstall,
    ) -> Result<(), AppError>;
    async fn list(&self, window: ListWindow) -> Result<Listing<ConfigPackageInstall>, AppError>;
    async fn find(&self, install_id: Uuid) -> Result<Option<ConfigPackageInstall>, AppError>;
    async fn organization_locale(&self) -> Result<Option<String>, AppError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageTemplate {
    pub key: String,
    pub package: Value,
}

pub trait TemplateCatalog: Send + Sync {
    fn templates(&self, locale: &str) -> Vec<PackageTemplate>;
    fn template(&self, key: &str, locale: &str) -> Option<PackageTemplate>;
}

pub type ConfigPackagesTx<P> = <<P as ConfigPackagePorts>::Transactions as TransactionManager>::Tx;

pub trait ConfigPackagePorts: Send + Sync {
    type Transactions: TransactionManager;
    type Installs: ConfigInstallRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Templates: TemplateCatalog;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn sections(&self) -> &ConfigSectionRegistry<<Self::Transactions as TransactionManager>::Tx>;
    fn transactions(&self) -> &Self::Transactions;
    fn installs(&self) -> &Self::Installs;
    fn templates(&self) -> &Self::Templates;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
