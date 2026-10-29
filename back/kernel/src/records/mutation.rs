use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::PERMISSION_DENIED;
use crate::application::fields::{CustomFieldPorts, ReferenceChecker};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::domain::fields::FieldValueError;
use crate::errors::AppError;
use crate::records::InputValidator;

pub struct RecordMutation {
    pub record: RecordRef,
    pub event_type: &'static str,
    pub payload: Value,
    pub changes: Vec<FieldChange>,
    pub ip: Option<String>,
}

pub async fn publish_mutation<Tx, E, A>(
    events: &E,
    audit: &A,
    tx: &mut Tx,
    scope: &AccessScope,
    now: DateTime<Utc>,
    mutation: RecordMutation,
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
            DomainEvent::new(actor.clone(), mutation.event_type, now)
                .with_record(mutation.record.clone())
                .with_payload(mutation.payload),
        )
        .await?;
    audit
        .record(
            tx,
            AuditEntry::new(actor, mutation.event_type, now)
                .with_record(mutation.record)
                .with_changes(mutation.changes)
                .with_ip(mutation.ip),
        )
        .await
}

pub fn changes_payload(changes: &[FieldChange]) -> Value {
    json!({
        "changes": changes
            .iter()
            .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
            .collect::<Vec<_>>(),
    })
}

pub fn merge_violations(
    validator: InputValidator,
    custom: Result<Vec<FieldChange>, AppError>,
    message: &str,
) -> Result<Vec<FieldChange>, AppError> {
    let mut violations = validator.into_violations();
    let changes = match custom {
        Ok(changes) => changes,
        Err(AppError::InvalidFields { fields, .. }) => {
            violations.extend(fields);
            Vec::new()
        }
        Err(other) => return Err(other),
    };
    if violations.is_empty() {
        return Ok(changes);
    }
    violations.sort_by(|left, right| left.field.cmp(&right.field));
    Err(AppError::invalid_fields(message, violations))
}

pub fn parse_record_id(
    value: &str,
    not_found: impl FnOnce() -> AppError,
) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| not_found())
}

pub fn permission_denied() -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        "you are not allowed to change this record",
    )
}

pub async fn check_owner<F>(
    fields: &F,
    owner: Option<&UserId>,
    validator: &mut InputValidator,
) -> Result<(), AppError>
where
    F: CustomFieldPorts,
{
    let Some(owner) = owner else {
        return Ok(());
    };
    if fields
        .references()
        .existing_users(std::slice::from_ref(owner))
        .await?
        .is_empty()
    {
        validator.violation("owner_id", FieldValueError::ReferenceNotFound.message());
    }
    Ok(())
}

pub fn archive_change(old: Option<DateTime<Utc>>, new: Option<DateTime<Utc>>) -> FieldChange {
    let instant = |value: Option<DateTime<Utc>>| {
        value.map_or(Value::Null, |at| {
            Value::String(at.to_rfc3339_opts(SecondsFormat::Secs, true))
        })
    };
    FieldChange::new("archived_at", instant(old), instant(new))
}
