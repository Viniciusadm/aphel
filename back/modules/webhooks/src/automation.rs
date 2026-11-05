use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::incoming::INCOMING_WEBHOOK_PROCESSED;
use aphel_contract_automation::application::actions::*;
use aphel_contract_automation::application::definition::TriggerSpec;
use aphel_contract_automation::application::gateway::WebhookCall;
use aphel_contract_automation::application::triggers::{
    AutomationTrigger, INCOMING_WEBHOOK, TriggerCandidate, TriggerReference, is_automatable, single,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::outbound_http::{HttpMethod, is_reserved_transport_header};
use aphel_sdk::platform::application::shared::DomainEvent;
use aphel_sdk::platform::domain::access::Permission;

pub(crate) struct IncomingWebhook;

impl AutomationTrigger for IncomingWebhook {
    fn key(&self) -> &'static str {
        INCOMING_WEBHOOK
    }
    fn label(&self) -> &'static str {
        "Webhook de entrada"
    }
    fn description(&self) -> &'static str {
        "An incoming webhook created or updated a record of the automation type; `incoming_webhook_id` is required and `trigger.payload.body` carries the received body (up to 16 KB)"
    }
    fn options_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "incoming_webhook_id": { "type": "string", "format": "uuid" } },
            "required": ["incoming_webhook_id"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type == INCOMING_WEBHOOK_PROCESSED
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        match event.record.as_ref() {
            Some(record) => single(INCOMING_WEBHOOK, event, record),
            None => Vec::new(),
        }
    }
    fn accepts(&self, spec: &TriggerSpec, candidate: &TriggerCandidate) -> bool {
        spec.text("incoming_webhook_id").is_some_and(|expected| {
            candidate
                .context
                .payload
                .get("incoming_webhook_id")
                .and_then(Value::as_str)
                == Some(expected)
        })
    }
    fn validate(&self, spec: &TriggerSpec, _fields: &FieldSet) -> Vec<FieldViolation> {
        if spec.text("incoming_webhook_id").is_none() {
            return vec![FieldViolation::new(
                "trigger.incoming_webhook_id",
                "is required",
            )];
        }
        Vec::new()
    }
    fn references(&self, spec: &TriggerSpec) -> Vec<TriggerReference> {
        spec.text("incoming_webhook_id")
            .map(|id| TriggerReference::IncomingWebhook {
                path: "trigger.incoming_webhook_id".to_owned(),
                id: id.to_owned(),
            })
            .into_iter()
            .collect()
    }
}

pub(crate) struct CallWebhook;

const MAX_EXTRA_HEADERS: usize = 20;

fn valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

impl CallWebhook {
    fn body(&self, context: &ActionContext<'_>, config: &Value) -> Result<String, AppError> {
        let body = match config.get("body") {
            None | Some(Value::Null) => json!({
                "automation_id": context.automation_id.to_string(),
                "run_id": context.run_id.to_string(),
                "record_type": context.record.record_type.as_str(),
                "record_id": context.record.record_id.to_string(),
                "record": context.record_data,
            }),
            Some(value) => value.clone(),
        };
        let body = match body {
            Value::String(text) => text,
            other => other.to_string(),
        };
        if body.len() > MAX_WEBHOOK_BODY_BYTES {
            return Err(invalid(format!(
                "the webhook body can have at most {MAX_WEBHOOK_BODY_BYTES} bytes"
            )));
        }
        Ok(body)
    }
}

#[async_trait]
impl AutomationAction for CallWebhook {
    fn key(&self) -> &'static str {
        CALL_WEBHOOK
    }
    fn label(&self) -> &'static str {
        "Chamar webhook"
    }
    fn description(&self) -> &'static str {
        "Calls an external URL (same SSRF protection as outgoing webhooks) with `method` (POST by default; GET and DELETE send no body), extra `headers` (the `X-Aphel-*`, `Content-Type` and `User-Agent` headers are reserved) and `content_type` (application/json by default); `signing_secret` is write-only, kept in the secret vault and used to sign `X-Aphel-Signature`; set `retry_safe` when the receiver is idempotent"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "format": "uri" },
                "method": { "type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"], "default": "POST" },
                "headers": { "type": "object", "additionalProperties": { "type": "string" }, "maxProperties": MAX_EXTRA_HEADERS },
                "content_type": { "type": "string", "default": "application/json" },
                "body": { "description": "JSON object or text with templates; defaults to the record and run ids plus the record" },
                "signing_secret": { "type": "string", "writeOnly": true, "minLength": 16 },
                "signing_secret_configured": { "type": "boolean", "readOnly": true },
                "retry_safe": { "type": "boolean", "default": false },
            },
            "required": ["url"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        vec![Permission::SETTINGS_MANAGE]
    }
    fn retry_safe(&self, config: &Value) -> bool {
        config
            .get("retry_safe")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
    fn secret_fields(&self) -> &'static [&'static str] {
        &["signing_secret"]
    }
    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        if !only_keys(
            context,
            config,
            &[
                "url",
                "method",
                "headers",
                "content_type",
                "body",
                "signing_secret",
                "signing_secret_configured",
                "retry_safe",
            ],
            &mut violations,
        ) {
            return Ok(violations);
        }
        match text(config, "url") {
            Some(url) if url.starts_with("https://") || url.starts_with("http://") => {
                if url.contains("{{") {
                    violations.push(context.violation("url", "cannot use templates"));
                }
            }
            Some(_) => violations.push(context.violation("url", "must be an http(s) URL")),
            None => violations.push(context.violation("url", "is required")),
        }
        if let Some(value) = config.get("method")
            && value.as_str().and_then(HttpMethod::parse).is_none()
        {
            violations.push(context.violation("method", "must be GET, POST, PUT, PATCH or DELETE"));
        }
        match config.get("headers") {
            None | Some(Value::Null) => {}
            Some(Value::Object(headers)) => {
                if headers.len() > MAX_EXTRA_HEADERS {
                    violations.push(context.violation(
                        "headers",
                        format!("can have at most {MAX_EXTRA_HEADERS} headers"),
                    ));
                }
                for (name, value) in headers {
                    if !valid_header_name(name) {
                        violations.push(
                            context
                                .violation("headers", format!("{name} is not a valid header name")),
                        );
                    } else if is_reserved_webhook_header(name) {
                        violations
                            .push(context.violation("headers", format!("{name} is reserved")));
                    } else if !value.is_string() {
                        violations
                            .push(context.violation("headers", format!("{name} must be text")));
                    }
                }
            }
            Some(_) => {
                violations.push(context.violation("headers", "must be an object of text values"))
            }
        }
        if let Some(value) = config.get("content_type")
            && value
                .as_str()
                .is_none_or(|text| text.trim().is_empty() || text.contains(['\r', '\n']))
        {
            violations.push(context.violation(
                "content_type",
                "must be a media type such as application/json",
            ));
        }
        if let Some(value) = config.get("retry_safe")
            && !value.is_boolean()
        {
            violations.push(context.violation("retry_safe", "must be true or false"));
        }
        if let Some(Value::String(secret)) = config.get("signing_secret")
            && secret.trim().len() < 16
        {
            violations
                .push(context.violation("signing_secret", "must have at least 16 characters"));
        }
        Ok(violations)
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let url = text(config, "url").ok_or_else(|| invalid("url is required"))?;
        let signing_secret = context
            .gateway
            .reveal_secret(context.scope, &context.secret_key("signing_secret"))
            .await?;
        let delivery_id = Uuid::now_v7();
        let result = context
            .gateway
            .call_webhook(WebhookCall {
                url: url.to_owned(),
                delivery_id,
                event_type: AUTOMATION_WEBHOOK_EVENT.to_owned(),
                body: self.body(context, config)?,
                signing_secret,
                method: text(config, "method")
                    .and_then(HttpMethod::parse)
                    .unwrap_or_default(),
                headers: config
                    .get("headers")
                    .and_then(Value::as_object)
                    .map(|headers| {
                        headers
                            .iter()
                            .filter_map(|(name, value)| {
                                value.as_str().map(|value| (name.clone(), value.to_owned()))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                content_type: text(config, "content_type").map(str::to_owned),
            })
            .await?;
        let output = json!({
            "delivery_id": delivery_id.to_string(),
            "status": result.status,
            "duration_ms": result.duration_ms,
        });
        if result.succeeded() {
            return Ok(output);
        }
        Err(AppError::Persistence(
            match (&result.error, result.status) {
                (Some(error), _) => format!("webhook call failed: {error}"),
                (None, Some(status)) => format!("webhook answered HTTP {status}"),
                (None, None) => "webhook call failed".to_owned(),
            },
        ))
    }
}

fn is_reserved_webhook_header(name: &str) -> bool {
    is_reserved_transport_header(name) || name.trim().to_ascii_lowercase().starts_with("x-aphel-")
}
