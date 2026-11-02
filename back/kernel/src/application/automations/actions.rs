use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::automations::gateway::{AutomationGateway, Reference, USER};
use crate::application::automations::templates::CUSTOM_FIELDS_KEY;
use crate::application::automations::triggers::is_attached_to_parent;
use crate::application::fields::FieldSet;
use crate::application::secrets::SecretKey;
use crate::application::shared::RecordRef;
use crate::domain::access::{AccessScope, Permission};
use crate::domain::custom_objects::is_custom_record_type;
use crate::domain::fields::FieldType;
use crate::errors::{AppError, FieldViolation};

pub const CREATE_TASK: &str = "create_task";
pub const UPDATE_FIELDS: &str = "update_fields";
pub const ASSIGN_OWNER: &str = "assign_owner";
pub const ADD_TAG: &str = "add_tag";
pub const REMOVE_TAG: &str = "remove_tag";
pub const MOVE_STAGE: &str = "move_stage";
pub const CREATE_NOTE: &str = "create_note";
pub const SEND_NOTIFICATION: &str = "send_notification";
pub const SEND_EMAIL: &str = "send_email";
pub const CALL_WEBHOOK: &str = "call_webhook";
pub const CREATE_RELATED_RECORD: &str = "create_related_record";

pub const SECRET_OWNER_TYPE: &str = "automation";
pub const AUTOMATION_WEBHOOK_EVENT: &str = "automation.webhook";
pub const MAX_RECIPIENTS: usize = 20;
pub const MAX_UPDATE_FIELDS: usize = 30;
pub const MAX_TITLE_LENGTH: usize = 255;
pub const MAX_BODY_LENGTH: usize = 10_000;
pub const MAX_WEBHOOK_BODY_BYTES: usize = 64 * 1024;

pub fn record_permission(record_type: &str, action: &str) -> Option<Permission> {
    if is_custom_record_type(record_type) {
        return Permission::parse(format!("{record_type}.{action}")).ok();
    }
    crate::modules::app()
        .record_types()
        .require(record_type)
        .ok()?
        .action_permission(action)
}

pub fn secret_key(automation_id: Uuid, step_id: &str, field: &str) -> SecretKey {
    SecretKey::new(
        SECRET_OWNER_TYPE,
        automation_id.to_string(),
        format!("{step_id}.{field}"),
    )
}

pub fn configured_flag(field: &str) -> String {
    format!("{field}_configured")
}

pub struct ActionContext<'a> {
    pub scope: &'a AccessScope,
    pub gateway: &'a dyn AutomationGateway,
    pub record: &'a RecordRef,
    pub record_data: &'a Value,
    pub fields: &'a FieldSet,
    pub automation_id: Uuid,
    pub run_id: Uuid,
    pub step_id: &'a str,
    pub now: DateTime<Utc>,
}

impl ActionContext<'_> {
    pub fn record_value(&self, key: &str) -> Option<&Value> {
        self.record_data
            .get(key)
            .filter(|value| !value.is_null())
            .or_else(|| {
                self.record_data
                    .get(CUSTOM_FIELDS_KEY)
                    .and_then(|custom| custom.get(key))
                    .filter(|value| !value.is_null())
            })
    }

    pub fn secret_key(&self, field: &str) -> SecretKey {
        secret_key(self.automation_id, self.step_id, field)
    }
}

pub struct ValidationContext<'a> {
    pub scope: &'a AccessScope,
    pub gateway: &'a dyn AutomationGateway,
    pub record_type: &'a str,
    pub fields: &'a FieldSet,
    pub path: String,
}

impl ValidationContext<'_> {
    pub fn violation(&self, field: &str, message: impl Into<String>) -> FieldViolation {
        if field.is_empty() {
            FieldViolation::new(self.path.clone(), message)
        } else {
            FieldViolation::new(format!("{}.{field}", self.path), message)
        }
    }

    pub async fn reference(
        &self,
        violations: &mut Vec<FieldViolation>,
        field: &str,
        reference: Reference<'_>,
    ) -> Result<(), AppError> {
        if !self.gateway.reference_exists(self.scope, reference).await? {
            violations.push(self.violation(field, "was not found"));
        }
        Ok(())
    }
}

#[async_trait]
pub trait AutomationAction: Send + Sync {
    fn key(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn config_schema(&self) -> Value;
    fn supports(&self, record_type: &str) -> bool;
    fn required_permissions(&self, record_type: &str, config: &Value) -> Vec<Permission>;
    fn retry_safe(&self, _config: &Value) -> bool {
        false
    }
    fn secret_fields(&self) -> &'static [&'static str] {
        &[]
    }
    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError>;
    async fn describe(
        &self,
        _context: &ActionContext<'_>,
        _config: &Value,
    ) -> Result<Value, AppError> {
        Ok(Value::Null)
    }
    async fn execute(&self, context: &ActionContext<'_>, config: &Value)
    -> Result<Value, AppError>;
}

pub fn text<'a>(config: &'a Value, key: &str) -> Option<&'a str> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn object<'a>(config: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    config.get(key).and_then(Value::as_object)
}

pub fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

pub fn required_text(
    context: &ValidationContext<'_>,
    config: &Value,
    key: &str,
    max: usize,
    violations: &mut Vec<FieldViolation>,
) {
    match config.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => {
            if value.chars().count() > max {
                violations
                    .push(context.violation(key, format!("must have at most {max} characters")));
            }
        }
        _ => violations.push(context.violation(key, "is required")),
    }
}

pub fn optional_text(
    context: &ValidationContext<'_>,
    config: &Value,
    key: &str,
    max: usize,
    violations: &mut Vec<FieldViolation>,
) {
    match config.get(key) {
        None | Some(Value::Null) => {}
        Some(Value::String(value)) => {
            if value.chars().count() > max {
                violations
                    .push(context.violation(key, format!("must have at most {max} characters")));
            }
        }
        Some(_) => violations.push(context.violation(key, "must be a text")),
    }
}

pub fn only_keys(
    context: &ValidationContext<'_>,
    config: &Value,
    allowed: &[&str],
    violations: &mut Vec<FieldViolation>,
) -> bool {
    let Some(map) = config.as_object() else {
        violations.push(context.violation("", "must be an object"));
        return false;
    };
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            violations.push(context.violation(key, "is not a known option"));
        }
    }
    true
}

pub fn user_spec_schema() -> Value {
    json!({
        "oneOf": [
            { "type": "object", "properties": { "type": { "const": "record_owner" } }, "required": ["type"] },
            { "type": "object", "properties": { "type": { "const": "user" }, "user_id": { "type": "string", "format": "uuid" } }, "required": ["type", "user_id"] },
            { "type": "object", "properties": { "type": { "const": "field" }, "field": { "type": "string" } }, "required": ["type", "field"] },
        ]
    })
}

pub async fn validate_user_spec(
    context: &ValidationContext<'_>,
    spec: Option<&Value>,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Result<(), AppError> {
    let Some(spec) = spec else {
        return Ok(());
    };
    match spec.get("type").and_then(Value::as_str) {
        Some("record_owner") => {
            if context.fields.find("owner_id").is_none() {
                violations.push(context.violation(field, "this record type has no owner"));
            }
        }
        Some("user") => match spec.get("user_id").and_then(Value::as_str) {
            Some(id) if id.contains("{{") => {}
            Some(id) => {
                context
                    .reference(
                        violations,
                        &format!("{field}.user_id"),
                        Reference::new(USER, id),
                    )
                    .await?
            }
            None => violations.push(context.violation(&format!("{field}.user_id"), "is required")),
        },
        Some("field") => match spec.get("field").and_then(Value::as_str) {
            Some(key) => match context.fields.find(key) {
                Some(field_def)
                    if field_def.definition.field_type.base_type() == FieldType::User => {}
                Some(_) => violations
                    .push(context.violation(&format!("{field}.field"), "must be a user field")),
                None => violations.push(context.violation(
                    &format!("{field}.field"),
                    "is not a field of this record type",
                )),
            },
            None => violations.push(context.violation(&format!("{field}.field"), "is required")),
        },
        _ => violations.push(context.violation(
            &format!("{field}.type"),
            "must be record_owner, user or field",
        )),
    }
    Ok(())
}

pub fn resolve_user(context: &ActionContext<'_>, spec: &Value) -> Result<Option<String>, AppError> {
    let id = match spec.get("type").and_then(Value::as_str) {
        Some("record_owner") => context
            .record_value("owner_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        Some("user") => spec
            .get("user_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned),
        Some("field") => spec
            .get("field")
            .and_then(Value::as_str)
            .and_then(|key| context.record_value(key))
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => return Err(invalid("the user must be record_owner, user or field")),
    };
    Ok(id)
}

pub fn parent_record(context: &ActionContext<'_>) -> Option<RecordRef> {
    if !is_attached_to_parent(context.record.record_type.as_str()) {
        return Some(context.record.clone());
    }
    let record_type = context.record_data.get("record_type")?.as_str()?;
    let record_id = context.record_data.get("record_id")?.as_str()?;
    RecordRef::parse(record_type, record_id).ok()
}

pub trait AutomationActionSource: Send + Sync {
    fn actions(&self) -> Vec<Arc<dyn AutomationAction>>;
}

#[derive(Clone, Default)]
pub struct ActionRegistry {
    actions: Vec<Arc<dyn AutomationAction>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builtin() -> Self {
        let app = crate::modules::app();
        Self::new()
            .with_all(app.enabled_actions().cloned().collect())
            .with_all(
                app.contributions::<dyn AutomationActionSource>()
                    .iter()
                    .flat_map(|source| source.actions())
                    .collect(),
            )
    }

    pub fn with_all(self, actions: Vec<Arc<dyn AutomationAction>>) -> Self {
        actions
            .into_iter()
            .fold(self, |registry, action| registry.with(action))
    }

    pub fn with(mut self, action: Arc<dyn AutomationAction>) -> Self {
        self.actions
            .retain(|existing| existing.key() != action.key());
        self.actions.push(action);
        self
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn AutomationAction>> {
        self.actions.iter().find(|action| action.key() == key)
    }

    pub fn all(&self) -> &[Arc<dyn AutomationAction>] {
        &self.actions
    }
}

pub async fn validate_values(
    context: &ValidationContext<'_>,
    values: Option<&Map<String, Value>>,
    field: &str,
    violations: &mut Vec<FieldViolation>,
    fields: &FieldSet,
) {
    let Some(values) = values else {
        violations.push(context.violation(field, "must be an object of field keys and values"));
        return;
    };
    if values.is_empty() || values.len() > MAX_UPDATE_FIELDS {
        violations
            .push(context.violation(field, format!("must have 1 to {MAX_UPDATE_FIELDS} fields")));
    }
    for key in values.keys() {
        match fields.find(key) {
            None => violations.push(context.violation(
                &format!("{field}.{key}"),
                "is not a field of this record type",
            )),
            Some(resolved) if resolved.is_hidden() || !resolved.is_editable() => violations.push(
                context.violation(&format!("{field}.{key}"), "cannot be changed by this user"),
            ),
            Some(_) => {}
        }
    }
}
