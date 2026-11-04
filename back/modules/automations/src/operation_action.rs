use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value, json};

use aphel_contract_automation::application::actions::{
    ActionContext, AutomationAction, ValidationContext, invalid, only_keys, record_permission, text,
};
use aphel_contract_automation::application::gateway::Reference;
use aphel_contract_automation::application::triggers::is_automatable;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::operations::{AutomationBinding, OperationEntry};

pub struct OperationAction {
    entry: Arc<OperationEntry>,
    binding: AutomationBinding,
}

impl OperationAction {
    pub fn new(entry: Arc<OperationEntry>) -> Option<Self> {
        let binding = entry.spec.automation.clone()?;
        Some(Self { entry, binding })
    }

    fn properties(&self) -> Vec<String> {
        self.binding
            .config_schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|properties| properties.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn required(&self) -> Vec<String> {
        self.binding
            .config_schema
            .get("required")
            .and_then(Value::as_array)
            .map(|required| {
                required
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl AutomationAction for OperationAction {
    fn key(&self) -> &'static str {
        self.binding.action_key
    }

    fn label(&self) -> &'static str {
        self.binding.label
    }

    fn description(&self) -> &'static str {
        self.entry.spec.description
    }

    fn config_schema(&self) -> Value {
        self.binding.config_schema.clone()
    }

    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type) && self.binding.supports_record_type(record_type)
    }

    fn required_permissions(&self, record_type: &str, _config: &Value) -> Vec<Permission> {
        let from_spec = self
            .entry
            .spec
            .permission
            .and_then(|permission| Permission::parse(permission).ok());
        let from_record = self
            .binding
            .record_permission
            .and_then(|action| record_permission(record_type, action));
        from_spec.into_iter().chain(from_record).collect()
    }

    fn retry_safe(&self, _config: &Value) -> bool {
        self.entry.spec.idempotent
    }

    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        let properties = self.properties();
        let allowed: Vec<&str> = properties.iter().map(String::as_str).collect();
        if !only_keys(context, config, &allowed, &mut violations) {
            return Ok(violations);
        }
        for field in self.required() {
            if text(config, &field).is_none() {
                violations.push(context.violation(&field, "is required"));
            }
        }
        if let Some(check) = self.binding.reference
            && let Some(id) = text(config, check.field)
        {
            context
                .reference(&mut violations, check.field, Reference::new(check.kind, id))
                .await?;
        }
        Ok(violations)
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let mut input: Map<String, Value> = config.as_object().cloned().unwrap_or_default();
        for field in self.required() {
            if text(config, &field).is_none() {
                return Err(invalid(format!("{field} is required")));
            }
        }
        input.insert(
            "record_type".to_owned(),
            json!(context.record.record_type.as_str()),
        );
        input.insert(
            "record_id".to_owned(),
            json!(context.record.record_id.to_string()),
        );
        context
            .gateway
            .call_operation(context.scope, self.entry.key, Value::Object(input))
            .await?;
        Ok(config.clone())
    }
}
