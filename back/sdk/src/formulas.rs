use crate::AppError;
use crate::access::AccessScope;
use crate::fields::RecordTypeRegistry;
use aphel_kernel::domain::fields::FieldDefinition;
use sqlx::MySqlPool;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait FormulaPredicates: Send + Sync {
    async fn violations(
        &self,
        pool: &MySqlPool,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        field: FieldDefinition,
    ) -> Result<Vec<String>, AppError>;
    async fn matches(
        &self,
        pool: &MySqlPool,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        field: FieldDefinition,
        record_id: Uuid,
    ) -> Result<bool, AppError>;
}

pub fn predicates() -> Result<std::sync::Arc<dyn FormulaPredicates>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn FormulaPredicates>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "formula_predicates_unavailable",
                "formula predicates are not enabled",
            )
        })
}
