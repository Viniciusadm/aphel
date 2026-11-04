pub use aphel_sdk::notifications::{
    DueTask, NotificationDirectory, NotificationView, Recipient, RecordFollowerSource,
    RecordSummary,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{NotificationChannel, NotificationCursor, NotificationKind};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{JobQueue, RecordRef, TransactionManager};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, PartialEq)]
pub struct NewNotification {
    pub id: Uuid,
    pub user_id: UserId,
    pub kind: NotificationKind,
    pub title: String,
    pub body: Option<String>,
    pub record: Option<RecordRef>,
    pub actor_id: Option<UserId>,
    pub link: Option<String>,
    pub data: Value,
    pub dedupe_key: String,
    pub in_app: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeliverableNotification {
    pub user_id: UserId,
    pub view: NotificationView,
    pub emailed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotificationFilter {
    pub unread_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationPage {
    pub items: Vec<NotificationView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenceSetting {
    pub kind: NotificationKind,
    pub channel: NotificationChannel,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationClock {
    pub timezone: chrono_tz::Tz,
}

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    type Tx: Send;

    async fn insert(
        &self,
        tx: &mut Self::Tx,
        notification: &NewNotification,
    ) -> Result<bool, AppError>;
    async fn list(
        &self,
        scope: &AccessScope,
        filter: NotificationFilter,
        after: Option<NotificationCursor>,
        limit: u32,
    ) -> Result<Vec<NotificationView>, AppError>;
    async fn count_unread(&self, scope: &AccessScope) -> Result<u64, AppError>;
    async fn mark_read(
        &self,
        scope: &AccessScope,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn mark_all_read(&self, scope: &AccessScope, now: DateTime<Utc>)
    -> Result<u64, AppError>;
    async fn find_deliverable(&self, id: Uuid)
    -> Result<Option<DeliverableNotification>, AppError>;
    async fn mark_emailed(&self, id: Uuid, now: DateTime<Utc>) -> Result<(), AppError>;
}

#[async_trait]
pub trait NotificationPreferenceRepository: Send + Sync {
    async fn stored(&self, user_id: &UserId) -> Result<Vec<PreferenceSetting>, AppError>;
    async fn save(
        &self,
        scope: &AccessScope,
        settings: &[PreferenceSetting],
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn subscribers(
        &self,
        kind: NotificationKind,
        channel: NotificationChannel,
    ) -> Result<Vec<UserId>, AppError>;
}

pub type NotificationsTx<P> = <<P as NotificationPorts>::Transactions as TransactionManager>::Tx;

pub trait NotificationPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Notifications: NotificationRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Preferences: NotificationPreferenceRepository;
    type Directory: NotificationDirectory;
    type Jobs: JobQueue<<Self::Transactions as TransactionManager>::Tx>;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn notifications(&self) -> &Self::Notifications;
    fn preferences(&self) -> &Self::Preferences;
    fn directory(&self) -> &Self::Directory;
    fn jobs(&self) -> &Self::Jobs;
    fn clock(&self) -> &Self::Clock;
}
