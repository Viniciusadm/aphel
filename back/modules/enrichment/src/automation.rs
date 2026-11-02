use aphel_contract_crm::CrmPermissions;
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::service;
use crate::{AUTOMATIC_SETTING, COMPANY, COMPANY_ENRICHED, KEY, USE};
use aphel_contract_automation::application::actions::{
    ActionContext, AutomationAction, ValidationContext,
};
use aphel_contract_automation::application::triggers::{
    AutomationTrigger, TriggerCandidate, event_context,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::module_settings;
use aphel_kernel::state::runtime_state;

pub(crate) const ENRICH_ACTION: &str = "enrichment.enrich_company";
pub(crate) const AUTOMATIC_DISABLED: &str = "enrichment_automatic_disabled";

pub(crate) struct EnrichCompanyAction;

fn overwrite(config: &Value) -> bool {
    config
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn automatic(settings: &Value) -> bool {
    settings
        .get(AUTOMATIC_SETTING)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

#[async_trait]
impl AutomationAction for EnrichCompanyAction {
    fn key(&self) -> &'static str {
        ENRICH_ACTION
    }

    fn label(&self) -> &'static str {
        "Enriquecer empresa pelo CNPJ"
    }

    fn description(&self) -> &'static str {
        "Consulta o CNPJ da empresa na fonte pública configurada e preenche os campos mapeados. Só funciona com \"Permitir enriquecimento automático\" ligado nas configurações da extensão"
    }

    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "overwrite": {
                    "type": "boolean",
                    "description": "Substituir valores já preenchidos (padrão: só preenche campos vazios)"
                }
            },
            "additionalProperties": false
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == COMPANY
    }

    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        vec![
            USE,
            Permission::COMPANIES_READ,
            Permission::COMPANIES_UPDATE,
        ]
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
        match config {
            Value::Object(object) => {
                for key in object.keys().filter(|key| key.as_str() != "overwrite") {
                    violations.push(context.violation(key, "unknown property"));
                }
                if object
                    .get("overwrite")
                    .is_some_and(|value| !value.is_boolean() && !value.is_null())
                {
                    violations.push(context.violation("overwrite", "must be a boolean"));
                }
            }
            Value::Null => {}
            _ => violations.push(context.violation("", "must be an object")),
        }
        let state = runtime_state()?;
        if !automatic(&module_settings(&state.db()?, KEY).await?) {
            violations.push(context.violation(
                "",
                "ligue \"Permitir enriquecimento automático\" nas configurações da extensão",
            ));
        }
        Ok(violations)
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let state = runtime_state()?;
        let ctx = Ctx::from_state(&state, context.scope).await?;
        if !automatic(&service::settings(&ctx).await?) {
            return Err(AppError::validation_code(
                AUTOMATIC_DISABLED,
                "o enriquecimento automático está desligado nas configurações da extensão",
            ));
        }
        let lookup = service::lookup(&ctx, context.record.record_id, None).await?;
        match service::apply(
            &ctx,
            lookup.id,
            &service::all_item_keys(),
            !overwrite(config),
        )
        .await
        {
            Ok(applied) => Ok(json!({
                "source": applied.source,
                "fields": applied.fields,
            })),
            Err(error) if service::is_nothing_to_apply(&error) => Ok(json!({
                "source": lookup.source,
                "fields": [],
            })),
            Err(error) => Err(error),
        }
    }
}

pub(crate) struct CompanyEnrichedTrigger;

impl AutomationTrigger for CompanyEnrichedTrigger {
    fn key(&self) -> &'static str {
        COMPANY_ENRICHED
    }

    fn label(&self) -> &'static str {
        "Empresa enriquecida pelo CNPJ"
    }

    fn description(&self) -> &'static str {
        "Quando campos de uma empresa são preenchidos com dados públicos do CNPJ"
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == COMPANY
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == COMPANY_ENRICHED
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        match &event.record {
            Some(record) if self.supports(record.record_type.as_str()) => {
                vec![TriggerCandidate {
                    record: record.clone(),
                    context: event_context(COMPANY_ENRICHED, event),
                }]
            }
            _ => Vec::new(),
        }
    }
}
