use aphel_contract_crm::CrmPermissions;
use aphel_contract_crm::refs;
use async_trait::async_trait;
use chrono::SecondsFormat;
use serde_json::{Value, json};

use crate::domain::ACTIVITY_COMPLETED;
use aphel_contract_automation::application::actions::{
    ActionContext, AutomationAction, CREATE_TASK, MAX_BODY_LENGTH, MAX_TITLE_LENGTH,
    ValidationContext, invalid, only_keys, optional_text, parent_record, required_text,
    resolve_user, text, user_spec_schema, validate_user_spec,
};
use aphel_contract_automation::application::gateway::{Reference, TaskSpec};
use aphel_contract_automation::application::templates::parse_offset;
use aphel_contract_automation::application::triggers::{
    AutomationTrigger, TriggerCandidate, event_context, is_automatable, parent_records,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) struct ActivityCompleted;

impl AutomationTrigger for ActivityCompleted {
    fn key(&self) -> &'static str {
        ACTIVITY_COMPLETED
    }
    fn label(&self) -> &'static str {
        "Atividade concluída"
    }
    fn description(&self) -> &'static str {
        "An activity was completed; on activities it runs for the activity itself, on other types for the records the activity is linked to"
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type == ACTIVITY_COMPLETED
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        let Some(activity) = event.record.as_ref() else {
            return Vec::new();
        };
        let mut context = event_context(ACTIVITY_COMPLETED, event);
        if let Some(object) = context.payload.as_object_mut() {
            object.insert(
                "activity_id".to_owned(),
                Value::String(activity.record_id.to_string()),
            );
        }
        std::iter::once(activity.clone())
            .chain(parent_records(&event.payload, true))
            .map(|record| TriggerCandidate {
                record,
                context: context.clone(),
            })
            .collect()
    }
}

pub(crate) struct CreateTask;

impl CreateTask {
    fn due_at(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Option<String>, AppError> {
        if let Some(due_in) = text(config, "due_in") {
            let offset = parse_offset(due_in).map_err(invalid)?;
            return Ok(Some(
                (context.now + offset).to_rfc3339_opts(SecondsFormat::Secs, true),
            ));
        }
        Ok(text(config, "due_at").map(str::to_owned))
    }

    fn task(&self, context: &ActionContext<'_>, config: &Value) -> Result<TaskSpec, AppError> {
        let owner = match config.get("assignee") {
            Some(spec) => resolve_user(context, spec)?,
            None => resolve_user(context, &json!({ "type": "record_owner" }))?,
        };
        let link = config
            .get("link_to_record")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        Ok(TaskSpec {
            title: text(config, "title")
                .ok_or_else(|| invalid("the task title is empty"))?
                .chars()
                .take(MAX_TITLE_LENGTH)
                .collect(),
            description: text(config, "description").map(str::to_owned),
            activity_type_id: text(config, "activity_type_id").map(str::to_owned),
            owner_id: owner,
            due_at: self.due_at(context, config)?,
            priority: text(config, "priority").map(str::to_owned),
            record: parent_record(context).filter(|_| link),
        })
    }
}

#[async_trait]
impl AutomationAction for CreateTask {
    fn key(&self) -> &'static str {
        CREATE_TASK
    }
    fn label(&self) -> &'static str {
        "Criar tarefa"
    }
    fn description(&self) -> &'static str {
        "Creates a task linked to the record; the assignee defaults to the record owner and the due date is relative (`due_in`: 30m, 2h, 2d, 1w) or a template (`due_at`)"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "maxLength": MAX_TITLE_LENGTH },
                "description": { "type": "string" },
                "activity_type_id": { "type": "string", "format": "uuid" },
                "assignee": user_spec_schema(),
                "due_in": { "type": "string", "examples": ["2d", "4h"] },
                "due_at": { "type": "string", "examples": ["{{now + 2d}}"] },
                "priority": { "enum": ["low", "normal", "high"] },
                "link_to_record": { "type": "boolean", "default": true },
            },
            "required": ["title"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        vec![Permission::ACTIVITIES_CREATE]
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
                "title",
                "description",
                "activity_type_id",
                "assignee",
                "due_in",
                "due_at",
                "priority",
                "link_to_record",
            ],
            &mut violations,
        ) {
            return Ok(violations);
        }
        required_text(context, config, "title", MAX_TITLE_LENGTH, &mut violations);
        optional_text(
            context,
            config,
            "description",
            MAX_BODY_LENGTH,
            &mut violations,
        );
        if let Some(due_in) = text(config, "due_in")
            && let Err(message) = parse_offset(due_in)
        {
            violations.push(context.violation("due_in", message));
        }
        if text(config, "due_in").is_some() && text(config, "due_at").is_some() {
            violations.push(context.violation("due_at", "use either due_in or due_at"));
        }
        if let Some(priority) = text(config, "priority")
            && !["low", "normal", "high"].contains(&priority)
        {
            violations.push(context.violation("priority", "must be low, normal or high"));
        }
        if let Some(id) = text(config, "activity_type_id") {
            context
                .reference(
                    &mut violations,
                    "activity_type_id",
                    Reference::new(refs::ACTIVITY_TYPE, id),
                )
                .await?;
        }
        validate_user_spec(context, config.get("assignee"), "assignee", &mut violations).await?;
        Ok(violations)
    }
    async fn describe(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let task = self.task(context, config)?;
        Ok(json!({
            "title": task.title,
            "owner_id": task.owner_id,
            "due_at": task.due_at,
            "record": task.record.map(|record| json!({ "record_type": record.record_type.as_str(), "record_id": record.record_id.to_string() })),
        }))
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let task = self.task(context, config)?;
        let id = context.gateway.create_task(context.scope, task).await?;
        Ok(json!({ "activity_id": id.to_string() }))
    }
}
