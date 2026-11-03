use crate::conditions::{record_part, trigger_violations};
use aphel_contract_automation::application::actions::{ActionRegistry, ValidationContext};
use aphel_contract_automation::application::definition::{
    AutomationDefinition, MAX_DEFINITION_BYTES, TriggerSpec,
};
use aphel_contract_automation::application::gateway::{AutomationGateway, Reference};
use aphel_contract_automation::application::templates::TemplateRules;
use aphel_contract_automation::application::time::waits::wait_violations;
use aphel_contract_automation::application::triggers::{
    TriggerReference, TriggerRegistry, is_automatable,
};
use aphel_contract_crm::refs;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::query::Filter;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) struct AutomationValidator<'a> {
    pub(crate) scope: &'a AccessScope,
    pub(crate) gateway: &'a dyn AutomationGateway,
    pub(crate) triggers: &'a TriggerRegistry,
    pub(crate) actions: &'a ActionRegistry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationLevel {
    Save,
    Publish,
}

impl AutomationValidator<'_> {
    pub(crate) async fn fields(
        &self,
        record_type: &str,
    ) -> Result<Result<FieldSet, FieldViolation>, AppError> {
        if !is_automatable(record_type) {
            return Ok(Err(FieldViolation::new(
                "record_type",
                "must be a record type that supports automations",
            )));
        }
        match self.gateway.fields(self.scope, record_type).await {
            Ok(fields) => Ok(Ok(fields)),
            Err(AppError::NotFound(_) | AppError::Coded { .. }) => Ok(Err(FieldViolation::new(
                "record_type",
                "is not available in this organization",
            ))),
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn validate(
        &self,
        record_type: &str,
        trigger: &TriggerSpec,
        definition: &AutomationDefinition,
        level: ValidationLevel,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let fields = match self.fields(record_type).await? {
            Ok(fields) => fields,
            Err(violation) => return Ok(vec![violation]),
        };
        let mut violations = self
            .trigger_violations(record_type, trigger, &fields)
            .await?;
        let size = serde_json::to_vec(definition)
            .map(|bytes| bytes.len())
            .unwrap_or(0);
        if size > MAX_DEFINITION_BYTES {
            violations.push(FieldViolation::new(
                "definition",
                format!("must have at most {MAX_DEFINITION_BYTES} bytes"),
            ));
            return Ok(violations);
        }
        let structure = definition.structure_violations();
        if !structure.is_empty() {
            violations.extend(structure);
            return Ok(violations);
        }
        if level == ValidationLevel::Publish && definition.actions().next().is_none() {
            violations.push(FieldViolation::new(
                "definition.steps",
                "must have at least one action to be published",
            ));
        }
        violations.extend(wait_violations(definition, &fields));
        for (path, filter) in definition.filters() {
            violations.extend(trigger_violations(filter, &path));
            if let Some(record_filter) = record_part(filter) {
                violations.extend(
                    self.filter_violations(record_type, &path, &record_filter)
                        .await?,
                );
            }
        }
        let rules = TemplateRules {
            record_fields: fields
                .active()
                .map(|field| field.key().to_owned())
                .collect(),
            custom_fields: fields
                .active_custom()
                .map(|field| field.key().to_owned())
                .collect(),
        };
        for (index, _step, key, config) in definition.actions() {
            let path = format!("definition.steps.{index}.config");
            let Some(action) = self.actions.get(key) else {
                violations.push(FieldViolation::new(
                    format!("definition.steps.{index}.action"),
                    "is not a known action",
                ));
                continue;
            };
            if !action.supports(record_type) {
                violations.push(FieldViolation::new(
                    format!("definition.steps.{index}.action"),
                    format!("cannot be used on {record_type} records"),
                ));
                continue;
            }
            for (field, message) in rules.violations(config, &path) {
                violations.push(FieldViolation::new(field, message));
            }
            let context = ValidationContext {
                scope: self.scope,
                gateway: self.gateway,
                record_type,
                fields: &fields,
                path: path.clone(),
            };
            violations.extend(action.validate(&context, config).await?);
            if level == ValidationLevel::Publish {
                for permission in action.required_permissions(record_type, config) {
                    if self.scope.require(&permission).is_err() {
                        violations.push(FieldViolation::new(
                            format!("definition.steps.{index}.action"),
                            format!(
                                "requires the permission {} that you do not have",
                                permission.as_str()
                            ),
                        ));
                    }
                }
            }
        }
        Ok(violations)
    }

    async fn filter_violations(
        &self,
        record_type: &str,
        path: &str,
        filter: &Filter,
    ) -> Result<Vec<FieldViolation>, AppError> {
        match self
            .gateway
            .validate_filter(self.scope, record_type, filter)
            .await
        {
            Ok(()) => Ok(Vec::new()),
            Err(AppError::InvalidFields { fields: list, .. })
            | Err(AppError::CodedFields { fields: list, .. }) => Ok(list
                .into_iter()
                .map(|violation| {
                    let rest = violation
                        .field
                        .trim_start_matches("filter")
                        .trim_start_matches('.')
                        .to_owned();
                    let field = if rest.is_empty() {
                        path.to_owned()
                    } else {
                        format!("{path}.{rest}")
                    };
                    FieldViolation::new(field, violation.message)
                })
                .collect()),
            Err(AppError::Validation(message)) => Ok(vec![FieldViolation::new(path, message)]),
            Err(error) => Err(error),
        }
    }

    async fn trigger_violations(
        &self,
        record_type: &str,
        trigger: &TriggerSpec,
        fields: &FieldSet,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let Some(definition) = self.triggers.get(&trigger.kind) else {
            return Ok(vec![FieldViolation::new(
                "trigger.type",
                "is not a known trigger",
            )]);
        };
        if !definition.supports(record_type) {
            return Ok(vec![FieldViolation::new(
                "trigger.type",
                format!("cannot be used on {record_type} records"),
            )]);
        }
        let mut violations = definition.validate(trigger, fields);
        for (path, filter) in definition.record_filters(trigger) {
            if violations
                .iter()
                .any(|violation| violation.field.starts_with(&path))
            {
                continue;
            }
            if record_part(&filter).as_ref() != Some(&filter) {
                violations.push(FieldViolation::new(
                    path,
                    "trigger.* fields cannot be used in this filter",
                ));
                continue;
            }
            violations.extend(self.filter_violations(record_type, &path, &filter).await?);
        }
        for reference in definition.references(trigger) {
            let (path, exists) = match &reference {
                TriggerReference::Stage { path, id } => (
                    path,
                    self.gateway
                        .reference_exists(self.scope, Reference::new(refs::STAGE, id))
                        .await?,
                ),
                TriggerReference::IncomingWebhook { path, id } => (
                    path,
                    self.gateway
                        .reference_exists(
                            self.scope,
                            Reference::new(aphel_contract_webhooks::INCOMING_WEBHOOK, id),
                        )
                        .await?,
                ),
            };
            if !exists {
                violations.push(FieldViolation::new(path.clone(), "was not found"));
            }
        }
        Ok(violations)
    }
}
