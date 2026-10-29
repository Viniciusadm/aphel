use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::fields::permissions::FieldAccess;
use crate::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use crate::application::security::Clock;
use crate::application::shared::{AuditLog, EventPublisher, RecordType, TransactionManager};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRole {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub is_system: bool,
}

impl PermissionRole {
    pub fn is_admin(&self) -> bool {
        self.is_system && self.key == crate::domain::access::RoleKey::ADMIN
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFieldPermission {
    pub field_id: Uuid,
    pub role_id: Uuid,
    pub access: FieldAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPermissionState {
    pub entries: Vec<StoredFieldPermission>,
    pub updated_at: Option<DateTime<Utc>>,
    pub updated_by: Option<UserId>,
}

#[async_trait]
pub trait FieldPermissionRepository: Send + Sync {
    type Tx: Send;

    async fn roles(&self) -> Result<Vec<PermissionRole>, AppError>;
    async fn state(&self, record_type: &RecordType) -> Result<FieldPermissionState, AppError>;
    async fn replace(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        record_type: &RecordType,
        entries: &[StoredFieldPermission],
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

pub type FieldPermissionsTx<P> =
    <<P as FieldPermissionPorts>::Transactions as TransactionManager>::Tx;

pub trait FieldPermissionPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Definitions: FieldDefinitionRepository;
    type Permissions: FieldPermissionRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn definitions(&self) -> &Self::Definitions;
    fn permissions(&self) -> &Self::Permissions;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
