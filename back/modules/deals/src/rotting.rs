use aphel_contract_automation::application::definition::TriggerSpec;
use aphel_contract_automation::application::time::triggers::{DueRule, TimePlan};
use aphel_contract_automation::application::triggers::{AutomationTrigger, TriggerCandidate};
use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::query::{Filter, FilterOperator};
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::errors::FieldViolation;
use aphel_sdk::workflow::rotting::{ROTTING_AT_FIELD, STAGE_ENTERED_COLUMN};
use chrono::Duration;
use serde_json::{Value, json};
pub const TIME_DEAL_ROTTING: &str = "time.deal_rotting";
pub(crate) fn rotting_plan(spec: &TriggerSpec) -> Result<TimePlan, Vec<FieldViolation>> {
    let violations: Vec<FieldViolation> = spec
        .options
        .keys()
        .map(|key| {
            FieldViolation::new(format!("trigger.{key}"), "is not an option of this trigger")
        })
        .collect();
    if !violations.is_empty() {
        return Err(violations);
    }
    Ok(TimePlan::Due(DueRule {
        field: ROTTING_AT_FIELD.to_owned(),
        fallback: None,
        offset: Duration::zero(),
        at: None,
        key_field: Some(STAGE_ENTERED_COLUMN.to_owned()),
    }))
}

pub(crate) struct DealRotting;

impl AutomationTrigger for DealRotting {
    fn key(&self) -> &'static str {
        TIME_DEAL_ROTTING
    }
    fn label(&self) -> &'static str {
        "Negócio parado na etapa"
    }
    fn description(&self) -> &'static str {
        "Runs once per stage entry when an open deal stays in a stage longer than the stage's `rotting_days`; stages without a limit never fire, and a new run only happens after the deal enters a stage again"
    }
    fn supports(&self, record_type: &str) -> bool {
        record_type == RecordType::DEAL.as_str()
    }
    fn listens_to(&self, _event_type: &str) -> bool {
        false
    }
    fn candidates(&self, _event: &DomainEvent) -> Vec<TriggerCandidate> {
        Vec::new()
    }
    fn validate(&self, spec: &TriggerSpec, fields: &FieldSet) -> Vec<FieldViolation> {
        let mut violations = rotting_plan(spec).err().unwrap_or_default();
        for key in [ROTTING_AT_FIELD, STAGE_ENTERED_COLUMN] {
            if fields.find_active(key).is_none() {
                violations.push(FieldViolation::new(
                    "trigger.type",
                    format!("is not available (it needs the field {key})"),
                ));
            }
        }
        violations
    }
    fn record_filters(&self, _spec: &TriggerSpec) -> Vec<(String, Filter)> {
        vec![(
            "trigger".to_owned(),
            Filter::condition(ROTTING_AT_FIELD, FilterOperator::IsNotEmpty, Value::Null),
        )]
    }
    fn options_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }
    fn is_time_based(&self) -> bool {
        true
    }
}
