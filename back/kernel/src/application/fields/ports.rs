use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::custom_objects::RecordLinkRepository;
use crate::application::fields::permissions::FieldAccessRule;
use crate::application::fields::{
    ConditionalRules, EntityDefinition, RecordPipeline, RecordTypeRegistry,
};
use crate::application::security::Clock;
use crate::application::shared::{
    AuditLog, EventPublisher, RecordRef, RecordType, TransactionManager,
};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::domain::fields::{FieldDefinition, FieldOption, FieldValue};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFieldValue {
    pub record_id: Uuid,
    pub field_id: Uuid,
    pub value: FieldValue,
}

#[async_trait]
pub trait FieldDefinitionRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self, record_type: &RecordType) -> Result<Vec<FieldDefinition>, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, field: &FieldDefinition) -> Result<(), AppError>;
    async fn update(&self, tx: &mut Self::Tx, field: &FieldDefinition) -> Result<(), AppError>;
    async fn reorder(
        &self,
        tx: &mut Self::Tx,
        record_type: &RecordType,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError>;
    async fn insert_option(
        &self,
        tx: &mut Self::Tx,
        field_id: Uuid,
        option: &FieldOption,
    ) -> Result<(), AppError>;
    async fn update_option(
        &self,
        tx: &mut Self::Tx,
        field_id: Uuid,
        option: &FieldOption,
    ) -> Result<(), AppError>;
    async fn reorder_options(
        &self,
        tx: &mut Self::Tx,
        field_id: Uuid,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError>;
    async fn versions(
        &self,
        _record_type: &RecordType,
    ) -> Result<HashMap<Uuid, DateTime<Utc>>, AppError> {
        Ok(HashMap::new())
    }
    async fn access_rules(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Vec<FieldAccessRule>, AppError>;
}

#[async_trait]
pub trait CustomFieldValueRepository: Send + Sync {
    type Tx: Send;

    async fn record_timestamp(
        &self,
        _definition: &EntityDefinition,
        _column: &str,
        _record_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(None)
    }

    async fn load(
        &self,
        record_type: &RecordType,
        fields: &[FieldDefinition],
        record_ids: &[Uuid],
    ) -> Result<Vec<StoredFieldValue>, AppError>;
    async fn upsert(
        &self,
        tx: &mut Self::Tx,
        record: &RecordRef,
        field: &FieldDefinition,
        value: &FieldValue,
    ) -> Result<(), AppError>;
    async fn delete(
        &self,
        tx: &mut Self::Tx,
        record: &RecordRef,
        field_id: Uuid,
    ) -> Result<(), AppError>;
    async fn delete_record(&self, tx: &mut Self::Tx, record: &RecordRef) -> Result<(), AppError>;
    async fn is_taken(
        &self,
        tx: &mut Self::Tx,
        definition: &EntityDefinition,
        record: &RecordRef,
        field: &FieldDefinition,
        value: &FieldValue,
    ) -> Result<bool, AppError>;
    async fn has_duplicates(
        &self,
        definition: &EntityDefinition,
        field: &FieldDefinition,
    ) -> Result<bool, AppError>;
    async fn reserve_unique(
        &self,
        _tx: &mut Self::Tx,
        _definition: &EntityDefinition,
        _record: &RecordRef,
        _field: &FieldDefinition,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn release_unique(
        &self,
        _tx: &mut Self::Tx,
        _record: &RecordRef,
        _field_id: Uuid,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn existing_records_in(
        &self,
        _tx: &mut Self::Tx,
        _scope: &AccessScope,
        _target: &EntityDefinition,
        _visibility: &RecordScopeFilter,
        _record_ids: &[Uuid],
    ) -> Result<Option<Vec<Uuid>>, AppError> {
        Ok(None)
    }
    async fn rebuild_unique(
        &self,
        _tx: &mut Self::Tx,
        _definition: &EntityDefinition,
        _field: &FieldDefinition,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

#[async_trait]
pub trait ReferenceChecker: Send + Sync {
    async fn existing_users(&self, user_ids: &[UserId]) -> Result<Vec<UserId>, AppError>;
    async fn existing_records(
        &self,
        scope: &AccessScope,
        target: &EntityDefinition,
        visibility: &RecordScopeFilter,
        record_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError>;
}

pub trait CustomFieldPorts: Send + Sync {
    type Tx: Send;
    type Definitions: FieldDefinitionRepository<Tx = Self::Tx>;
    type Values: CustomFieldValueRepository<Tx = Self::Tx>;
    type References: ReferenceChecker;
    type Rules: ConditionalRules;
    type Links: RecordLinkRepository<Tx = Self::Tx>;
    type Pipeline: RecordPipeline<Tx = Self::Tx>;

    fn registry(&self) -> &RecordTypeRegistry;
    fn definitions(&self) -> &Self::Definitions;
    fn values(&self) -> &Self::Values;
    fn references(&self) -> &Self::References;
    fn rules(&self) -> &Self::Rules;
    fn links(&self) -> &Self::Links;
    fn pipeline(&self) -> &Self::Pipeline;
}

pub type FieldsTx<P> = <<P as FieldPorts>::Transactions as TransactionManager>::Tx;

pub trait FieldPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Definitions: FieldDefinitionRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Values: CustomFieldValueRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn definitions(&self) -> &Self::Definitions;
    fn values(&self) -> &Self::Values;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
