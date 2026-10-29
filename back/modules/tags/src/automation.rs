use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::automations::actions::{
    ADD_TAG, ActionContext, AutomationAction, REMOVE_TAG, ValidationContext, invalid, only_keys,
    record_permission, text,
};
use aphel_kernel::application::automations::gateway::Reference;
use aphel_kernel::application::automations::triggers::is_automatable;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) struct Tag {
    pub(crate) present: bool,
}

#[async_trait]
impl AutomationAction for Tag {
    fn key(&self) -> &'static str {
        if self.present { ADD_TAG } else { REMOVE_TAG }
    }
    fn label(&self) -> &'static str {
        if self.present {
            "Adicionar etiqueta"
        } else {
            "Remover etiqueta"
        }
    }
    fn description(&self) -> &'static str {
        if self.present {
            "Adds a tag to the record (nothing happens when it already has it)"
        } else {
            "Removes a tag from the record (nothing happens when it does not have it)"
        }
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "tag_id": { "type": "string", "format": "uuid" } },
            "required": ["tag_id"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type) && record_type != RecordType::ACTIVITY.as_str()
    }
    fn required_permissions(&self, record_type: &str, _config: &Value) -> Vec<Permission> {
        record_permission(record_type, "update")
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
        let mut violations = Vec::new();
        if !only_keys(context, config, &["tag_id"], &mut violations) {
            return Ok(violations);
        }
        match text(config, "tag_id") {
            Some(id) => {
                context
                    .reference(&mut violations, "tag_id", Reference::Tag(id))
                    .await?
            }
            None => violations.push(context.violation("tag_id", "is required")),
        }
        Ok(violations)
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let tag_id = text(config, "tag_id").ok_or_else(|| invalid("tag_id is required"))?;
        context
            .gateway
            .set_tag(context.scope, context.record, tag_id, self.present)
            .await?;
        Ok(json!({ "tag_id": tag_id }))
    }
}
