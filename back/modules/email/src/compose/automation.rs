use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::compose::{MAX_RECIPIENTS, OwnerEmailRequest, RecordInput};
use crate::domain::normalize_address;
use crate::templates::{
    MAX_TEMPLATE_BODY_BYTES, MAX_TEMPLATE_SUBJECT_LENGTH, is_email_record_type,
};
use crate::{EMAIL_BOUNCED, EMAIL_CLICKED, EMAIL_OPENED, EMAIL_RECEIVED, EMAIL_SENT, EmailAddress};
use aphel_kernel::application::automations::AutomationTrigger;
use aphel_kernel::application::automations::actions::{
    ActionContext, AutomationAction, ValidationContext, record_permission,
};
use aphel_kernel::application::automations::triggers::{TriggerCandidate, event_context};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const SEND_OWNER_EMAIL: &str = "send_owner_email";
const OPTIONS: [&str; 5] = ["to", "template_id", "subject", "body", "tracking"];
const DEFAULT_EMAIL_FIELD: &str = "email";

fn text<'a>(config: &'a Value, key: &str) -> Option<&'a str> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn recipient_specs(config: &Value) -> Vec<Value> {
    match config.get("to").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => items.clone(),
        _ => vec![json!({ "type": "field", "field": DEFAULT_EMAIL_FIELD })],
    }
}

fn push_address(addresses: &mut Vec<EmailAddress>, value: &str) {
    if let Ok(address) = normalize_address(value)
        && !addresses.iter().any(|existing| existing.address == address)
    {
        addresses.push(EmailAddress::new(address));
    }
}

fn recipients(context: &ActionContext<'_>, config: &Value) -> Vec<EmailAddress> {
    let mut addresses = Vec::new();
    for spec in recipient_specs(config) {
        match spec.get("type").and_then(Value::as_str) {
            Some("address") => {
                if let Some(address) = spec.get("address").and_then(Value::as_str) {
                    push_address(&mut addresses, address);
                }
            }
            Some("field") => {
                let field = spec
                    .get("field")
                    .and_then(Value::as_str)
                    .unwrap_or(DEFAULT_EMAIL_FIELD);
                match context.record_value(field) {
                    Some(Value::String(value)) => push_address(&mut addresses, value),
                    Some(Value::Array(values)) => {
                        for value in values.iter().filter_map(Value::as_str) {
                            push_address(&mut addresses, value);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    addresses.truncate(MAX_RECIPIENTS);
    addresses
}

pub(crate) struct SendOwnerEmail;

#[async_trait]
impl AutomationAction for SendOwnerEmail {
    fn key(&self) -> &'static str {
        SEND_OWNER_EMAIL
    }

    fn label(&self) -> &'static str {
        "Enviar e-mail pela caixa do responsável"
    }

    fn description(&self) -> &'static str {
        "Sends an e-mail to the record (its e-mail field or fixed addresses) from the mailbox of the record owner, with a template or a subject and body"
    }

    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "to": {
                    "type": "array",
                    "maxItems": MAX_RECIPIENTS,
                    "items": {
                        "oneOf": [
                            { "type": "object", "properties": { "type": { "const": "field" }, "field": { "type": "string" } }, "required": ["type", "field"] },
                            { "type": "object", "properties": { "type": { "const": "address" }, "address": { "type": "string", "format": "email" } }, "required": ["type", "address"] },
                        ]
                    }
                },
                "template_id": { "type": "string", "format": "uuid" },
                "subject": { "type": "string", "maxLength": MAX_TEMPLATE_SUBJECT_LENGTH },
                "body": { "type": "string" },
                "tracking": { "type": "boolean" },
            },
            "additionalProperties": false,
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        is_email_record_type(record_type)
    }

    fn required_permissions(&self, record_type: &str, _config: &Value) -> Vec<Permission> {
        let mut permissions = vec![Permission::EMAILS_SEND];
        permissions.extend(record_permission(record_type, "read"));
        permissions
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
        match config.get("to") {
            None | Some(Value::Null) => {}
            Some(Value::Array(items)) if items.len() <= MAX_RECIPIENTS => {
                for (index, item) in items.iter().enumerate() {
                    let path = format!("to.{index}");
                    match item.get("type").and_then(Value::as_str) {
                        Some("address") => {
                            let valid = item.get("address").and_then(Value::as_str).is_some_and(
                                |address| {
                                    address.contains("{{") || normalize_address(address).is_ok()
                                },
                            );
                            if !valid {
                                violations.push(
                                    context.violation(&path, "must have a valid e-mail address"),
                                );
                            }
                        }
                        Some("field") => {
                            let known = item
                                .get("field")
                                .and_then(Value::as_str)
                                .is_some_and(|field| context.fields.find_active(field).is_some());
                            if !known {
                                violations.push(
                                    context.violation(&path, "must reference a readable field"),
                                );
                            }
                        }
                        _ => violations.push(context.violation(&path, "must be field or address")),
                    }
                }
            }
            Some(_) => violations.push(context.violation(
                "to",
                format!("must be a list of at most {MAX_RECIPIENTS} recipients"),
            )),
        }
        let template = text(config, "template_id");
        if let Some(id) = template
            && uuid::Uuid::parse_str(id).is_err()
        {
            violations.push(context.violation("template_id", "must be a UUID"));
        }
        let subject = text(config, "subject");
        let body = text(config, "body");
        if template.is_none() && (subject.is_none() || body.is_none()) {
            violations.push(context.violation("", "requires template_id or subject and body"));
        }
        if subject.is_some_and(|subject| subject.chars().count() > MAX_TEMPLATE_SUBJECT_LENGTH) {
            violations.push(context.violation(
                "subject",
                format!("must have at most {MAX_TEMPLATE_SUBJECT_LENGTH} characters"),
            ));
        }
        if body.is_some_and(|body| body.len() > MAX_TEMPLATE_BODY_BYTES) {
            violations.push(context.violation(
                "body",
                format!("must have at most {MAX_TEMPLATE_BODY_BYTES} bytes"),
            ));
        }
        if !matches!(
            config.get("tracking"),
            None | Some(Value::Null | Value::Bool(_))
        ) {
            violations.push(context.violation("tracking", "must be a boolean"));
        }
        Ok(violations)
    }

    async fn describe(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        Ok(json!({
            "owner_id": context.record_value("owner_id"),
            "recipients": recipients(context, config).len(),
        }))
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let Some(owner_id) = context
            .record_value("owner_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return Err(AppError::Validation("the record has no owner".to_owned()));
        };
        let to = recipients(context, config);
        if to.is_empty() {
            return Ok(json!({ "sent": false, "reason": "no_recipients" }));
        }
        let email_id = crate::service::compose::send_as_owner(
            &aphel_kernel::Ctx::from_state(context.gateway.state(), context.scope).await?,
            OwnerEmailRequest {
                owner_id: owner_id.clone(),
                record: RecordInput {
                    record_type: context.record.record_type.as_str().to_owned(),
                    record_id: context.record.record_id.to_string(),
                },
                to: to.clone(),
                subject: text(config, "subject").map(str::to_owned),
                body_html: text(config, "body").map(str::to_owned),
                template_id: text(config, "template_id").map(str::to_owned),
                tracking: config.get("tracking").and_then(Value::as_bool),
            },
        )
        .await?
        .message
        .id;
        Ok(json!({
            "sent": true,
            "email_id": email_id,
            "owner_id": owner_id,
            "recipients": to.len(),
        }))
    }
}

struct EmailEventTrigger {
    key: &'static str,
    label: &'static str,
    description: &'static str,
}

impl AutomationTrigger for EmailEventTrigger {
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
        is_email_record_type(record_type)
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == self.key
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        event
            .record
            .iter()
            .filter(|record| is_email_record_type(record.record_type.as_str()))
            .map(|record| TriggerCandidate {
                record: record.clone(),
                context: event_context(self.key, event),
            })
            .collect()
    }
}

pub(crate) fn email_triggers() -> Vec<Arc<dyn AutomationTrigger>> {
    vec![
        Arc::new(EmailEventTrigger {
            key: EMAIL_RECEIVED,
            label: "E-mail recebido",
            description: "An e-mail linked to the record was received in a connected mailbox",
        }),
        Arc::new(EmailEventTrigger {
            key: EMAIL_SENT,
            label: "E-mail enviado",
            description: "An e-mail linked to the record was sent from a connected mailbox",
        }),
        Arc::new(EmailEventTrigger {
            key: EMAIL_OPENED,
            label: "E-mail aberto",
            description: "A tracked e-mail linked to the record was opened for the first time",
        }),
        Arc::new(EmailEventTrigger {
            key: EMAIL_CLICKED,
            label: "Link de e-mail clicado",
            description: "A link of a tracked e-mail linked to the record was clicked for the first time",
        }),
        Arc::new(EmailEventTrigger {
            key: EMAIL_BOUNCED,
            label: "E-mail devolvido",
            description: "An e-mail linked to the record bounced",
        }),
    ]
}
