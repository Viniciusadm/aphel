use serde_json::Value;
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_values};
use crate::application::config_packages::{is_config_key, key_from_name};
use crate::application::fields::formulas::MAX_REPORTED_ERRORS;
use crate::application::fields::formulas::{FormulaContext, FormulaWorkspace};
use crate::application::fields::validation_rules::{
    CreateValidationRuleCommand, MAX_RULE_MESSAGE_LENGTH, MAX_RULE_NAME_LENGTH,
    MAX_RULES_PER_RECORD_TYPE, ReorderValidationRulesCommand, RuleTiming,
    UpdateValidationRuleCommand, VALIDATION_RULE_CREATED, VALIDATION_RULE_DELETED,
    VALIDATION_RULE_NOT_FOUND, VALIDATION_RULE_RECORD, VALIDATION_RULE_UPDATED,
    VALIDATION_RULES_REORDERED, ValidationRule, ValidationRulePorts, ValidationRuleRepository,
    compile_rule, is_validated_record_type,
};
use crate::application::security::Clock;
use crate::application::shared::{
    FieldChange, RecordRef, RecordType, TransactionManager, ensure_version,
};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::fields::MAX_FORMULA_LENGTH;
use crate::errors::{AppError, FieldViolation};
use crate::records::{RecordMutation, changes_payload, publish_mutation};

const INVALID_VALIDATION_RULE: &str = "invalid validation rule";

fn rule_not_found() -> AppError {
    AppError::not_found_code(VALIDATION_RULE_NOT_FOUND, "validation rule was not found")
}

fn parse_rule_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| rule_not_found())
}

pub struct ValidationRuleUseCases<P> {
    ports: P,
}

impl<P> ValidationRuleUseCases<P>
where
    P: ValidationRulePorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list(
        &self,
        scope: &AccessScope,
        record_type: Option<&str>,
    ) -> Result<Vec<ValidationRule>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let record_type = record_type.map(str::trim).filter(|value| !value.is_empty());
        self.ports.rules().list(record_type).await
    }

    pub async fn get(&self, scope: &AccessScope, id: &str) -> Result<ValidationRule, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.require(id).await
    }

    pub async fn create(
        &self,
        scope: &AccessScope,
        command: CreateValidationRuleCommand,
    ) -> Result<ValidationRule, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let record_type = command.record_type.trim().to_owned();
        if !is_validated_record_type(&record_type) {
            return Err(AppError::invalid_fields(
                INVALID_VALIDATION_RULE,
                vec![FieldViolation::new(
                    "record_type",
                    "does not support validation rules",
                )],
            ));
        }
        let definition = self.ports.registry().require(&record_type)?;
        let record_type = definition.record_type.clone();
        let existing = self.ports.rules().list(Some(record_type.as_str())).await?;
        let mut violations = Vec::new();
        if existing.len() >= MAX_RULES_PER_RECORD_TYPE {
            violations.push(FieldViolation::new(
                "record_type",
                format!("can have at most {MAX_RULES_PER_RECORD_TYPE} validation rules"),
            ));
        }
        let key = match command.key.as_deref().map(str::trim) {
            Some(key) if !key.is_empty() => key.to_owned(),
            _ => self.free_key(&key_from_name(&command.name)).await?,
        };
        if !is_config_key(&key) {
            violations.push(FieldViolation::new(
                "key",
                "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
            ));
        } else if self.ports.rules().find_by_key(&key).await?.is_some() {
            violations.push(FieldViolation::new(
                "key",
                "is already used by another rule",
            ));
        }
        let applies_on = match command.applies_on.as_deref().map(str::trim) {
            None | Some("") => RuleTiming::Always,
            Some(value) => RuleTiming::parse(value).unwrap_or_else(|| {
                violations.push(FieldViolation::new(
                    "applies_on",
                    "must be always, create or update",
                ));
                RuleTiming::Always
            }),
        };
        let now = self.ports.clock().now();
        let rule = ValidationRule {
            id: Uuid::now_v7(),
            key,
            record_type: record_type.as_str().to_owned(),
            name: command.name.trim().to_owned(),
            expression: command.expression.trim().to_owned(),
            message: command.message.trim().to_owned(),
            field: command
                .field
                .map(|field| field.trim().to_owned())
                .filter(|field| !field.is_empty()),
            applies_on,
            is_active: command.is_active.unwrap_or(true),
            position: existing
                .iter()
                .map(|rule| rule.position)
                .max()
                .map_or(0, |position| position + 1),
            created_by: Some(scope.user_id().clone()),
            updated_by: Some(scope.user_id().clone()),
            created_at: now,
            updated_at: now,
        };
        let context = self.context(&record_type).await?;
        violations.extend(rule_violations(&rule, &context));
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                INVALID_VALIDATION_RULE,
                violations,
            ));
        }
        let changes = diff_values(&Value::Null, &rule.snapshot(), &DiffOptions::default())?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.rules().insert(&mut tx, &rule).await?;
        self.publish(
            &mut tx,
            scope,
            &rule,
            VALIDATION_RULE_CREATED,
            changes,
            command.ip,
        )
        .await?;
        transactions.commit(tx).await?;
        Ok(rule)
    }

    pub async fn update(
        &self,
        scope: &AccessScope,
        id: &str,
        command: UpdateValidationRuleCommand,
    ) -> Result<ValidationRule, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.require(id).await?;
        ensure_version(command.expected_updated_at, current.updated_at)?;
        let mut rule = current.clone();
        let mut violations = Vec::new();
        if let Some(name) = command.name {
            rule.name = name.trim().to_owned();
        }
        if let Some(expression) = command.expression {
            rule.expression = expression.trim().to_owned();
        }
        if let Some(message) = command.message {
            rule.message = message.trim().to_owned();
        }
        if let Some(field) = command.field {
            rule.field = field
                .map(|field| field.trim().to_owned())
                .filter(|field| !field.is_empty());
        }
        if let Some(applies_on) = command.applies_on {
            match RuleTiming::parse(applies_on.trim()) {
                Some(timing) => rule.applies_on = timing,
                None => violations.push(FieldViolation::new(
                    "applies_on",
                    "must be always, create or update",
                )),
            }
        }
        if let Some(is_active) = command.is_active {
            rule.is_active = is_active;
        }
        let record_type = RecordType::parse(rule.record_type.clone())?;
        let context = self.context(&record_type).await?;
        violations.extend(rule_violations(&rule, &context));
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                INVALID_VALIDATION_RULE,
                violations,
            ));
        }
        let changes = diff_values(
            &current.snapshot(),
            &rule.snapshot(),
            &DiffOptions::default(),
        )?;
        if changes.is_empty() {
            return Ok(current);
        }
        rule.updated_by = Some(scope.user_id().clone());
        rule.updated_at = self.ports.clock().now();
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.rules().update(&mut tx, &rule).await?;
        self.publish(
            &mut tx,
            scope,
            &rule,
            VALIDATION_RULE_UPDATED,
            changes,
            command.ip,
        )
        .await?;
        transactions.commit(tx).await?;
        Ok(rule)
    }

    pub async fn delete(
        &self,
        scope: &AccessScope,
        id: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let rule = self.require(id).await?;
        let changes = diff_values(&rule.snapshot(), &Value::Null, &DiffOptions::default())?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.rules().delete(&mut tx, rule.id).await?;
        self.publish(&mut tx, scope, &rule, VALIDATION_RULE_DELETED, changes, ip)
            .await?;
        transactions.commit(tx).await
    }

    pub async fn reorder(
        &self,
        scope: &AccessScope,
        command: ReorderValidationRulesCommand,
    ) -> Result<Vec<ValidationRule>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let definition = self.ports.registry().require(command.record_type.trim())?;
        let record_type = definition.record_type.as_str().to_owned();
        let existing = self.ports.rules().list(Some(&record_type)).await?;
        let mut ordered: Vec<Uuid> = Vec::with_capacity(command.ids.len());
        for value in &command.ids {
            let id = Uuid::parse_str(value.trim()).ok();
            match id {
                Some(id) if existing.iter().any(|rule| rule.id == id) && !ordered.contains(&id) => {
                    ordered.push(id)
                }
                _ => {
                    return Err(AppError::invalid_fields(
                        INVALID_VALIDATION_RULE,
                        vec![FieldViolation::new(
                            "ids",
                            "must list the rules of the record type once each",
                        )],
                    ));
                }
            }
        }
        if ordered.len() != existing.len() {
            return Err(AppError::invalid_fields(
                INVALID_VALIDATION_RULE,
                vec![FieldViolation::new(
                    "ids",
                    "must list every rule of the record type",
                )],
            ));
        }
        let positions: Vec<(Uuid, i32)> = ordered
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, i32::try_from(index).unwrap_or(i32::MAX)))
            .collect();
        let unchanged = positions.iter().all(|(id, position)| {
            existing
                .iter()
                .any(|rule| rule.id == *id && rule.position == *position)
        });
        if unchanged {
            return Ok(existing);
        }
        let changes: Vec<FieldChange> = positions
            .iter()
            .filter_map(|(id, position)| {
                let rule = existing.iter().find(|rule| rule.id == *id)?;
                (rule.position != *position).then(|| FieldChange {
                    field: format!("{}.position", rule.key),
                    old: Value::from(rule.position),
                    new: Value::from(*position),
                })
            })
            .collect();
        let now = self.ports.clock().now();
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .rules()
            .set_positions(&mut tx, &positions)
            .await?;
        let mut payload = changes_payload(&changes);
        payload["record_type"] = Value::from(record_type.clone());
        publish_mutation(
            self.ports.events(),
            self.ports.audit(),
            &mut tx,
            scope,
            now,
            RecordMutation {
                record: RecordRef::new(VALIDATION_RULE_RECORD, Uuid::nil()),
                event_type: VALIDATION_RULES_REORDERED,
                payload,
                changes,
                ip: command.ip,
            },
        )
        .await?;
        transactions.commit(tx).await?;
        self.ports.rules().list(Some(&record_type)).await
    }

    async fn publish(
        &self,
        tx: &mut <P::Transactions as TransactionManager>::Tx,
        scope: &AccessScope,
        rule: &ValidationRule,
        event_type: &'static str,
        changes: Vec<FieldChange>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let mut payload = changes_payload(&changes);
        payload["rule"] = rule.summary();
        publish_mutation(
            self.ports.events(),
            self.ports.audit(),
            tx,
            scope,
            self.ports.clock().now(),
            RecordMutation {
                record: RecordRef::new(VALIDATION_RULE_RECORD, rule.id),
                event_type,
                payload,
                changes,
                ip,
            },
        )
        .await
    }

    async fn context(&self, record_type: &RecordType) -> Result<FormulaContext, AppError> {
        FormulaWorkspace::new(self.ports.registry(), self.ports.definitions())
            .context(record_type)
            .await
    }

    async fn free_key(&self, base: &str) -> Result<String, AppError> {
        let base: String = base.chars().take(56).collect();
        let base = base.trim_end_matches('_').to_owned();
        let mut candidate = base.clone();
        let mut suffix = 2;
        while self.ports.rules().find_by_key(&candidate).await?.is_some() {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        Ok(candidate)
    }

    async fn require(&self, id: &str) -> Result<ValidationRule, AppError> {
        let id = parse_rule_id(id)?;
        self.ports
            .rules()
            .find(id)
            .await?
            .ok_or_else(rule_not_found)
    }
}

pub fn rule_violations(rule: &ValidationRule, context: &FormulaContext) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    let name_length = rule.name.chars().count();
    if name_length == 0 || name_length > MAX_RULE_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_RULE_NAME_LENGTH} characters"),
        ));
    }
    let message_length = rule.message.chars().count();
    if message_length == 0 || message_length > MAX_RULE_MESSAGE_LENGTH {
        violations.push(FieldViolation::new(
            "message",
            format!("must have between 1 and {MAX_RULE_MESSAGE_LENGTH} characters"),
        ));
    }
    if rule.expression.chars().count() > MAX_FORMULA_LENGTH {
        violations.push(FieldViolation::new(
            "expression",
            format!("must have at most {MAX_FORMULA_LENGTH} characters"),
        ));
    } else if let Err(errors) = compile_rule(&rule.expression, context) {
        violations.extend(
            errors
                .iter()
                .take(MAX_REPORTED_ERRORS)
                .map(|error| FieldViolation::new("expression", error.describe())),
        );
    }
    if let Some(field) = &rule.field
        && context.field(field).is_none()
    {
        violations.push(FieldViolation::new(
            "field",
            "must be an active field of the record type",
        ));
    }
    violations
}
