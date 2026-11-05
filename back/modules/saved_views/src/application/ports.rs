use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_sdk::platform::application::query::{Filter, SortSpec};
use aphel_sdk::platform::application::security::Clock;
use aphel_sdk::platform::application::shared::{
    AuditLog, EventPublisher, RecordType, TransactionManager,
};
use aphel_sdk::platform::domain::access::AccessScope;

pub use aphel_sdk::views::{ViewType, ViewVisibility};

#[derive(Debug, Clone, PartialEq)]
pub struct SavedView {
    pub id: Uuid,
    pub record_type: RecordType,
    pub name: String,
    pub owner_id: Option<UserId>,
    pub visibility: ViewVisibility,
    pub team_id: Option<TeamId>,
    pub view_type: ViewType,
    pub filter: Option<Filter>,
    pub sort: Vec<SortSpec>,
    pub columns: Vec<String>,
    pub settings: Map<String, Value>,
    pub is_default: bool,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait SavedViewRepository: Send + Sync {
    type Tx: Send;

    async fn list_visible(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Vec<SavedView>, AppError>;
    async fn find(&self, view_id: Uuid) -> Result<Option<SavedView>, AppError>;
    async fn next_position(&self, record_type: &RecordType) -> Result<i32, AppError>;
    async fn team_exists(&self, team_id: &TeamId) -> Result<bool, AppError>;
    async fn user_default(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Option<Uuid>, AppError>;
    async fn insert(&self, tx: &mut Self::Tx, view: &SavedView) -> Result<(), AppError>;
    async fn update(&self, tx: &mut Self::Tx, view: &SavedView) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, view_id: Uuid) -> Result<(), AppError>;
    async fn clear_organization_default(
        &self,
        tx: &mut Self::Tx,
        record_type: &RecordType,
        except: Uuid,
    ) -> Result<(), AppError>;
    async fn set_user_default(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        view_id: Option<Uuid>,
    ) -> Result<(), AppError>;
}

pub type ViewsTx<P> = <<P as ViewPorts>::Transactions as TransactionManager>::Tx;

pub trait ViewPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Views: SavedViewRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Definitions: FieldDefinitionRepository;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn views(&self) -> &Self::Views;
    fn definitions(&self) -> &Self::Definitions;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
}
