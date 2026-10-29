use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::public_events::catalog::{
    CUSTOM_RECORD_RESOURCE, PreviousValues, PublicEventDefinition, PublicResource,
    custom_event_record_type, find_public_event,
};
use crate::application::shared::{Actor, DomainEvent, RecordType};
use crate::domain::UserId;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq)]
pub struct PublicEvent {
    pub id: Uuid,
    pub event_type: String,
    pub actor: Actor,
    pub occurred_at: DateTime<Utc>,
    pub data: Value,
    pub previous: Option<Value>,
    pub context: Option<Value>,
}

impl PublicEvent {
    pub fn to_json(&self) -> Value {
        let mut envelope = json!({
            "id": self.id.to_string(),
            "type": self.event_type,
            "actor": { "type": self.actor.kind(), "id": self.actor.id() },
            "occurred_at": self.occurred_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            "data": self.data,
        });
        if let (Some(previous), Some(object)) = (&self.previous, envelope.as_object_mut()) {
            object.insert("previous".to_owned(), previous.clone());
        }
        if let (Some(context), Some(object)) = (&self.context, envelope.as_object_mut()) {
            object.insert("context".to_owned(), context.clone());
        }
        envelope
    }
}

#[async_trait]
pub trait PublicEventRenderer: Send + Sync {
    async fn render(&self, event: &DomainEvent) -> Result<Option<PublicEvent>, AppError>;
    async fn restrict(
        &self,
        event: &PublicEvent,
        viewer: Option<&UserId>,
    ) -> Result<PublicEvent, AppError>;
}

#[async_trait]
impl<R> PublicEventRenderer for Arc<R>
where
    R: PublicEventRenderer + ?Sized,
{
    async fn render(&self, event: &DomainEvent) -> Result<Option<PublicEvent>, AppError> {
        (**self).render(event).await
    }

    async fn restrict(
        &self,
        event: &PublicEvent,
        viewer: Option<&UserId>,
    ) -> Result<PublicEvent, AppError> {
        (**self).restrict(event, viewer).await
    }
}

pub type SharedPublicEventRenderer = Arc<dyn PublicEventRenderer>;

pub fn public_definition_for(event: &DomainEvent) -> Option<PublicEventDefinition> {
    let definition = find_public_event(&event.event_type)?;
    if !definition.resource.has_record() {
        return Some(definition);
    }
    let record = event.record.as_ref()?;
    let matches = match definition.resource {
        PublicResource::CUSTOM_RECORD => {
            custom_event_record_type(&event.event_type) == Some(record.record_type.as_str())
        }
        resource => record.record_type == resource.record_type(),
    };
    matches.then_some(definition)
}

pub fn public_record_type(event_type: &str, definition: &PublicEventDefinition) -> RecordType {
    match definition.resource {
        PublicResource::CUSTOM_RECORD => custom_event_record_type(event_type)
            .and_then(|record_type| RecordType::parse(record_type).ok())
            .unwrap_or(CUSTOM_RECORD_RESOURCE),
        resource => resource.record_type(),
    }
}

const INTERNAL_PAYLOAD_KEYS: &[&str] = &["changes", "requested_by"];
const SECRET_KEY_MARKERS: &[&str] = &["secret", "password", "token", "credential", "api_key"];

fn is_public_payload_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    !INTERNAL_PAYLOAD_KEYS.contains(&lowered.as_str())
        && !SECRET_KEY_MARKERS
            .iter()
            .any(|marker| lowered.contains(marker))
}

pub fn payload_resource_data(event: &DomainEvent) -> Value {
    let mut data: Map<String, Value> = event
        .payload
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| is_public_payload_key(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if let Some(record) = event.record.as_ref() {
        data.entry("id".to_owned())
            .or_insert_with(|| json!(record.record_id.to_string()));
    }
    Value::Object(data)
}

pub fn deleted_resource_data(event: &DomainEvent) -> Value {
    json!({ "id": event.record.as_ref().map(|record| record.record_id.to_string()) })
}

pub const PREVIOUS_CHANGES_KEY: &str = "changes";

fn previous_fields(pairs: &[(&str, &str)], payload: &Value) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(field, from)| {
            (
                (*field).to_owned(),
                payload.get(*from).cloned().unwrap_or(Value::Null),
            )
        })
        .collect()
}

pub fn previous_from_changes(payload: &Value) -> Map<String, Value> {
    payload
        .get(PREVIOUS_CHANGES_KEY)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|change| {
            let field = change.get("field").and_then(Value::as_str)?;
            Some((
                field.to_owned(),
                change.get("old").cloned().unwrap_or(Value::Null),
            ))
        })
        .collect()
}

pub fn previous_values(definition: &PublicEventDefinition, payload: &Value) -> Option<Value> {
    let previous = match definition.previous {
        PreviousValues::None => return None,
        PreviousValues::Changes => previous_from_changes(payload),
        PreviousValues::FromTo { field, from } => previous_fields(&[(field, from)], payload),
        PreviousValues::OldNew { field } => previous_fields(&[(field, "old")], payload),
        PreviousValues::Pipeline => {
            previous_fields(&[("pipeline", "from"), ("stage", "from_stage")], payload)
        }
        PreviousValues::Fields(pairs) => previous_fields(pairs, payload),
    };
    (!previous.is_empty()).then_some(Value::Object(previous))
}
