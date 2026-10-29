use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::application::shared::events::{Actor, EventCausation};
use crate::application::shared::record_ref::RecordRef;
use crate::errors::AppError;

pub const REDACTED: &str = "[redacted]";

#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub old: Value,
    pub new: Value,
}

impl FieldChange {
    pub fn new(field: impl Into<String>, old: Value, new: Value) -> Self {
        Self {
            field: field.into(),
            old,
            new,
        }
    }

    pub fn redacted(field: impl Into<String>) -> Self {
        Self::new(
            field,
            Value::String(REDACTED.to_owned()),
            Value::String(REDACTED.to_owned()),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditEntry {
    pub id: Uuid,
    pub actor: Actor,
    pub action: String,
    pub record: Option<RecordRef>,
    pub changes: Vec<FieldChange>,
    pub ip: Option<String>,
    pub causation: Option<EventCausation>,
    pub occurred_at: DateTime<Utc>,
}

impl AuditEntry {
    pub fn new(actor: Actor, action: impl Into<String>, occurred_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::now_v7(),
            actor,
            action: action.into(),
            record: None,
            changes: Vec::new(),
            ip: None,
            causation: None,
            occurred_at,
        }
    }

    pub fn with_record(mut self, record: RecordRef) -> Self {
        self.record = Some(record);
        self
    }

    pub fn with_changes(mut self, changes: Vec<FieldChange>) -> Self {
        self.changes = changes;
        self
    }

    pub fn with_ip(mut self, ip: Option<String>) -> Self {
        self.ip = ip;
        self
    }

    pub fn with_causation(mut self, causation: Option<EventCausation>) -> Self {
        self.causation = causation;
        self
    }
}

#[async_trait]
pub trait AuditLog<Tx: Send>: Send + Sync {
    async fn record(&self, tx: &mut Tx, entry: AuditEntry) -> Result<(), AppError>;
}

#[async_trait]
impl<Tx, A> AuditLog<Tx> for Arc<A>
where
    Tx: Send,
    A: AuditLog<Tx> + ?Sized,
{
    async fn record(&self, tx: &mut Tx, entry: AuditEntry) -> Result<(), AppError> {
        (**self).record(tx, entry).await
    }
}

pub struct CausedAuditLog<A> {
    inner: A,
    causation: EventCausation,
}

impl<A> CausedAuditLog<A> {
    pub fn new(inner: A, causation: EventCausation) -> Self {
        Self { inner, causation }
    }
}

#[async_trait]
impl<Tx, A> AuditLog<Tx> for CausedAuditLog<A>
where
    Tx: Send,
    A: AuditLog<Tx>,
{
    async fn record(&self, tx: &mut Tx, entry: AuditEntry) -> Result<(), AppError> {
        let causation = entry.causation.clone().or(Some(self.causation.clone()));
        self.inner.record(tx, entry.with_causation(causation)).await
    }
}
