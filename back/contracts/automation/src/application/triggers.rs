use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::application::context::TriggerContext;
use crate::application::definition::TriggerSpec;
use aphel_kernel::application::fields::{EntityTraits, FieldSet};
use aphel_kernel::application::query::Filter;
use aphel_kernel::application::shared::{DomainEvent, RecordRef};
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::errors::FieldViolation;

pub const RECORD_CREATED: &str = "record.created";
pub const RECORD_UPDATED: &str = "record.updated";
pub const RECORD_DELETED: &str = "record.deleted";
pub const MANUAL: &str = "manual";
pub const INCOMING_WEBHOOK: &str = "incoming_webhook";

pub fn is_automatable(record_type: &str) -> bool {
    is_custom_record_type(record_type)
        || has_capability(record_type, |caps| caps.has_trait("automations.subject"))
}

pub fn is_attached_to_parent(record_type: &str) -> bool {
    has_capability(record_type, |caps| {
        caps.has_trait("records.attached_to_parent")
    })
}

fn has_capability(record_type: &str, check: impl Fn(&EntityTraits) -> bool) -> bool {
    aphel_kernel::modules::try_app().is_some_and(|app| {
        app.record_types()
            .capabilities(record_type)
            .is_some_and(check)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerCandidate {
    pub record: RecordRef,
    pub context: TriggerContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerReference {
    Stage { path: String, id: String },
    IncomingWebhook { path: String, id: String },
}

pub trait AutomationTrigger: Send + Sync {
    fn key(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn options_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }
    fn supports(&self, record_type: &str) -> bool;
    fn listens_to(&self, event_type: &str) -> bool;
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate>;
    fn accepts(&self, _spec: &TriggerSpec, _candidate: &TriggerCandidate) -> bool {
        true
    }
    fn validate(&self, _spec: &TriggerSpec, _fields: &FieldSet) -> Vec<FieldViolation> {
        Vec::new()
    }
    fn references(&self, _spec: &TriggerSpec) -> Vec<TriggerReference> {
        Vec::new()
    }
    fn is_manual(&self) -> bool {
        false
    }
    fn record_filters(&self, _spec: &TriggerSpec) -> Vec<(String, Filter)> {
        Vec::new()
    }
    fn is_time_based(&self) -> bool {
        false
    }
}

pub fn event_context(trigger_type: &str, event: &DomainEvent) -> TriggerContext {
    TriggerContext {
        trigger_type: trigger_type.to_owned(),
        event_id: Some(event.id),
        event_type: Some(event.event_type.clone()),
        occurred_at: Some(event.occurred_at),
        actor_type: Some(event.actor.kind().to_owned()),
        actor_id: event.actor.id().map(str::to_owned),
        changed_fields: Vec::new(),
        previous: Map::new(),
        payload: event.payload.clone(),
    }
}

pub fn record_event<'a>(event: &'a DomainEvent, action: &str) -> Option<&'a RecordRef> {
    let record = event.record.as_ref()?;
    let (prefix, suffix) = event.event_type.rsplit_once('.')?;
    (suffix == action
        && aphel_kernel::application::shared::RecordType::parse(prefix)
            .is_ok_and(|key| key == record.record_type)
        && is_automatable(prefix))
    .then_some(record)
}

pub fn changes(payload: &Value) -> (Vec<String>, Map<String, Value>) {
    let mut fields = Vec::new();
    let mut previous = Map::new();
    for change in payload
        .get("changes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(field) = change.get("field").and_then(Value::as_str) else {
            continue;
        };
        fields.push(field.to_owned());
        previous.insert(
            field.to_owned(),
            change.get("old").cloned().unwrap_or(Value::Null),
        );
    }
    (fields, previous)
}

pub fn stage_change(payload: &Value) -> (Vec<String>, Map<String, Value>) {
    let mut previous = Map::new();
    if let Some(from) = payload.get("from") {
        previous.insert(
            "stage_id".to_owned(),
            from.get("id").cloned().unwrap_or(Value::Null),
        );
        previous.insert("stage".to_owned(), from.clone());
    }
    (vec!["stage_id".to_owned()], previous)
}

pub fn pipeline_change(payload: &Value) -> (Vec<String>, Map<String, Value>) {
    let mut previous = Map::new();
    if let Some(pipeline) = payload.get("from") {
        previous.insert(
            "pipeline_id".to_owned(),
            pipeline.get("id").cloned().unwrap_or(Value::Null),
        );
    }
    if let Some(stage) = payload.get("from_stage") {
        previous.insert(
            "stage_id".to_owned(),
            stage.get("id").cloned().unwrap_or(Value::Null),
        );
    }
    (
        vec!["pipeline_id".to_owned(), "stage_id".to_owned()],
        previous,
    )
}

pub fn single(trigger: &str, event: &DomainEvent, record: &RecordRef) -> Vec<TriggerCandidate> {
    vec![TriggerCandidate {
        record: record.clone(),
        context: event_context(trigger, event),
    }]
}

pub fn parent_records(payload: &Value, include_links: bool) -> Vec<RecordRef> {
    let mut records = Vec::new();
    let mut push = |record_type: Option<&str>, record_id: Option<&str>| {
        if let (Some(record_type), Some(record_id)) = (record_type, record_id)
            && is_automatable(record_type)
            && !is_attached_to_parent(record_type)
            && let Ok(record) = RecordRef::parse(record_type, record_id)
            && !records.contains(&record)
        {
            records.push(record);
        }
    };
    push(
        payload.get("record_type").and_then(Value::as_str),
        payload.get("record_id").and_then(Value::as_str),
    );
    if include_links {
        for link in payload
            .get("links")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            push(
                link.get("record_type").and_then(Value::as_str),
                link.get("record_id").and_then(Value::as_str),
            );
        }
    }
    records
}

pub fn string_list_schema(description: &str) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": description })
}

pub trait AutomationTriggerSource: Send + Sync {
    fn triggers(&self) -> Vec<Arc<dyn AutomationTrigger>>;
}

#[derive(Clone, Default)]
pub struct TriggerRegistry {
    triggers: Vec<Arc<dyn AutomationTrigger>>,
}

impl TriggerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builtin() -> Self {
        let app = aphel_kernel::modules::app();
        Self::new()
            .with_all(app.contributions::<dyn AutomationTrigger>())
            .with_all(
                app.contributions::<dyn AutomationTriggerSource>()
                    .iter()
                    .flat_map(|source| source.triggers())
                    .collect(),
            )
    }

    pub fn with_all(self, triggers: Vec<Arc<dyn AutomationTrigger>>) -> Self {
        triggers
            .into_iter()
            .fold(self, |registry, trigger| registry.with(trigger))
    }

    pub fn with(mut self, trigger: Arc<dyn AutomationTrigger>) -> Self {
        self.triggers
            .retain(|existing| existing.key() != trigger.key());
        self.triggers.push(trigger);
        self
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn AutomationTrigger>> {
        self.triggers.iter().find(|trigger| trigger.key() == key)
    }

    pub fn all(&self) -> &[Arc<dyn AutomationTrigger>] {
        &self.triggers
    }

    pub fn listens_to(&self, event_type: &str) -> bool {
        self.triggers
            .iter()
            .any(|trigger| trigger.listens_to(event_type))
    }

    pub fn candidates(
        &self,
        event: &DomainEvent,
    ) -> Vec<(Arc<dyn AutomationTrigger>, TriggerCandidate)> {
        self.triggers
            .iter()
            .filter(|trigger| trigger.listens_to(&event.event_type))
            .flat_map(|trigger| {
                trigger
                    .candidates(event)
                    .into_iter()
                    .map(|candidate| (trigger.clone(), candidate))
            })
            .collect()
    }
}
