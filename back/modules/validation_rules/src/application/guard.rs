use crate::application::{
    ProposedFieldValues, VALIDATION_RULE_FAILED, ValidationRuleRepository, is_validated_record_type,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::formulas::{PredicateInput, ProspectiveRequest, prospective};
use aphel_sdk::platform::application::fields::{RecordTypeRegistry, RecordWrite};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct ValidationRuleGuard<R, V> {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
    rules: R,
    values: V,
}
impl<R, V> ValidationRuleGuard<R, V> {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>, rules: R, values: V) -> Self {
        Self {
            pool,
            registry,
            rules,
            values,
        }
    }
}
impl<R, V> ValidationRuleGuard<R, V>
where
    R: ValidationRuleRepository,
    V: ProposedFieldValues<Tx = MySqlTx>,
{
    pub async fn check(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        write: &RecordWrite<'_>,
    ) -> Result<(), AppError> {
        let record_type = &write.record.record_type;
        if !is_validated_record_type(record_type.as_str()) {
            return Ok(());
        }
        let rules: Vec<_> = self
            .rules
            .active_for(record_type.as_str())
            .await?
            .into_iter()
            .filter(|rule| rule.applies_on.applies(write.mode))
            .collect();
        if rules.is_empty() || self.registry.get(record_type).is_none() {
            return Ok(());
        }
        let predicates: Vec<_> = rules
            .iter()
            .map(|rule| PredicateInput {
                id: rule.id,
                expression: rule.expression.clone(),
                field: rule.field.clone(),
            })
            .collect();
        let outcomes = prospective()?
            .evaluate(
                ProspectiveRequest {
                    pool: &self.pool,
                    registry: &self.registry,
                    scope,
                    write,
                    predicates: &predicates,
                },
                tx,
                &self.values,
            )
            .await?;
        let mut violations = Vec::new();
        for rule in &rules {
            let Some(outcome) = outcomes.iter().find(|outcome| outcome.id == rule.id) else {
                continue;
            };
            match outcome.matched {
                None => {
                    tracing::warn!(rule = %rule.key, record_type = %record_type.as_str(), error = ?outcome.error, "validation rule skipped because its condition no longer compiles")
                }
                Some(true) => {
                    violations.push(FieldViolation::new(&outcome.path, rule.message.clone()))
                }
                Some(false) => (),
            }
        }
        if violations.is_empty() {
            return Ok(());
        }
        Err(AppError::invalid_fields_code(
            VALIDATION_RULE_FAILED,
            "the record breaks validation rules",
            violations,
        ))
    }
}
