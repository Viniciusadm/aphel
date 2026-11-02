use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TriggerContext {
    pub trigger_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default)]
    pub changed_fields: Vec<String>,
    #[serde(default)]
    pub previous: Map<String, Value>,
    #[serde(default)]
    pub payload: Value,
}

impl TriggerContext {
    pub fn new(trigger_type: impl Into<String>) -> Self {
        Self {
            trigger_type: trigger_type.into(),
            ..Self::default()
        }
    }

    pub fn json(&self) -> Value {
        json!({
            "type": self.trigger_type,
            "event_id": self.event_id.map(|id| id.to_string()),
            "event_type": self.event_type,
            "occurred_at": self
                .occurred_at
                .map(|at| at.to_rfc3339_opts(SecondsFormat::Secs, true)),
            "actor": { "type": self.actor_type, "id": self.actor_id },
            "changed_fields": self.changed_fields,
            "previous": self.previous,
            "payload": self.payload,
        })
    }
}

pub fn lookup<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = match current {
            Value::Object(map) => map.get(*segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}
