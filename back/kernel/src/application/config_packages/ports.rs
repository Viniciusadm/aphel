use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::application::config_packages::ConfigSectionRegistry;
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::Clock;
use crate::application::shared::{AuditLog, EventPublisher, TransactionManager};
use crate::domain::UserId;
use crate::errors::AppError;

pub const PIPELINE_ITEM: &str = "pipeline";
pub const PIPELINE_STAGE_ITEM: &str = "pipeline_stage";
pub const LOST_REASON_ITEM: &str = "lost_reason";
pub const TAG_ITEM: &str = "tag";
pub const SAVED_VIEW_ITEM: &str = "saved_view";

#[async_trait]
pub trait ConfigKeyRepository: Send + Sync {
    type Tx: Send;

    async fn keys(&self, item_type: &str) -> Result<HashMap<Uuid, String>, AppError>;
    async fn assign(
        &self,
        tx: &mut Self::Tx,
        item_type: &str,
        item_id: Uuid,
        key: &str,
    ) -> Result<(), AppError>;
}

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
