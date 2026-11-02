use serde_json::{Value, json};

use aphel_contract_automation::application::definition::TriggerSpec;
use aphel_contract_automation::application::triggers::{
    AutomationTrigger, MANUAL, RECORD_CREATED, RECORD_DELETED, RECORD_UPDATED, TriggerCandidate,
    changes, event_context, is_automatable, pipeline_change, record_event, single, stage_change,
    string_list_schema,
};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::errors::FieldViolation;

const DEAL_STAGE_CHANGED: &str = "deal.stage_changed";

pub(crate) struct RecordCreated;

impl AutomationTrigger for RecordCreated {
    fn key(&self) -> &'static str {
        RECORD_CREATED
    }
    fn label(&self) -> &'static str {
        "Registro criado"
    }
    fn description(&self) -> &'static str {
        "A record of the automation type was created"
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type.ends_with(".created")
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        match record_event(event, "created") {
            Some(record) => single(RECORD_CREATED, event, record),
            None => Vec::new(),
        }
    }
}

const DEAL_PIPELINE_CHANGED: &str = "deal.pipeline_changed";
const DEAL_FIELD_EVENTS: [&str; 3] = [
    "deal.amount_changed",
    "deal.owner_changed",
    "deal.expected_close_date_changed",
];

pub(crate) struct RecordUpdated;

impl AutomationTrigger for RecordUpdated {
    fn key(&self) -> &'static str {
        RECORD_UPDATED
    }
    fn label(&self) -> &'static str {
        "Registro alterado"
    }
    fn description(&self) -> &'static str {
        "Fields of a record changed; `fields` limits the trigger to changes in any of the listed fields"
    }
    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "fields": string_list_schema("Field keys; empty means any change") },
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type.ends_with(".updated")
            || event_type == DEAL_STAGE_CHANGED
            || event_type == DEAL_PIPELINE_CHANGED
            || DEAL_FIELD_EVENTS.contains(&event_type)
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        let event_type = event.event_type.as_str();
        let record = if event_type == DEAL_STAGE_CHANGED
            || event_type == DEAL_PIPELINE_CHANGED
            || DEAL_FIELD_EVENTS.contains(&event_type)
        {
            event.record.as_ref()
        } else {
            record_event(event, "updated")
        };
        let Some(record) = record else {
            return Vec::new();
        };
        let (changed_fields, previous) = match event.event_type.as_str() {
            DEAL_STAGE_CHANGED => stage_change(&event.payload),
            DEAL_PIPELINE_CHANGED => pipeline_change(&event.payload),
            _ => changes(&event.payload),
        };
        let mut context = event_context(RECORD_UPDATED, event);
        context.changed_fields = changed_fields;
        context.previous = previous;
        vec![TriggerCandidate {
            record: record.clone(),
            context,
        }]
    }
    fn accepts(&self, spec: &TriggerSpec, candidate: &TriggerCandidate) -> bool {
        let fields = spec.texts("fields");
        fields.is_empty()
            || fields
                .iter()
                .any(|field| candidate.context.changed_fields.contains(field))
    }
    fn validate(&self, spec: &TriggerSpec, fields: &FieldSet) -> Vec<FieldViolation> {
        let mut violations = Vec::new();
        if let Some(value) = spec.options.get("fields")
            && !value.is_array()
        {
            violations.push(FieldViolation::new(
                "trigger.fields",
                "must be a list of field keys",
            ));
        }
        for (index, key) in spec.texts("fields").iter().enumerate() {
            if fields.find(key).is_none() {
                violations.push(FieldViolation::new(
                    format!("trigger.fields.{index}"),
                    "is not a field of this record type",
                ));
            }
        }
        violations
    }
}

pub(crate) struct RecordDeleted;

impl AutomationTrigger for RecordDeleted {
    fn key(&self) -> &'static str {
        RECORD_DELETED
    }
    fn label(&self) -> &'static str {
        "Registro excluído"
    }
    fn description(&self) -> &'static str {
        "A record was deleted; the record itself is no longer available, only its id and the event payload"
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type.ends_with(".deleted")
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        match record_event(event, "deleted") {
            Some(record) => single(RECORD_DELETED, event, record),
            None => Vec::new(),
        }
    }
}

pub(crate) struct Manual;

impl AutomationTrigger for Manual {
    fn key(&self) -> &'static str {
        MANUAL
    }
    fn label(&self) -> &'static str {
        "Execução manual"
    }
    fn description(&self) -> &'static str {
        "Runs only when someone asks to run the automation on a record (`POST /automations/{id}/run`)"
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, _event_type: &str) -> bool {
        false
    }
    fn candidates(&self, _event: &DomainEvent) -> Vec<TriggerCandidate> {
        Vec::new()
    }
    fn is_manual(&self) -> bool {
        true
    }
}
