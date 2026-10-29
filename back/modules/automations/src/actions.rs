use async_trait::async_trait;
use serde_json::{Map, Value, json};

use aphel_kernel::application::automations::actions::*;
use aphel_kernel::application::automations::gateway::{MessageChannel, MessageSpec};
use aphel_kernel::application::automations::triggers::is_automatable;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) struct UpdateFields;

const ACTIVITY_UPDATABLE_FIELDS: [&str; 9] = [
    "title",
    "activity_type_id",
    "status",
    "priority",
    "due_at",
    "completed_at",
    "owner_id",
    "duration_minutes",
    "description",
];

fn activity_field_violations(
    context: &ValidationContext<'_>,
    values: Option<&Map<String, Value>>,
    violations: &mut Vec<FieldViolation>,
) {
    for key in values.into_iter().flat_map(Map::keys) {
        if context
            .fields
            .find(key)
            .is_some_and(|field| field.is_system())
            && !ACTIVITY_UPDATABLE_FIELDS.contains(&key.as_str())
        {
            violations.push(context.violation(
                &format!("values.{key}"),
                "cannot be changed by an automation on activities",
            ));
        }
    }
}

#[async_trait]
impl AutomationAction for UpdateFields {
    fn key(&self) -> &'static str {
        UPDATE_FIELDS
    }
    fn label(&self) -> &'static str {
        "Atualizar campos"
    }
    fn description(&self) -> &'static str {
        "Changes system or custom fields of the record; values accept templates and `null` clears a field"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "values": { "type": "object", "additionalProperties": true } },
            "required": ["values"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
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
        if only_keys(context, config, &["values"], &mut violations) {
            validate_values(
                context,
                object(config, "values"),
                "values",
                &mut violations,
                context.fields,
            )
            .await;
            if context.record_type == RecordType::ACTIVITY.as_str() {
                activity_field_violations(context, object(config, "values"), &mut violations);
            }
        }
        Ok(violations)
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let values = object(config, "values")
            .cloned()
            .ok_or_else(|| invalid("values must be an object"))?;
        let keys: Vec<String> = values.keys().cloned().collect();
        context
            .gateway
            .update_record(context.scope, context.record, values)
            .await?;
        Ok(json!({ "updated_fields": keys }))
    }
}

pub(crate) struct AssignOwner;

#[async_trait]
impl AutomationAction for AssignOwner {
    fn key(&self) -> &'static str {
        ASSIGN_OWNER
    }
    fn label(&self) -> &'static str {
        "Atribuir responsável"
    }
    fn description(&self) -> &'static str {
        "Sets the owner of the record to a fixed user or to the user stored in a user field"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "owner": user_spec_schema() },
            "required": ["owner"],
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
        if !only_keys(context, config, &["owner"], &mut violations) {
            return Ok(violations);
        }
        if context.fields.find("owner_id").is_none() {
            violations.push(context.violation("", "this record type has no owner"));
            return Ok(violations);
        }
        match config.get("owner") {
            Some(spec) => validate_user_spec(context, Some(spec), "owner", &mut violations).await?,
            None => violations.push(context.violation("owner", "is required")),
        }
        Ok(violations)
    }
    async fn describe(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let owner = resolve_user(context, config.get("owner").unwrap_or(&Value::Null))?;
        Ok(json!({ "owner_id": owner }))
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let owner = resolve_user(context, config.get("owner").unwrap_or(&Value::Null))?;
        let mut values = Map::new();
        values.insert(
            "owner_id".to_owned(),
            owner.clone().map(Value::String).unwrap_or(Value::Null),
        );
        context
            .gateway
            .update_record(context.scope, context.record, values)
            .await?;
        Ok(json!({ "owner_id": owner }))
    }
}

pub(crate) struct SendMessage {
    pub(crate) channel: MessageChannel,
}

impl SendMessage {
    fn title_key(&self) -> &'static str {
        match self.channel {
            MessageChannel::InApp => "title",
            MessageChannel::Email => "subject",
        }
    }

    fn recipients(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Vec<UserId>, AppError> {
        let mut recipients = Vec::new();
        for spec in config
            .get("recipients")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(id) = resolve_user(context, spec)?
                && let Ok(user) = UserId::new(id)
                && !recipients.contains(&user)
            {
                recipients.push(user);
            }
        }
        Ok(recipients)
    }
}

#[async_trait]
impl AutomationAction for SendMessage {
    fn key(&self) -> &'static str {
        match self.channel {
            MessageChannel::InApp => SEND_NOTIFICATION,
            MessageChannel::Email => SEND_EMAIL,
        }
    }
    fn label(&self) -> &'static str {
        match self.channel {
            MessageChannel::InApp => "Enviar notificação",
            MessageChannel::Email => "Enviar e-mail para usuário",
        }
    }
    fn description(&self) -> &'static str {
        match self.channel {
            MessageChannel::InApp => {
                "Sends an in-app notification to members of the organization (record owner, fixed users or user fields)"
            }
            MessageChannel::Email => {
                "Sends an e-mail to members of the organization through the notification e-mail job"
            }
        }
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "recipients": { "type": "array", "items": user_spec_schema(), "maxItems": MAX_RECIPIENTS },
                (self.title_key()): { "type": "string", "maxLength": MAX_TITLE_LENGTH },
                "body": { "type": "string", "maxLength": MAX_BODY_LENGTH },
            },
            "required": ["recipients", self.title_key()],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        Vec::new()
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
            &["recipients", self.title_key(), "body"],
            &mut violations,
        ) {
            return Ok(violations);
        }
        required_text(
            context,
            config,
            self.title_key(),
            MAX_TITLE_LENGTH,
            &mut violations,
        );
        optional_text(context, config, "body", MAX_BODY_LENGTH, &mut violations);
        match config.get("recipients").and_then(Value::as_array) {
            Some(items) if !items.is_empty() && items.len() <= MAX_RECIPIENTS => {
                for (index, item) in items.iter().enumerate() {
                    validate_user_spec(
                        context,
                        Some(item),
                        &format!("recipients.{index}"),
                        &mut violations,
                    )
                    .await?;
                }
            }
            _ => violations.push(context.violation(
                "recipients",
                format!("must have 1 to {MAX_RECIPIENTS} recipients"),
            )),
        }
        Ok(violations)
    }
    async fn describe(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let recipients = self.recipients(context, config)?;
        Ok(json!({ "recipients": recipients.iter().map(UserId::as_str).collect::<Vec<_>>() }))
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let recipients = self.recipients(context, config)?;
        if recipients.is_empty() {
            return Ok(json!({ "recipients": [] }));
        }
        let title = text(config, self.title_key())
            .ok_or_else(|| invalid("the message title is empty"))?
            .chars()
            .take(MAX_TITLE_LENGTH)
            .collect();
        context
            .gateway
            .send_message(
                context.scope,
                MessageSpec {
                    automation_id: context.automation_id,
                    run_id: context.run_id,
                    record: Some(context.record.clone()),
                    recipients: recipients.clone(),
                    channel: self.channel,
                    title,
                    body: text(config, "body").map(str::to_owned),
                },
            )
            .await?;
        Ok(json!({ "recipients": recipients.iter().map(UserId::as_str).collect::<Vec<_>>() }))
    }
}

pub(crate) struct CreateRelatedRecord;

#[async_trait]
impl AutomationAction for CreateRelatedRecord {
    fn key(&self) -> &'static str {
        CREATE_RELATED_RECORD
    }
    fn label(&self) -> &'static str {
        "Criar registro relacionado"
    }
    fn description(&self) -> &'static str {
        "Creates a record of another type (contact, company, deal or custom object) with templated values and optionally links it through a relationship of the current record"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "record_type": { "type": "string" },
                "values": { "type": "object", "additionalProperties": true },
                "relationship": { "type": "string", "description": "Relationship key on the current record" },
            },
            "required": ["record_type", "values"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn required_permissions(&self, record_type: &str, config: &Value) -> Vec<Permission> {
        let mut permissions: Vec<Permission> = text(config, "record_type")
            .and_then(|target| record_permission(target, "create"))
            .into_iter()
            .collect();
        if text(config, "relationship").is_some()
            && let Some(update) = record_permission(record_type, "update")
        {
            permissions.push(update);
        }
        permissions
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
            &["record_type", "values", "relationship"],
            &mut violations,
        ) {
            return Ok(violations);
        }
        let Some(target) = text(config, "record_type") else {
            violations.push(context.violation("record_type", "is required"));
            return Ok(violations);
        };
        if !is_automatable(target) || target == RecordType::ACTIVITY.as_str() {
            violations.push(context.violation(
                "record_type",
                "must be contact, company, deal or a custom object (use create_task for activities)",
            ));
            return Ok(violations);
        }
        let target_fields = match context.gateway.fields(context.scope, target).await {
            Ok(fields) => fields,
            Err(_) => {
                violations.push(context.violation("record_type", "is not available"));
                return Ok(violations);
            }
        };
        validate_values(
            context,
            object(config, "values"),
            "values",
            &mut violations,
            &target_fields,
        )
        .await;
        if let Some(relationship) = text(config, "relationship") {
            let keys = context.gateway.relation_keys(context.record_type).await?;
            if !keys.iter().any(|key| key == relationship) {
                violations.push(
                    context.violation("relationship", "is not a relationship of this record type"),
                );
            }
        }
        Ok(violations)
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let target =
            text(config, "record_type").ok_or_else(|| invalid("record_type is required"))?;
        let values = object(config, "values")
            .cloned()
            .ok_or_else(|| invalid("values must be an object"))?;
        let id = context
            .gateway
            .create_record(context.scope, target, values)
            .await?;
        if let Some(relationship) = text(config, "relationship") {
            context
                .gateway
                .link(context.scope, context.record, relationship, id)
                .await?;
        }
        Ok(json!({ "record_type": target, "record_id": id.to_string() }))
    }
}
