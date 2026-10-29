use std::sync::Arc;

use serde_json::{Value, json};

use crate::domain::{COMPANY_SCORE_BAND_CHANGED, SCORE_BAND_CHANGED, ScoreBand};
use crate::engine::is_scored_record_type;
use aphel_kernel::application::automations::AutomationTrigger;
use aphel_kernel::application::automations::definition::TriggerSpec;
use aphel_kernel::application::automations::triggers::{TriggerCandidate, event_context};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::errors::FieldViolation;

const DIRECTIONS: [&str; 2] = ["up", "down"];

struct ScoreBandChanged;

fn payload_text<'a>(candidate: &'a TriggerCandidate, key: &str) -> Option<&'a str> {
    candidate.context.payload.get(key).and_then(Value::as_str)
}

impl AutomationTrigger for ScoreBandChanged {
    fn key(&self) -> &'static str {
        SCORE_BAND_CHANGED
    }

    fn label(&self) -> &'static str {
        "Faixa de pontuação alterada"
    }

    fn description(&self) -> &'static str {
        "The score of the record crossed into another band of a scoring model; `model_key`, `band` (cold, warm, hot) and `direction` (up, down) narrow it"
    }

    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "model_key": { "type": "string" },
                "band": { "type": "string", "enum": ["cold", "warm", "hot"] },
                "direction": { "type": "string", "enum": DIRECTIONS },
            },
            "additionalProperties": false,
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        is_scored_record_type(record_type)
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == SCORE_BAND_CHANGED || event_type == COMPANY_SCORE_BAND_CHANGED
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        event
            .record
            .iter()
            .filter(|record| is_scored_record_type(record.record_type.as_str()))
            .map(|record| TriggerCandidate {
                record: record.clone(),
                context: event_context(SCORE_BAND_CHANGED, event),
            })
            .collect()
    }

    fn accepts(&self, spec: &TriggerSpec, candidate: &TriggerCandidate) -> bool {
        ["model_key", "band", "direction"]
            .into_iter()
            .all(|option| {
                spec.text(option)
                    .is_none_or(|expected| payload_text(candidate, option) == Some(expected))
            })
    }

    fn validate(&self, spec: &TriggerSpec, _fields: &FieldSet) -> Vec<FieldViolation> {
        let mut violations = Vec::new();
        if spec
            .text("band")
            .is_some_and(|band| ScoreBand::parse(band).is_none())
        {
            violations.push(FieldViolation::new(
                "trigger.band",
                "must be cold, warm or hot",
            ));
        }
        if spec
            .text("direction")
            .is_some_and(|direction| !DIRECTIONS.contains(&direction))
        {
            violations.push(FieldViolation::new(
                "trigger.direction",
                "must be up or down",
            ));
        }
        violations
    }
}

pub(crate) fn scoring_triggers() -> Vec<Arc<dyn AutomationTrigger>> {
    vec![Arc::new(ScoreBandChanged)]
}
