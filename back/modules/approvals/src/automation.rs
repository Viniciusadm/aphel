use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    APPROVAL_APPROVED, APPROVAL_CANCELLED, APPROVAL_REJECTED, APPROVAL_REQUESTED, ApprovalMoment,
    SubmitApprovalCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::application::automations::AutomationTrigger;
use aphel_kernel::application::automations::actions::{
    ActionContext, AutomationAction, ValidationContext,
};
use aphel_kernel::application::automations::definition::TriggerSpec;
use aphel_kernel::application::automations::triggers::{
    TriggerCandidate, event_context, is_automatable,
};
use aphel_kernel::application::config_packages::is_config_key;
use aphel_kernel::application::shared::{DomainEvent, RecordRef};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const REQUEST_APPROVAL: &str = "request_approval";
const ACTION_OPTIONS: [&str; 2] = ["policy_key", "comment"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalAutomationRequest {
    pub(crate) record: RecordRef,
    pub(crate) policy_key: String,
    pub(crate) comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalAutomationOutcome {
    pub(crate) outcome: String,
    pub(crate) request_ids: Vec<Uuid>,
}

struct ApprovalTrigger {
    key: &'static str,
    label: &'static str,
    description: &'static str,
}

impl AutomationTrigger for ApprovalTrigger {
    fn key(&self) -> &'static str {
        self.key
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "policy_key": { "type": "string" },
            },
            "additionalProperties": false,
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == self.key
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        event
            .record
            .iter()
            .filter(|record| is_automatable(record.record_type.as_str()))
            .map(|record| TriggerCandidate {
                record: record.clone(),
                context: event_context(self.key, event),
            })
            .collect()
    }

    fn accepts(&self, spec: &TriggerSpec, candidate: &TriggerCandidate) -> bool {
        spec.text("policy_key").is_none_or(|expected| {
            candidate
                .context
                .payload
                .get("policy_key")
                .and_then(Value::as_str)
                == Some(expected)
        })
    }
}

pub(crate) fn approval_triggers() -> Vec<Arc<dyn AutomationTrigger>> {
    vec![
        Arc::new(ApprovalTrigger {
            key: APPROVAL_REQUESTED,
            label: "Aprovação solicitada",
            description: "An approval was requested for the record; `policy_key` narrows the policy",
        }),
        Arc::new(ApprovalTrigger {
            key: APPROVAL_APPROVED,
            label: "Aprovação concedida",
            description: "Every step of an approval request of the record was approved; `policy_key` narrows the policy",
        }),
        Arc::new(ApprovalTrigger {
            key: APPROVAL_REJECTED,
            label: "Aprovação rejeitada",
            description: "An approval request of the record was rejected; `policy_key` narrows the policy",
        }),
        Arc::new(ApprovalTrigger {
            key: APPROVAL_CANCELLED,
            label: "Aprovação cancelada",
            description: "An approval request of the record was cancelled; `policy_key` narrows the policy",
        }),
    ]
}

pub(crate) struct RequestApproval;

fn policy_key(config: &Value) -> Option<&str> {
    config
        .get("policy_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
}

#[async_trait]
impl AutomationAction for RequestApproval {
    fn key(&self) -> &'static str {
        REQUEST_APPROVAL
    }

    fn label(&self) -> &'static str {
        "Solicitar aprovação"
    }

    fn description(&self) -> &'static str {
        "Requests the approval of an on-demand approval policy (by `policy_key`) for the record; nothing happens when the policy condition does not match or a request is already open"
    }

    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "policy_key": { "type": "string" },
                "comment": { "type": "string" },
            },
            "required": ["policy_key"],
            "additionalProperties": false,
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }

    fn required_permissions(&self, record_type: &str, _config: &Value) -> Vec<Permission> {
        aphel_kernel::application::automations::actions::record_permission(record_type, "update")
            .into_iter()
            .collect()
    }

    fn retry_safe(&self, _config: &Value) -> bool {
        true
    }

    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let Some(map) = config.as_object() else {
            return Ok(vec![context.violation("", "must be an object")]);
        };
        let mut violations = Vec::new();
        for key in map.keys() {
            if !ACTION_OPTIONS.contains(&key.as_str()) {
                violations.push(context.violation(key, "is not a known option"));
            }
        }
        if !policy_key(config).is_some_and(is_config_key) {
            violations.push(context.violation(
                "policy_key",
                "must be the key of an on-demand approval policy",
            ));
        }
        match config.get("comment") {
            None | Some(Value::Null) | Some(Value::String(_)) => {}
            Some(_) => violations.push(context.violation("comment", "must be a text")),
        }
        Ok(violations)
    }

    async fn describe(
        &self,
        _context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        Ok(json!({ "policy_key": policy_key(config) }))
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let key = policy_key(config).unwrap_or_default().to_owned();
        let outcome = crate::automation::request_for_automation(
            &aphel_kernel::Ctx::from_state(context.gateway.state(), context.scope).await?,
            ApprovalAutomationRequest {
                record: context.record.clone(),
                policy_key: key.clone(),
                comment: config
                    .get("comment")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
        )
        .await?;
        Ok(json!({
            "policy_key": key,
            "outcome": outcome.outcome,
            "request_ids": outcome
                .request_ids
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>(),
        }))
    }
}

pub(crate) async fn request_for_automation(
    ctx: &Ctx<'_>,
    request: ApprovalAutomationRequest,
) -> Result<ApprovalAutomationOutcome, AppError> {
    let submission = service::submit(
        ctx,
        SubmitApprovalCommand {
            record_type: request.record.record_type.as_str().to_owned(),
            record_id: request.record.record_id.to_string(),
            moment: ApprovalMoment::OnDemand.as_str().to_owned(),
            target_id: None,
            comment: request.comment,
            ip: None,
        },
        Some(&request.policy_key),
    )
    .await?;
    Ok(ApprovalAutomationOutcome {
        outcome: submission.outcome.to_owned(),
        request_ids: submission
            .requests
            .iter()
            .map(|view| view.record.request.id)
            .collect(),
    })
}
