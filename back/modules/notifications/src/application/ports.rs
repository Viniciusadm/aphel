use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{NotificationChannel, NotificationCursor, NotificationKind};
use aphel_kernel::application::files::UserReference;
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
pub struct NotificationView {
    pub id: Uuid,
    pub kind: NotificationKind,
    pub title: String,
    pub body: Option<String>,
    pub record: Option<RecordRef>,
    pub actor: Option<UserReference>,
    pub link: Option<String>,
    pub data: Value,
    pub read_at: Option<DateTime<Utc>>,
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
pub struct Recipient {
    pub user_id: UserId,
    pub display_name: String,
    pub email: String,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSummary {
    pub title: String,
    pub owner_id: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationClock {
    pub timezone: Tz,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueTask {
    pub id: Uuid,
    pub owner_id: UserId,
    pub title: String,
    pub due_at: DateTime<Utc>,
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

#[async_trait]
pub trait RecordFollowerSource: Send + Sync {
    async fn followers(
        &self,
        pool: &sqlx::MySqlPool,
        record: &RecordRef,
    ) -> Result<Vec<UserId>, AppError>;
}

#[async_trait]
pub trait NotificationDirectory: Send + Sync {
    async fn recipient(&self, user_id: &UserId) -> Result<Option<Recipient>, AppError>;
    async fn record_summary(&self, record: &RecordRef) -> Result<Option<RecordSummary>, AppError>;
    async fn can_read(&self, user_id: &UserId, record: &RecordRef) -> Result<bool, AppError>;
    async fn followers(&self, record: &RecordRef) -> Result<Vec<UserId>, AppError>;
    async fn organization_timezone(&self) -> Result<Tz, AppError>;
    async fn api_token_owner(&self, token_id: &str) -> Result<Option<UserId>, AppError>;
    async fn user_timezone(&self, user_id: &UserId) -> Result<Tz, AppError>;
    async fn open_tasks_due(
        &self,
        owner_id: Option<&UserId>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DueTask>, AppError>;
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
