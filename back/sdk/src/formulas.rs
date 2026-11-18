use crate::AppError;
use crate::access::AccessScope;
use crate::fields::RecordTypeRegistry;
use aphel_kernel::database::Db;
use aphel_kernel::domain::fields::FieldDefinition;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait FormulaPredicates: Send + Sync {
    async fn violations(
        &self,
        pool: &Db,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        field: FieldDefinition,
    ) -> Result<Vec<String>, AppError>;
    async fn matches(
        &self,
        pool: &Db,
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

pub mod dto;
use aphel_kernel::application::fields::{FieldDefinitionReader, RecordWrite};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::Tx;
use aphel_kernel::domain::fields::FieldValue;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreviewFormulaCommand {
    pub record_type: String,
    pub expression: String,
    pub result_type: String,
    pub decimals: Option<u32>,
    pub record_id: Option<String>,
    pub field_key: Option<String>,
}

#[async_trait]
pub trait ProposedFieldValues: Send + Sync {
    type Tx: Send;

    async fn load(
        &self,
        tx: &mut Self::Tx,
        record: &RecordRef,
        fields: &[FieldDefinition],
    ) -> Result<HashMap<Uuid, FieldValue>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateInput {
    pub id: Uuid,
    pub expression: String,
    pub field: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateOutcome {
    pub id: Uuid,
    pub matched: Option<bool>,
    pub path: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateCheck {
    pub errors: Vec<String>,
    pub field_exists: bool,
}
#[derive(Debug)]
pub struct BooleanFormulaPreview {
    pub formula: dto::FormulaPreviewResponse,
    pub matched: Option<bool>,
}
pub struct ProspectiveRequest<'a, 'w> {
    pub pool: &'a Db,
    pub registry: &'a Arc<RecordTypeRegistry>,
    pub scope: &'a AccessScope,
    pub write: &'a RecordWrite<'w>,
    pub predicates: &'a [PredicateInput],
}
#[async_trait]
pub trait ProspectivePredicates: Send + Sync {
    async fn check(
        &self,
        registry: &RecordTypeRegistry,
        definitions: &dyn FieldDefinitionReader,
        record_type: &RecordType,
        expression: &str,
        field: Option<&str>,
    ) -> Result<PredicateCheck, AppError>;
    async fn evaluate(
        &self,
        request: ProspectiveRequest<'_, '_>,
        tx: &mut Tx,
        values: &dyn ProposedFieldValues<Tx = Tx>,
    ) -> Result<Vec<PredicateOutcome>, AppError>;
    async fn preview(
        &self,
        pool: &Db,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        command: PreviewFormulaCommand,
    ) -> Result<BooleanFormulaPreview, AppError>;
}
pub fn prospective() -> Result<Arc<dyn ProspectivePredicates>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn ProspectivePredicates>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "prospective_predicates_unavailable",
                "prospective formula evaluation is not enabled",
            )
        })
}
