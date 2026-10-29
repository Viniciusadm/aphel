use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::fields::{RecordTypeDefinition, RecordTypeRegistry};
use crate::application::record_sharing::{
    RecordHeader, RecordShare, ShareAccess, ShareGranteeType,
};
use crate::application::security::{Clock, SharedSecurityEventRecorder};
use crate::application::shared::{AuditLog, EventPublisher, RecordRef, TransactionManager};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRecordShare {
    pub id: Uuid,
    pub record: RecordRef,
    pub grantee_type: ShareGranteeType,
    pub grantee_id: String,
    pub access: ShareAccess,
    pub shared_by: UserId,
    pub at: DateTime<Utc>,
}

#[async_trait]
pub trait RecordDirectory: Send + Sync {
    async fn header(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        visibility: &RecordScopeFilter,
        record_id: Uuid,
    ) -> Result<Option<RecordHeader>, AppError>;
    async fn visible_titles(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        visibility: &RecordScopeFilter,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Option<String>>, AppError>;
}

#[async_trait]
pub trait RecordShareRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self, record: &RecordRef) -> Result<Vec<RecordShare>, AppError>;
    async fn lock_list(
        &self,
        tx: &mut Self::Tx,
        record: &RecordRef,
    ) -> Result<Vec<RecordShare>, AppError>;
    async fn grantee_name(
        &self,
        grantee_type: ShareGranteeType,
        grantee_id: &str,
    ) -> Result<Option<String>, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, share: &NewRecordShare) -> Result<(), AppError>;
    async fn update_access(
        &self,
        tx: &mut Self::Tx,
        share_id: Uuid,
        access: ShareAccess,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, share_id: Uuid) -> Result<(), AppError>;
}

pub type RecordSharingTx<P> = <<P as RecordSharingPorts>::Transactions as TransactionManager>::Tx;

pub trait RecordSharingPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Shares: RecordShareRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Records: RecordDirectory;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn shares(&self) -> &Self::Shares;
    fn records(&self) -> &Self::Records;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
    fn security_events(&self) -> &SharedSecurityEventRecorder;
}
