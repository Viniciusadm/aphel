use aphel_contract_crm::CrmPermissions;
use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    NO_MAILBOX_AVAILABLE, SEQUENCE_ALREADY_ENROLLED, SEQUENCE_CONTACT_OBJECTED,
    SEQUENCE_CONTACT_WITHOUT_EMAIL, SEQUENCE_EMPTY, SEQUENCE_INACTIVE,
};
use aphel_kernel::application::automations::actions::{
    ActionContext, AutomationAction, ValidationContext, record_permission,
};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const ENROLL_IN_SEQUENCE: &str = "enroll_in_sequence";
const OPTIONS: [&str; 2] = ["sequence_id", "sender"];
pub(crate) const SENDER_RECORD_OWNER: &str = "record_owner";
pub(crate) const SENDER_AUTOMATION_AUTHOR: &str = "automation_author";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceEnrollmentRequest {
    pub(crate) sequence_id: String,
    pub(crate) contact_id: Uuid,
    pub(crate) enroll_as: Option<String>,
}

const SKIPPABLE: [&str; 6] = [
    SEQUENCE_ALREADY_ENROLLED,
    SEQUENCE_CONTACT_WITHOUT_EMAIL,
    SEQUENCE_CONTACT_OBJECTED,
    NO_MAILBOX_AVAILABLE,
    SEQUENCE_INACTIVE,
    SEQUENCE_EMPTY,
];

pub(crate) struct EnrollInSequence;

#[async_trait]
impl AutomationAction for EnrollInSequence {
    fn key(&self) -> &'static str {
        ENROLL_IN_SEQUENCE
    }

    fn label(&self) -> &'static str {
        "Inscrever em sequência"
    }

    fn description(&self) -> &'static str {
        "Enrolls the contact in a sequence; e-mails go out from the mailbox of the record owner (default) or of the automation author"
    }

    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "sequence_id": { "type": "string", "format": "uuid" },
                "sender": { "type": "string", "enum": [SENDER_RECORD_OWNER, SENDER_AUTOMATION_AUTHOR] },
            },
            "required": ["sequence_id"],
            "additionalProperties": false,
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == "contact"
    }

    fn required_permissions(&self, record_type: &str, _config: &Value) -> Vec<Permission> {
        let mut permissions = vec![Permission::SEQUENCES_ENROLL];
        permissions.extend(record_permission(record_type, "read"));
        permissions
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
        let Some(map) = config.as_object() else {
            return Ok(vec![context.violation("", "must be an object")]);
        };
        for key in map.keys() {
            if !OPTIONS.contains(&key.as_str()) {
                violations.push(context.violation(key, "is not a known option"));
            }
        }
        let valid_id = config
            .get("sequence_id")
            .and_then(Value::as_str)
            .is_some_and(|id| Uuid::parse_str(id.trim()).is_ok());
        if !valid_id {
            violations.push(context.violation("sequence_id", "must be a sequence id"));
        }
        match config.get("sender") {
            None | Some(Value::Null) => {}
            Some(Value::String(sender))
                if sender == SENDER_RECORD_OWNER || sender == SENDER_AUTOMATION_AUTHOR => {}
            Some(_) => violations
                .push(context.violation("sender", "must be record_owner or automation_author")),
        }
        Ok(violations)
    }

    async fn describe(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        Ok(json!({
            "sequence_id": config.get("sequence_id"),
            "owner_id": context.record_value("owner_id"),
        }))
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let sequence_id = config
            .get("sequence_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_owned();
        let enroll_as = match config.get("sender").and_then(Value::as_str) {
            Some(SENDER_AUTOMATION_AUTHOR) => None,
            _ => context
                .record_value("owner_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
        };
        match crate::service::enroll_for_automation(
            &aphel_kernel::Ctx::from_state(context.gateway.state(), context.scope).await?,
            SequenceEnrollmentRequest {
                sequence_id: sequence_id.clone(),
                contact_id: context.record.record_id,
                enroll_as,
            },
        )
        .await
        {
            Ok(enrollment_id) => Ok(json!({
                "enrolled": true,
                "sequence_id": sequence_id,
                "enrollment_id": enrollment_id,
            })),
            Err(AppError::Coded { code, .. }) if SKIPPABLE.contains(&code) => Ok(json!({
                "enrolled": false,
                "sequence_id": sequence_id,
                "reason": code,
            })),
            Err(error) => Err(error),
        }
    }
}
