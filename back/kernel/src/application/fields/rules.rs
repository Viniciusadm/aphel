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
