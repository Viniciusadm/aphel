use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use chrono_tz::Tz;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::application::fields::formulas::{AggregateColumn, AggregateFunction, AggregateSource};
use crate::application::fields::{
    CustomFieldValueRepository, FieldDefinitionRepository, RecordTypeDefinition,
    RecordTypeRegistry, ReferenceChecker, SystemField,
};
use crate::application::shared::{JobQueue, RecordType, TransactionManager};
use crate::domain::access::AccessScope;
use crate::domain::fields::FieldValue;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateQuery<'a> {
    pub source: &'a AggregateSource,
    pub function: AggregateFunction,
    pub column: Option<&'a AggregateColumn>,
}

#[async_trait]
pub trait FormulaSource: Send + Sync {
    async fn timezone(&self) -> Result<Tz, AppError>;
    async fn formula_record_types(&self) -> Result<Vec<RecordType>, AppError>;
    async fn system_values(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        fields: &[SystemField],
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, HashMap<String, FieldValue>>, AppError>;
    async fn aggregate(
        &self,
        scope: &AccessScope,
        query: &AggregateQuery<'_>,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Decimal>, AppError>;
    async fn parents(
        &self,
        source: &AggregateSource,
        child_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError>;
    async fn record_ids(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        after: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<Uuid>, AppError>;
    async fn mark_daily_run(&self, local_date: NaiveDate) -> Result<bool, AppError>;
}

pub type FormulaTx<P> = <<P as FormulaPorts>::Transactions as TransactionManager>::Tx;

pub trait FormulaPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Definitions: FieldDefinitionRepository;
    type Values: CustomFieldValueRepository<Tx = FormulaTx<Self>>;
    type References: ReferenceChecker;
    type Source: FormulaSource;
    type Jobs: JobQueue<FormulaTx<Self>>;

    fn registry(&self) -> &RecordTypeRegistry;
    fn transactions(&self) -> &Self::Transactions;
    fn definitions(&self) -> &Self::Definitions;
    fn values(&self) -> &Self::Values;
    fn references(&self) -> &Self::References;
    fn source(&self) -> &Self::Source;
    fn jobs(&self) -> &Self::Jobs;
}
