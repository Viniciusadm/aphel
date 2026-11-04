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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDefinitionIssue {
    pub index: usize,
    pub property: String,
    pub code: String,
    pub message: String,
}
#[async_trait::async_trait]
pub trait FormulaDefinitionPlanner: Send + Sync {
    async fn projected_issues(
        &self,
        registry: &RecordTypeRegistry,
        definitions: &dyn aphel_kernel::application::fields::FieldDefinitionReader,
        projected: &[(usize, FieldDefinition)],
    ) -> Result<Vec<FormulaDefinitionIssue>, AppError>;
    fn recalculate_job(
        &self,
        record_type: &crate::shared::RecordType,
    ) -> aphel_kernel::application::shared::Job;
}
pub fn definition_planner() -> Result<std::sync::Arc<dyn FormulaDefinitionPlanner>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn FormulaDefinitionPlanner>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "formula_planner_unavailable",
                "formula definition planning is not enabled",
            )
        })
}
