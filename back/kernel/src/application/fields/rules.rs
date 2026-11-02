use async_trait::async_trait;
use serde_json::{Map, Value};

use crate::application::fields::{FieldSet, WriteMode};
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};

#[async_trait]
pub trait ConditionalRules: Send + Sync {
    async fn violations(
        &self,
        scope: &AccessScope,
        fields: &FieldSet,
        values: &Map<String, Value>,
        mode: WriteMode,
    ) -> Result<Vec<FieldViolation>, AppError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoConditionalRules;

#[async_trait]
impl ConditionalRules for NoConditionalRules {
    async fn violations(
        &self,
        _scope: &AccessScope,
        _fields: &FieldSet,
        _values: &Map<String, Value>,
        _mode: WriteMode,
    ) -> Result<Vec<FieldViolation>, AppError> {
        Ok(Vec::new())
    }
}

pub trait ConditionalRulesFactory: Send + Sync {
    fn create(&self, pool: &sqlx::MySqlPool) -> std::sync::Arc<dyn ConditionalRules>;
}

pub struct ContributedConditionalRules(pub sqlx::MySqlPool);
#[async_trait]
impl ConditionalRules for ContributedConditionalRules {
    async fn violations(
        &self,
        scope: &AccessScope,
        fields: &FieldSet,
        values: &Map<String, Value>,
        mode: WriteMode,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        for factory in crate::modules::app().contributions::<dyn ConditionalRulesFactory>() {
            violations.extend(
                factory
                    .create(&self.0)
                    .violations(scope, fields, values, mode)
                    .await?,
            );
        }
        Ok(violations)
    }
}
