use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::security::Clock;
use crate::domain::{SessionId, UserId};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPushSubscription {
    pub id: Uuid,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub user_agent: Option<String>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushTarget {
    pub id: Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushMessage {
    pub title: String,
    pub body: Option<String>,
    pub url: String,
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    Delivered,
    Gone,
    Failed(String),
}

#[async_trait]
pub trait PushSubscriptionRepository: Send + Sync {
    async fn upsert(&self, subscription: &NewPushSubscription) -> Result<(), AppError>;
    async fn remove(&self, user_id: &UserId, endpoint: &str) -> Result<bool, AppError>;
    async fn active_targets(
        &self,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<Vec<PushTarget>, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn mark_used(&self, id: Uuid, now: DateTime<Utc>) -> Result<(), AppError>;
    async fn claim_notification(
        &self,
        notification_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn release_notification(&self, notification_id: Uuid) -> Result<(), AppError>;
}

#[async_trait]
pub trait VapidKeys: Send + Sync {
    async fn public_key(&self) -> Result<String, AppError>;
}

#[async_trait]
pub trait PushSender: Send + Sync {
    async fn send(
        &self,
        target: &PushTarget,
        message: &PushMessage,
    ) -> Result<PushOutcome, AppError>;
}

pub trait PushPorts: Send + Sync {
    type Subscriptions: PushSubscriptionRepository;
    type Keys: VapidKeys;
    type Clock: Clock;

    fn subscriptions(&self) -> &Self::Subscriptions;
    fn keys(&self) -> &Self::Keys;
    fn clock(&self) -> &Self::Clock;
}
