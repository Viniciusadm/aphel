use aphel_contract_crm::CrmRecordTypes;
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    DistributionMode, DistributionOutcome, DistributionRequest, NO_ELIGIBLE_MEMBER, RULE_INACTIVE,
    SkipCause, flatten_values,
};
use aphel_kernel::application::automations::actions::{
    ActionContext, AutomationAction, ValidationContext, only_keys, record_permission, text,
};
use aphel_kernel::application::automations::triggers::is_automatable;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const ASSIGN_BY_RULE: &str = "assign_by_rule";
pub(crate) const DEFAULT_FIELD: &str = "owner_id";

pub(crate) struct AssignByRule;

impl AssignByRule {
    fn field(config: &Value) -> String {
        text(config, "field").unwrap_or(DEFAULT_FIELD).to_owned()
    }

    fn request(
        context: &ActionContext<'_>,
        config: &Value,
        mode: DistributionMode,
    ) -> Result<DistributionRequest, AppError> {
        Ok(DistributionRequest {
            rule_id: text(config, "rule_id")
                .ok_or_else(|| AppError::Validation("rule_id is required".to_owned()))?
                .to_owned(),
            record: context.record.clone(),
            field: Self::field(config),
            values: flatten_values(context.record_data),
            mode,
        })
    }

    fn output(config: &Value, outcome: &DistributionOutcome) -> Value {
        json!({
            "rule_id": text(config, "rule_id"),
            "field": Self::field(config),
            "user_id": outcome.user_id,
            "user_name": outcome.user_name,
            "selection": outcome.selection.map(|selection| selection.as_str()),
            "route_index": outcome.selection.and_then(|selection| selection.route_index()),
            "reused": outcome.reused,
        })
    }
}

#[async_trait]
impl AutomationAction for AssignByRule {
    fn key(&self) -> &'static str {
        ASSIGN_BY_RULE
    }
    fn label(&self) -> &'static str {
        "Distribuir pela regra"
    }
    fn description(&self) -> &'static str {
        "Assigns the record to the next user of a lead distribution rule (round robin, weighted, least loaded or routed by a field), skipping away and inactive users; `field` is the user field to set (default `owner_id`)"
    }
    fn config_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "rule_id": { "type": "string", "format": "uuid", "x-reference": "lead_distribution_rule" },
                "field": { "type": "string", "default": DEFAULT_FIELD, "x-field-types": ["user"] },
            },
            "required": ["rule_id"],
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
        if !only_keys(context, config, &["rule_id", "field"], &mut violations) {
            return Ok(violations);
        }
        let field = Self::field(config);
        match context.fields.find(&field) {
            Some(_) if field == DEFAULT_FIELD => {}
            Some(definition) if definition.definition.field_type.base_type() == FieldType::User => {
            }
            Some(_) => violations.push(context.violation("field", "must be a user field")),
            None if field == DEFAULT_FIELD => {
                violations.push(context.violation("", "this record type has no owner"))
            }
            None => {
                violations.push(context.violation("field", "is not a field of this record type"))
            }
        }
        let Some(rule_id) = text(config, "rule_id") else {
            violations.push(context.violation("rule_id", "is required"));
            return Ok(violations);
        };
        let Ok(record) = RecordRef::parse(context.record_type, &Uuid::nil().to_string()) else {
            violations.push(context.violation("", "this record type cannot be distributed"));
            return Ok(violations);
        };
        let outcome = crate::service::distribute(
            &aphel_kernel::Ctx::from_state(context.gateway.state(), context.scope).await?,
            context.gateway,
            DistributionRequest {
                rule_id: rule_id.to_owned(),
                record,
                field,
                values: Map::new(),
                mode: DistributionMode::Check,
            },
        )
        .await?;
        match outcome.rule {
            None => violations.push(context.violation("rule_id", "was not found")),
            Some(rule) if rule.record_type != context.record_type => {
                violations.push(context.violation(
                    "rule_id",
                    format!(
                        "distributes {} records, not {}",
                        rule.record_type, context.record_type
                    ),
                ))
            }
            Some(rule) if !rule.is_active => {
                violations.push(context.violation("rule_id", "is not active"))
            }
            Some(_) => {}
        }
        Ok(violations)
    }
    async fn describe(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let outcome = crate::service::distribute(
            &aphel_kernel::Ctx::from_state(context.gateway.state(), context.scope).await?,
            context.gateway,
            Self::request(context, config, DistributionMode::Preview)?,
        )
        .await?;
        Ok(Self::output(config, &outcome))
    }
    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let idempotency_key = format!("automation:{}:{}", context.run_id, context.step_id);
        let outcome = crate::service::distribute(
            &aphel_kernel::Ctx::from_state(context.gateway.state(), context.scope).await?,
            context.gateway,
            Self::request(
                context,
                config,
                DistributionMode::Assign { idempotency_key },
            )?,
        )
        .await?;
        match outcome.skipped {
            Some(SkipCause::RuleInactive) => Err(AppError::validation_code(
                RULE_INACTIVE,
                "the lead distribution rule is not active",
            )),
            Some(SkipCause::NoEligibleMember) => Err(AppError::validation_code(
                NO_ELIGIBLE_MEMBER,
                "no member of the lead distribution rule is available",
            )),
            Some(cause) => Ok(json!({
                "rule_id": text(config, "rule_id"),
                "user_id": Value::Null,
                "skipped": cause.as_str(),
            })),
            None => Ok(Self::output(config, &outcome)),
        }
    }
}
