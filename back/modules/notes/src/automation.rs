use async_trait::async_trait;
use serde_json::{Value, json};

use crate::domain::NOTE_CREATED;
use aphel_kernel::application::automations::actions::{
    ActionContext, AutomationAction, CREATE_NOTE, MAX_BODY_LENGTH, ValidationContext, invalid,
    only_keys, parent_record, required_text, text,
};
use aphel_kernel::application::automations::triggers::{
    AutomationTrigger, TriggerCandidate, event_context, is_automatable, parent_records,
};
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) struct NoteCreated;

impl AutomationTrigger for NoteCreated {
    fn key(&self) -> &'static str {
        NOTE_CREATED
    }
    fn label(&self) -> &'static str {
        "Nota criada"
    }
    fn description(&self) -> &'static str {
        "A note was added to a record of the automation type"
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type) && record_type != RecordType::ACTIVITY.as_str()
    }
    fn listens_to(&self, event_type: &str) -> bool {
        event_type == NOTE_CREATED
    }
    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        let mut context = event_context(NOTE_CREATED, event);
        if let (Some(note), Some(object)) = (event.record.as_ref(), context.payload.as_object_mut())
        {
            object.insert(
                "note_id".to_owned(),
                Value::String(note.record_id.to_string()),
            );
        }
        parent_records(&event.payload, false)
            .into_iter()
            .map(|record| TriggerCandidate {
                record,
                context: context.clone(),
            })
            .collect()
    }
}

pub(crate) struct CreateNote;

#[async_trait]
impl AutomationAction for CreateNote {
    fn key(&self) -> &'static str {
        CREATE_NOTE
    }
    fn label(&self) -> &'static str {
        "Criar nota"
    }
    fn description(&self) -> &'static str {
        "Adds a Markdown note to the record (the parent record when the automation runs on activities)"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "body": { "type": "string", "maxLength": MAX_BODY_LENGTH } },
            "required": ["body"],
            "additionalProperties": false,
        })
    }
    fn supports(&self, record_type: &str) -> bool {
        is_automatable(record_type)
    }
    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        vec![Permission::NOTES_CREATE]
    }
    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        if only_keys(context, config, &["body"], &mut violations) {
            required_text(context, config, "body", MAX_BODY_LENGTH, &mut violations);
        }
        Ok(violations)
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let body = text(config, "body").ok_or_else(|| invalid("the note body is empty"))?;
        let record = parent_record(context)
            .ok_or_else(|| invalid("the activity is not linked to a record"))?;
        let id = context
            .gateway
            .create_note(context.scope, &record, body)
            .await?;
        Ok(json!({ "note_id": id.to_string() }))
    }
}
