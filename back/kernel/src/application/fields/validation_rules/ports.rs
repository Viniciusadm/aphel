use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::fields::validation_rules::ValidationRule;
use crate::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use crate::application::security::Clock;
use crate::application::shared::{AuditLog, EventPublisher, RecordRef, TransactionManager};
use crate::domain::fields::{FieldDefinition, FieldValue};
use crate::errors::AppError;

#[async_trait]
pub trait ValidationRuleRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self, record_type: Option<&str>) -> Result<Vec<ValidationRule>, AppError>;
    async fn active_for(&self, record_type: &str) -> Result<Vec<ValidationRule>, AppError>;
    async fn find(&self, id: Uuid) -> Result<Option<ValidationRule>, AppError>;
    async fn find_by_key(&self, key: &str) -> Result<Option<ValidationRule>, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, rule: &ValidationRule) -> Result<(), AppError>;
    async fn update(&self, tx: &mut Self::Tx, rule: &ValidationRule) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, id: Uuid) -> Result<(), AppError>;
    async fn set_positions(
        &self,
        tx: &mut Self::Tx,
        positions: &[(Uuid, i32)],
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait ProposedFieldValues: Send + Sync {
    type Tx: Send;

    async fn load(
        &self,
        tx: &mut Self::Tx,
        record: &RecordRef,
        fields: &[FieldDefinition],
    ) -> Result<HashMap<Uuid, FieldValue>, AppError>;
}

pub type ValidationRulesTx<P> =
    <<P as ValidationRulePorts>::Transactions as TransactionManager>::Tx;

pub trait ValidationRulePorts: Send + Sync {
    type Transactions: TransactionManager;
    type Rules: ValidationRuleRepository<Tx = ValidationRulesTx<Self>>;
    type Definitions: FieldDefinitionRepository;
    type Events: EventPublisher<ValidationRulesTx<Self>>;
    type Audit: AuditLog<ValidationRulesTx<Self>>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn rules(&self) -> &Self::Rules;
    fn definitions(&self) -> &Self::Definitions;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
