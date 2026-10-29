use async_trait::async_trait;
use serde_json::{Value, json};

use crate::service::{self, GENERATE, PROPOSAL_GENERATED, TemplateChoice};
use crate::store;
use aphel_kernel::application::automations::actions::{
    ActionContext, AutomationAction, ValidationContext,
};
use aphel_kernel::application::automations::triggers::{
    AutomationTrigger, TriggerCandidate, event_context,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::runtime_state;

pub(crate) const GENERATE_ACTION: &str = "proposals.generate_proposal";
const DEAL: &str = "deal";

pub(crate) struct ProposalGeneratedTrigger;

impl AutomationTrigger for ProposalGeneratedTrigger {
    fn key(&self) -> &'static str {
        PROPOSAL_GENERATED
    }

    fn label(&self) -> &'static str {
        "Proposta gerada"
    }

    fn description(&self) -> &'static str {
        "Quando uma proposta em PDF é gerada e anexada à oportunidade"
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == DEAL
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == PROPOSAL_GENERATED
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        match &event.record {
            Some(record) if self.supports(record.record_type.as_str()) => {
                vec![TriggerCandidate {
                    record: record.clone(),
                    context: event_context(PROPOSAL_GENERATED, event),
                }]
            }
            _ => Vec::new(),
        }
    }
}

pub(crate) struct GenerateProposalAction;

fn template_key(config: &Value) -> Option<&str> {
    config
        .get("template_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
}

#[async_trait]
impl AutomationAction for GenerateProposalAction {
    fn key(&self) -> &'static str {
        GENERATE_ACTION
    }

    fn label(&self) -> &'static str {
        "Gerar proposta"
    }

    fn description(&self) -> &'static str {
        "Gera a proposta em PDF a partir de um modelo e anexa à oportunidade"
    }

    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "template_key": {
                    "type": "string",
                    "maxLength": 64,
                    "description": "Chave do modelo; vazio usa o modelo padrão"
                }
            },
            "additionalProperties": false
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == DEAL
    }

    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        vec![GENERATE, Permission::DEALS_UPDATE]
    }

    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        if let Some(object) = config.as_object() {
            for key in object.keys().filter(|key| key.as_str() != "template_key") {
                violations.push(context.violation(key, "unknown property"));
            }
            if object
                .get("template_key")
                .is_some_and(|value| !value.is_string() && !value.is_null())
            {
                violations.push(context.violation("template_key", "must be a string"));
            }
        } else if !config.is_null() {
            violations.push(context.violation("", "must be an object"));
        }
        let pool = runtime_state()?.db()?;
        let template = match template_key(config) {
            Some(key) => store::find_template_by_key(&pool, key).await?,
            None => store::default_template(&pool).await?,
        };
        match template {
            Some(template) if template.is_active => {}
            Some(_) => {
                violations.push(context.violation("template_key", "the template is inactive"))
            }
            None if template_key(config).is_some() => {
                violations.push(context.violation("template_key", "template not found"))
            }
            None => {
                violations.push(context.violation("template_key", "there is no active template"))
            }
        }
        Ok(violations)
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let choice = match template_key(config) {
            Some(key) => TemplateChoice::Key(key.to_owned()),
            None => TemplateChoice::Default,
        };
        let state = runtime_state()?;
        let ctx = Ctx::from_state(&state, context.scope).await?;
        let document = service::generate(&ctx, context.record.record_id, choice, None).await?;
        Ok(json!({
            "document_id": document.id,
            "number": document.number,
            "file_id": document.file_id,
            "file_name": document.file_name,
            "template_name": document.template_name,
        }))
    }
}
