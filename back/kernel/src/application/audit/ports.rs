use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::shared::{Actor, EventCausation, FieldChange, RecordRef, RecordType};
use crate::errors::AppError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditLogFilter {
    pub record_type: Option<RecordType>,
    pub record_id: Option<Uuid>,
    pub actor_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditLogView {
    pub id: Uuid,
    pub actor: Actor,
    pub actor_name: Option<String>,
    pub actor_email: Option<String>,
    pub action: String,
    pub record: Option<RecordRef>,
    pub changes: Vec<FieldChange>,
    pub ip: Option<String>,
    pub causation: Option<EventCausation>,
    pub execution: Option<crate::application::shared::ExecutionContext>,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn list(
        &self,
        filter: &AuditLogFilter,
        window: ListWindow,
    ) -> Result<Listing<AuditLogView>, AppError>;
}
