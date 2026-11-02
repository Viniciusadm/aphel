use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use crate::domain::access::AccessScope;
use crate::domain::custom_objects::{RelationSide, RelationshipDefinition};
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use serde_json::Value;
pub fn custom_object_not_found() -> AppError {
    AppError::not_found_code("custom_object_not_found", "custom object was not found")
}
pub struct ChangeRecord {
    pub record: RecordRef,
    pub event_type: String,
    pub payload: Value,
    pub changes: Vec<FieldChange>,
    pub ip: Option<String>,
}

pub async fn publish_change<Tx, E, A>(
    events: &E,
    audit: &A,
    tx: &mut Tx,
    scope: &AccessScope,
    now: DateTime<Utc>,
    change: ChangeRecord,
) -> Result<(), AppError>
where
    Tx: Send,
    E: EventPublisher<Tx> + ?Sized,
    A: AuditLog<Tx> + ?Sized,
{
    let actor = Actor::from_scope(scope);
    events
        .publish(
            tx,
            DomainEvent::new(actor.clone(), change.event_type.clone(), now)
                .with_record(change.record.clone())
                .with_payload(change.payload),
        )
        .await?;
    audit
        .record(
            tx,
            AuditEntry::new(actor, change.event_type, now)
                .with_record(change.record)
                .with_changes(change.changes)
                .with_ip(change.ip),
        )
        .await
}

pub fn relation_sides(
    relationships: &[RelationshipDefinition],
    record_type: &str,
) -> Vec<(RelationshipDefinition, RelationSide)> {
    relationships
        .iter()
        .flat_map(|relationship| {
            relationship
                .sides_of(record_type)
                .into_iter()
                .map(|side| (relationship.clone(), side))
        })
        .collect()
}
