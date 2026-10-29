use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::application::shared::record_ref::RecordRef;
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    User(UserId),
    System,
    ApiToken(String),
    Automation(String),
}

impl Actor {
    pub const USER: &'static str = "user";
    pub const SYSTEM: &'static str = "system";
    pub const API_TOKEN: &'static str = "api_token";
    pub const AUTOMATION: &'static str = "automation";

    pub fn from_scope(scope: &AccessScope) -> Self {
        if let Some(automation_id) = scope.automation_id() {
            return Self::Automation(automation_id.to_owned());
        }
        if scope.is_system() {
            return Self::System;
        }
        match scope.api_token_id() {
            Some(token_id) => Self::ApiToken(token_id.to_owned()),
            None => Self::User(scope.user_id().clone()),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::User(_) => Self::USER,
            Self::System => Self::SYSTEM,
            Self::ApiToken(_) => Self::API_TOKEN,
            Self::Automation(_) => Self::AUTOMATION,
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::User(id) => Some(id.as_str()),
            Self::System => None,
            Self::ApiToken(id) | Self::Automation(id) => Some(id),
        }
    }

    pub fn from_parts(kind: &str, id: Option<String>) -> Result<Self, AppError> {
        let missing_id = || AppError::Internal(format!("actor of kind {kind} requires an id"));
        match kind {
            Self::USER => Ok(Self::User(UserId::new(id.ok_or_else(missing_id)?)?)),
            Self::SYSTEM => Ok(Self::System),
            Self::API_TOKEN => Ok(Self::ApiToken(id.ok_or_else(missing_id)?)),
            Self::AUTOMATION => Ok(Self::Automation(id.ok_or_else(missing_id)?)),
            other => Err(AppError::Internal(format!("unknown actor kind {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCausation {
    pub run_id: Uuid,
    pub automation_id: Uuid,
    pub depth: u32,
    #[serde(default)]
    pub chain: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventOrigin {
    pub kind: String,
    pub id: Option<String>,
}

impl EventOrigin {
    pub fn new(kind: impl Into<String>, id: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainEvent {
    pub id: Uuid,
    pub actor: Actor,
    pub event_type: String,
    pub record: Option<RecordRef>,
    pub payload: Value,
    pub causation: Option<EventCausation>,
    pub origin: Option<EventOrigin>,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent {
    pub fn new(actor: Actor, event_type: impl Into<String>, occurred_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::now_v7(),
            actor,
            event_type: event_type.into(),
            record: None,
            payload: Value::Object(Default::default()),
            causation: None,
            origin: None,
            occurred_at,
        }
    }

    pub fn with_record(mut self, record: RecordRef) -> Self {
        self.record = Some(record);
        self
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_causation(mut self, causation: Option<EventCausation>) -> Self {
        self.causation = causation;
        self
    }

    pub fn with_origin(mut self, origin: Option<EventOrigin>) -> Self {
        self.origin = origin;
        self
    }
}

pub struct CausedEventPublisher<P> {
    inner: P,
    causation: EventCausation,
}

impl<P> CausedEventPublisher<P> {
    pub fn new(inner: P, causation: EventCausation) -> Self {
        Self { inner, causation }
    }
}

#[async_trait]
impl<Tx, P> EventPublisher<Tx> for CausedEventPublisher<P>
where
    Tx: Send,
    P: EventPublisher<Tx>,
{
    async fn publish(&self, tx: &mut Tx, event: DomainEvent) -> Result<(), AppError> {
        let causation = event.causation.clone().or(Some(self.causation.clone()));
        self.inner
            .publish(tx, event.with_causation(causation))
            .await
    }
}

pub struct OriginEventPublisher<P> {
    inner: P,
    origin: EventOrigin,
}

impl<P> OriginEventPublisher<P> {
    pub fn new(inner: P, origin: EventOrigin) -> Self {
        Self { inner, origin }
    }
}

#[async_trait]
impl<Tx, P> EventPublisher<Tx> for OriginEventPublisher<P>
where
    Tx: Send,
    P: EventPublisher<Tx>,
{
    async fn publish(&self, tx: &mut Tx, event: DomainEvent) -> Result<(), AppError> {
        let origin = event.origin.clone().or(Some(self.origin.clone()));
        self.inner.publish(tx, event.with_origin(origin)).await
    }
}

#[async_trait]
pub trait EventPublisher<Tx: Send>: Send + Sync {
    async fn publish(&self, tx: &mut Tx, event: DomainEvent) -> Result<(), AppError>;
}

#[async_trait]
impl<Tx, P> EventPublisher<Tx> for Arc<P>
where
    Tx: Send,
    P: EventPublisher<Tx> + ?Sized,
{
    async fn publish(&self, tx: &mut Tx, event: DomainEvent) -> Result<(), AppError> {
        (**self).publish(tx, event).await
    }
}

#[async_trait]
pub trait EventConsumer: Send + Sync {
    fn name(&self) -> &str;
    fn accepts(&self, event_type: &str) -> bool;
    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError>;
}
