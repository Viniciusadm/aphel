use async_trait::async_trait;
use serde_json::Value;

use crate::application::module_admin::{ModuleDescriptor, ModuleState};
use crate::application::secrets::SecretVault;
use crate::application::security::Clock;
use crate::application::shared::{AuditLog, EventPublisher, TransactionManager};
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};

#[async_trait]
pub trait ModuleStateRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self) -> Result<Vec<ModuleState>, AppError>;
    async fn find(&self, key: &str) -> Result<Option<ModuleState>, AppError>;
    async fn save(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        module: &ModuleState,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait ModuleLifecycle: Send + Sync {
    type Tx: Send;

    fn descriptors(&self) -> Vec<ModuleDescriptor>;
    fn validate_settings(&self, key: &str, settings: &Value) -> Vec<FieldViolation>;
    async fn held_modules(&self) -> Result<std::collections::HashMap<String, String>, AppError>;
    async fn extension_dependents(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<String>>, AppError>;
    async fn on_enable(&self, scope: &AccessScope, key: &str) -> Result<(), AppError>;
    async fn on_disable(&self, scope: &AccessScope, key: &str) -> Result<(), AppError>;
    async fn export_data(&self, scope: &AccessScope, key: &str) -> Result<Value, AppError>;
    async fn purge_data(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        key: &str,
    ) -> Result<u64, AppError>;
}

pub trait ModulePorts: Send + Sync {
    type Transactions: TransactionManager;
    type Modules: ModuleStateRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Runtime: ModuleLifecycle<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Secrets: SecretVault<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn modules(&self) -> &Self::Modules;
    fn runtime(&self) -> &Self::Runtime;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn secrets(&self) -> &Self::Secrets;
    fn clock(&self) -> &Self::Clock;
}
