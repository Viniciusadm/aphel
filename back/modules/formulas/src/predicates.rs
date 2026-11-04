use crate::application::{FormulaEngine, TypeFormulas, ValueKind, compile_formula};
use crate::infrastructure::MySqlFormulaPorts;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::fields::{FieldDefinition, FieldValue};
use aphel_sdk::access::AccessScope;
use aphel_sdk::fields::RecordTypeRegistry;
use aphel_sdk::{AppError, formulas::FormulaPredicates};
use sqlx::MySqlPool;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) struct Predicates;
fn expression(field: &FieldDefinition) -> Result<&str, AppError> {
    field
        .config
        .formula
        .as_ref()
        .map(|formula| formula.expression.as_str())
        .ok_or_else(|| AppError::Validation("a formula predicate requires an expression".into()))
}
#[async_trait::async_trait]
impl FormulaPredicates for Predicates {
    async fn violations(
        &self,
        pool: &MySqlPool,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        field: FieldDefinition,
    ) -> Result<Vec<String>, AppError> {
        let ports = MySqlFormulaPorts::new(pool.clone(), registry.clone());
        let engine = FormulaEngine::new(&ports, scope);
        let mut workspace = engine.workspace().with_override(field.clone());
        let context = workspace
            .context(&RecordType::parse(field.record_type.clone())?)
            .await?;
        match compile_formula(expression(&field)?, &context) {
            Err(errors) => Ok(errors.into_iter().map(|error| error.describe()).collect()),
            Ok(compiled) if compiled.kind != Some(ValueKind::Bool) => {
                Ok(vec!["must result in a boolean (true or false)".into()])
            }
            Ok(_) => Ok(Vec::new()),
        }
    }
    async fn matches(
        &self,
        pool: &MySqlPool,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        field: FieldDefinition,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        let ports = MySqlFormulaPorts::new(pool.clone(), registry.clone());
        let engine = FormulaEngine::new(&ports, scope);
        let mut workspace = engine.workspace().with_override(field.clone());
        let context = workspace
            .context(&RecordType::parse(field.record_type.clone())?)
            .await?;
        if compile_formula(expression(&field)?, &context).is_err() {
            return Ok(false);
        }
        let formulas = TypeFormulas::new(context);
        let mut evaluated = engine.evaluate(&formulas, &[record_id]).await?;
        let Some(record) = evaluated.remove(&record_id) else {
            return Ok(false);
        };
        Ok(record
            .results
            .iter()
            .find(|(candidate, _)| candidate.key == field.key)
            .is_some_and(|(_, value)| matches!(value, Some(FieldValue::Bool(true)))))
    }
}
