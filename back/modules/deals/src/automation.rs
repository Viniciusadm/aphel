use aphel_contract_crm::refs;
use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::domain::{DEAL_LOST, DEAL_STAGE_CHANGED, DEAL_WON};
use aphel_contract_automation::application::actions::{
    ActionContext, AutomationAction, MOVE_STAGE, ValidationContext, invalid, only_keys, text,
};
use aphel_contract_automation::application::definition::TriggerSpec;
use aphel_contract_automation::application::gateway::Reference;
use aphel_contract_automation::application::triggers::{
    AutomationTrigger, TriggerCandidate, TriggerReference, event_context, single, stage_change,
};
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) struct StageChanged;

impl AutomationTrigger for StageChanged {
    fn key(&self) -> &'static str {
        DEAL_STAGE_CHANGED
    }
    fn label(&self) -> &'static str {
        "Oportunidade mudou de etapa"
    }
    fn description(&self) -> &'static str {
        "A deal moved to another stage; `from_stage_id` and `to_stage_id` narrow the change"
    }
    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "from_stage_id": { "type": "string", "format": "uuid" },
                "to_stage_id": { "type": "string", "format": "uuid" },
            },
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        record_type == RecordType::DEAL.as_str()
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type == DEAL_STAGE_CHANGED
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        let Some(record) = event.record.as_ref() else {
            return Vec::new();
        };
        let (changed_fields, previous) = stage_change(&event.payload);
        let mut context = event_context(DEAL_STAGE_CHANGED, event);
        context.changed_fields = changed_fields;
        context.previous = previous;
        vec![TriggerCandidate {
            record: record.clone(),
            context,
        }]
    }
    fn accepts(&self, spec: &TriggerSpec, candidate: &TriggerCandidate) -> bool {
        let payload = &candidate.context.payload;
        let stage = |side: &str| {
            payload
                .get(side)
                .and_then(|stage| stage.get("id"))
                .and_then(Value::as_str)
        };
        spec.text("from_stage_id")
            .is_none_or(|expected| stage("from") == Some(expected))
            && spec
                .text("to_stage_id")
                .is_none_or(|expected| stage("to") == Some(expected))
    }
    fn references(&self, spec: &TriggerSpec) -> Vec<TriggerReference> {
        ["from_stage_id", "to_stage_id"]
            .into_iter()
            .filter_map(|key| {
                spec.text(key).map(|id| TriggerReference::Stage {
                    path: format!("trigger.{key}"),
                    id: id.to_owned(),
                })
            })
            .collect()
    }
}

pub(crate) struct DealClosed {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) description: &'static str,
}

impl AutomationTrigger for DealClosed {
    fn key(&self) -> &'static str {
        self.key
    }
    fn label(&self) -> &'static str {
        self.label
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn supports(&self, record_type: &str) -> bool {
        record_type == RecordType::DEAL.as_str()
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type == self.key
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        match event.record.as_ref() {
            Some(record) if record.record_type == RecordType::DEAL => {
                single(self.key, event, record)
            }
            _ => Vec::new(),
        }
    }
}

pub(crate) fn deal_closed_triggers() -> [Arc<dyn AutomationTrigger>; 2] {
    [
        Arc::new(DealClosed {
            key: DEAL_WON,
            label: "Oportunidade ganha",
            description: "A deal was won",
        }),
        Arc::new(DealClosed {
            key: DEAL_LOST,
            label: "Oportunidade perdida",
            description: "A deal was lost",
        }),
    ]
}

pub(crate) struct MoveStage;

#[async_trait]
impl AutomationAction for MoveStage {
    fn key(&self) -> &'static str {
        MOVE_STAGE
    }
    fn label(&self) -> &'static str {
        "Mover etapa"
    }
    fn description(&self) -> &'static str {
        "Moves the deal to a stage (stage requirements and lost reasons still apply)"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "stage_id": { "type": "string", "format": "uuid" } },
            "required": ["stage_id"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        record_type == RecordType::DEAL.as_str()
    }
    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        vec![Permission::DEALS_UPDATE]
    }
    fn retry_safe(&self, _config: &Value) -> bool {
        true
    }
    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        if !only_keys(context, config, &["stage_id"], &mut violations) {
            return Ok(violations);
        }
        match text(config, "stage_id") {
            Some(id) => {
                context
                    .reference(&mut violations, "stage_id", Reference::new(refs::STAGE, id))
                    .await?
            }
            None => violations.push(context.violation("stage_id", "is required")),
        }
        Ok(violations)
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let stage_id = text(config, "stage_id").ok_or_else(|| invalid("stage_id is required"))?;
        context
            .gateway
            .move_stage(context.scope, context.record, stage_id)
            .await?;
        Ok(json!({ "stage_id": stage_id }))
    }
}
