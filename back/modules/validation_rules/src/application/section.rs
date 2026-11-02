use ::config_packages::application::ConfigReferenceKind;
use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::{
    MAX_RULE_MESSAGE_LENGTH, MAX_RULE_NAME_LENGTH, RuleTiming, ValidationRule,
    ValidationRuleRepository, is_validated_record_type,
};
use ::config_packages::application::sections::common::{check_key, diff};
use ::config_packages::application::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::MAX_FORMULA_LENGTH;
use aphel_kernel::errors::AppError;

pub const VALIDATION_RULES_SECTION: &str = "validation_rules";
pub const VALIDATION_RULE_ITEM: &str = "validation_rule";

fn default_applies_on() -> String {
    RuleTiming::Always.as_str().to_owned()
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageValidationRule {
    pub key: String,
    pub record_type: String,
    pub name: String,
    pub expression: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default = "default_applies_on")]
    pub applies_on: String,
    #[serde(default = "default_active")]
    pub active: bool,
}

struct RulePlan {
    item: PlanItem,
    rule: ValidationRule,
    is_new: bool,
    conflicting: bool,
}

fn snapshot(rule: &ValidationRule) -> Vec<(String, Value)> {
    vec![
        ("record_type".to_owned(), json!(rule.record_type)),
        ("name".to_owned(), json!(rule.name)),
        ("expression".to_owned(), json!(rule.expression)),
        ("message".to_owned(), json!(rule.message)),
        ("field".to_owned(), json!(rule.field)),
        ("applies_on".to_owned(), json!(rule.applies_on.as_str())),
        ("active".to_owned(), json!(rule.is_active)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageValidationRule>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

pub struct ValidationRulesSection<R> {
    rules: R,
}

impl<R> ValidationRulesSection<R>
where
    R: ValidationRuleRepository,
{
    pub fn new(rules: R) -> Self {
        Self { rules }
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &[PackageValidationRule],
        at: DateTime<Utc>,
    ) -> Result<Vec<RulePlan>, AppError> {
        let existing = self.rules.list(None).await?;
        let mut position = existing
            .iter()
            .map(|rule| rule.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in package {
            let item = PlanItem::new(
                VALIDATION_RULES_SECTION,
                VALIDATION_RULE_ITEM,
                entry.key.clone(),
                entry.name.trim(),
            );
            let field = entry
                .field
                .as_deref()
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .map(str::to_owned);
            let applies_on = RuleTiming::parse(entry.applies_on.trim()).unwrap_or_default();
            match existing.iter().find(|current| current.key == entry.key) {
                None => {
                    position += 1;
                    let rule = ValidationRule {
                        id: Uuid::now_v7(),
                        key: entry.key.clone(),
                        record_type: entry.record_type.trim().to_owned(),
                        name: entry.name.trim().to_owned(),
                        expression: entry.expression.trim().to_owned(),
                        message: entry.message.trim().to_owned(),
                        field,
                        applies_on,
                        is_active: entry.active,
                        position,
                        created_by: Some(scope.user_id().clone()),
                        updated_by: Some(scope.user_id().clone()),
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = snapshot(&rule)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(RulePlan {
                        item: item.created(changes),
                        rule,
                        is_new: true,
                        conflicting: false,
                    });
                }
                Some(current) => {
                    let updated = ValidationRule {
                        name: entry.name.trim().to_owned(),
                        expression: entry.expression.trim().to_owned(),
                        message: entry.message.trim().to_owned(),
                        field,
                        applies_on,
                        is_active: entry.active,
                        updated_by: Some(scope.user_id().clone()),
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    let conflicting = current.record_type != entry.record_type.trim();
                    let item = if conflicting {
                        item.conflicting(
                            changes,
                            "record_type_mismatch",
                            format!(
                                "the rule {} already exists for the record type {}",
                                current.key, current.record_type
                            ),
                            false,
                        )
                    } else {
                        item.compared(changes)
                    };
                    plans.push(RulePlan {
                        item,
                        rule: updated,
                        is_new: false,
                        conflicting,
                    });
                }
            }
        }
        Ok(plans)
    }
}

#[async_trait]
impl<R, Tx> ConfigSection<Tx> for ValidationRulesSection<R>
where
    Tx: Send,
    R: ValidationRuleRepository<Tx = Tx>,
{
    fn key(&self) -> &'static str {
        VALIDATION_RULES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let rules = self.rules.list(None).await?;
        if rules.is_empty() {
            return Ok(None);
        }
        let entries: Vec<PackageValidationRule> = rules
            .into_iter()
            .map(|rule| PackageValidationRule {
                key: rule.key,
                record_type: rule.record_type,
                name: rule.name,
                expression: rule.expression,
                message: rule.message,
                field: rule.field,
                applies_on: rule.applies_on.as_str().to_owned(),
                active: rule.is_active,
            })
            .collect();
        Ok(Some(json!(entries)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let rules: Vec<PackageValidationRule> = match parse_section(content) {
            Ok(rules) => rules,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        let mut keys = BTreeSet::new();
        for (index, rule) in rules.iter().enumerate() {
            let path = |field: &str| format!("{index}.{field}");
            check_key(&rule.key, &path("key"), &mut keys, &mut issues);
            if !is_validated_record_type(rule.record_type.trim()) {
                issues.push(PackageIssue::new(
                    path("record_type"),
                    "invalid_record_type",
                    "does not support validation rules",
                ));
            }
            let name = rule.name.trim().chars().count();
            if name == 0 || name > MAX_RULE_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    path("name"),
                    "invalid_name",
                    format!("must have between 1 and {MAX_RULE_NAME_LENGTH} characters"),
                ));
            }
            let message = rule.message.trim().chars().count();
            if message == 0 || message > MAX_RULE_MESSAGE_LENGTH {
                issues.push(PackageIssue::new(
                    path("message"),
                    "invalid_message",
                    format!("must have between 1 and {MAX_RULE_MESSAGE_LENGTH} characters"),
                ));
            }
            let expression = rule.expression.trim().chars().count();
            if expression == 0 || expression > MAX_FORMULA_LENGTH {
                issues.push(PackageIssue::new(
                    path("expression"),
                    "invalid_expression",
                    format!("must have between 1 and {MAX_FORMULA_LENGTH} characters"),
                ));
            }
            if RuleTiming::parse(rule.applies_on.trim()).is_none() {
                issues.push(PackageIssue::new(
                    path("applies_on"),
                    "invalid_applies_on",
                    "must be always, create or update",
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(scope, &parse(content)?, DateTime::<Utc>::UNIX_EPOCH)
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for plan in self.reconcile(scope, &parse(content)?, context.at).await? {
            if plan.is_new {
                self.rules.insert(tx, &plan.rule).await?;
            } else if !plan.conflicting && decisions.executes(&plan.item) {
                self.rules.update(tx, &plan.rule).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("validation_rules.rule"),
                &plan.rule.key,
                plan.rule.id,
            );
        }
        Ok(())
    }
}
