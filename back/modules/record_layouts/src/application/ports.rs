use async_trait::async_trait;

use crate::application::{LayoutContext, RulesSource, StoredFormLayout};
use aphel_kernel::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{AuditLog, EventPublisher, RecordType, TransactionManager};
use aphel_kernel::errors::AppError;

#[async_trait]
pub trait FormLayoutRepository: Send + Sync {
    type Tx: Send;

    async fn find(
        &self,
        record_type: &RecordType,
        context: LayoutContext,
    ) -> Result<Option<StoredFormLayout>, AppError>;
    async fn rules_source(
        &self,
        record_type: &RecordType,
        contexts: &[LayoutContext],
    ) -> Result<RulesSource, AppError>;
    async fn upsert(&self, tx: &mut Self::Tx, layout: &StoredFormLayout) -> Result<(), AppError>;
    async fn delete(
        &self,
        tx: &mut Self::Tx,
        record_type: &RecordType,
        context: LayoutContext,
    ) -> Result<(), AppError>;
}

pub type LayoutsTx<P> = <<P as LayoutPorts>::Transactions as TransactionManager>::Tx;

pub trait LayoutPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Layouts: FormLayoutRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Definitions: FieldDefinitionRepository;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn layouts(&self) -> &Self::Layouts;
    fn definitions(&self) -> &Self::Definitions;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
