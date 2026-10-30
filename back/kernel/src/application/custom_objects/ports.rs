use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::fields::{
    CustomFieldPorts, FieldDefinitionRepository, RecordTypeDefinition, RecordTypeRegistry,
};
use crate::application::files::RecordAccessChecker;
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::query::RecordQueryPorts;
use crate::application::security::Clock;
use crate::application::shared::{AuditLog, EventPublisher, RecordRef, TransactionManager};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScope, RecordScopeFilter};
use crate::domain::custom_objects::{CustomObject, CustomRecord, RelationshipDefinition};
use crate::errors::AppError;

pub const CUSTOM_OBJECTS_MAX: usize = 50;
pub const CUSTOM_OBJECT_FIELDS_MAX: usize = 200;

#[async_trait]
pub trait CustomObjectRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self) -> Result<Vec<CustomObject>, AppError>;
    async fn find_by_key(&self, key: &str) -> Result<Option<CustomObject>, AppError>;
    async fn record_counts(&self) -> Result<HashMap<Uuid, u64>, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, object: &CustomObject) -> Result<(), AppError>;
    async fn update(&self, tx: &mut Self::Tx, object: &CustomObject) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, object: &CustomObject) -> Result<(), AppError>;
    async fn reorder(&self, tx: &mut Self::Tx, ordered_ids: &[Uuid]) -> Result<(), AppError>;
    async fn refresh_titles(
        &self,
        tx: &mut Self::Tx,
        object: &CustomObject,
        field_id: Uuid,
    ) -> Result<u64, AppError>;
    async fn bump_schema_version(&self, tx: &mut Self::Tx) -> Result<(), AppError>;
    async fn grant_default_permissions(
        &self,
        tx: &mut Self::Tx,
        record_type: &str,
        scope: Option<RecordScope>,
    ) -> Result<(), AppError>;
    async fn seed_system_fields(
        &self,
        tx: &mut Self::Tx,
        definition: &RecordTypeDefinition,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait RelationshipRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self) -> Result<Vec<RelationshipDefinition>, AppError>;
    async fn link_counts(&self) -> Result<HashMap<Uuid, u64>, AppError>;
    async fn insert(
        &self,
        tx: &mut Self::Tx,
        relationship: &RelationshipDefinition,
    ) -> Result<(), AppError>;
    async fn update(
        &self,
        tx: &mut Self::Tx,
        relationship: &RelationshipDefinition,
    ) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, relationship_id: Uuid) -> Result<u64, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordLink {
    pub relationship_id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSummary {
    pub record: RecordRef,
    pub title: Option<String>,
}

#[async_trait]
pub trait RecordLinkRepository: Send + Sync {
    type Tx: Send;

    #[allow(clippy::too_many_arguments)]
    async fn linked_ids(
        &self,
        scope: &AccessScope,
        target: &RecordTypeDefinition,
        visibility: &RecordScopeFilter,
        relationship_id: Uuid,
        inverse: bool,
        record_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<Uuid>, AppError>;
    async fn count_links(
        &self,
        relationship_id: Uuid,
        inverse: bool,
        record_id: Uuid,
    ) -> Result<u64, AppError>;
    async fn lock_links(
        &self,
        tx: &mut Self::Tx,
        relationship_id: Uuid,
        inverse: bool,
        record_id: Uuid,
    ) -> Result<u64, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, link: &RecordLink) -> Result<bool, AppError>;
    async fn delete(
        &self,
        tx: &mut Self::Tx,
        relationship_id: Uuid,
        from_id: Uuid,
        to_id: Uuid,
    ) -> Result<bool, AppError>;
    async fn delete_record_links(
        &self,
        tx: &mut Self::Tx,
        record_id: Uuid,
    ) -> Result<u64, AppError>;
    async fn ensure_deletable(&self, tx: &mut Self::Tx, record: &RecordRef)
    -> Result<(), AppError>;
    async fn titles(
        &self,
        definition: &RecordTypeDefinition,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError>;
}

pub type CustomObjectsTx<P> =
    <<P as CustomObjectAdminPorts>::Transactions as TransactionManager>::Tx;

pub trait CustomObjectAdminPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Objects: CustomObjectRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Relationships: RelationshipRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Definitions: FieldDefinitionRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn objects(&self) -> &Self::Objects;
    fn relationships(&self) -> &Self::Relationships;
    fn definitions(&self) -> &Self::Definitions;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}

#[async_trait]
pub trait CustomRecordRepository: Send + Sync {
    type Tx: Send;

    async fn find(
        &self,
        object_id: Uuid,
        record_id: Uuid,
    ) -> Result<Option<CustomRecord>, AppError>;
    async fn find_many(
        &self,
        object_id: Uuid,
        record_ids: &[Uuid],
    ) -> Result<Vec<CustomRecord>, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, record: &CustomRecord) -> Result<(), AppError>;
    async fn update(&self, tx: &mut Self::Tx, record: &CustomRecord) -> Result<(), AppError>;
    async fn set_archived(
        &self,
        tx: &mut Self::Tx,
        record_id: Uuid,
        archived_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn soft_delete(
        &self,
        tx: &mut Self::Tx,
        record_id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn lock(&self, _tx: &mut Self::Tx, _record_id: Uuid) -> Result<(), AppError> {
        Ok(())
    }
}

pub type RelationsTx<P> = <<P as RelationPorts>::Transactions as TransactionManager>::Tx;

pub trait RelationPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Links: RecordLinkRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Access: RecordAccessChecker;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn relationships(&self) -> &[RelationshipDefinition];
    fn transactions(&self) -> &Self::Transactions;
    fn links(&self) -> &Self::Links;
    fn access(&self) -> &Self::Access;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}

pub type CustomRecordsTx<P> = <<P as CustomRecordPorts>::Transactions as TransactionManager>::Tx;

pub trait CustomRecordPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Records: CustomRecordRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Links: RecordLinkRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Fields: CustomFieldPorts<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Query: RecordQueryPorts;
    type Access: RecordAccessChecker;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn objects(&self) -> &[CustomObject];
    fn relationships(&self) -> &[RelationshipDefinition];
    fn transactions(&self) -> &Self::Transactions;
    fn records(&self) -> &Self::Records;
    fn links(&self) -> &Self::Links;
    fn fields(&self) -> &Self::Fields;
    fn query(&self) -> &Self::Query;
    fn access(&self) -> &Self::Access;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
