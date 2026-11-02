use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::contents::SearchContents;
use crate::application::document::{IndexDocument, IndexEntry, SourceRow};
use aphel_kernel::application::fields::{
    CustomFieldValueRepository, EntityDefinition, FieldDefinitionRepository, RecordTypeRegistry,
};
use aphel_kernel::application::query::{QueryPlan, RecordQueryExecutor};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{JobQueue, RecordType};
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::BoxFuture;

#[derive(Debug, Clone, PartialEq)]
pub struct IndexQuery<'a> {
    pub definition: &'a EntityDefinition,
    pub visibility: &'a RecordScopeFilter,
    pub field_keys: &'a [String],
    pub terms: &'a [String],
    pub limit: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexCandidate {
    pub record_id: Uuid,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexHeader {
    pub record_id: Uuid,
    pub title: String,
    pub title_field: Option<String>,
    pub subtitle: Option<String>,
    pub subtitle_field: Option<String>,
}

#[async_trait]
pub trait SearchIndexRepository: Send + Sync {
    async fn upsert(
        &self,
        document: &IndexDocument,
        indexed_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn remove(&self, record_type: &RecordType, record_ids: &[Uuid]) -> Result<(), AppError>;
    async fn remove_stale(
        &self,
        record_type: &RecordType,
        indexed_before: DateTime<Utc>,
    ) -> Result<u64, AppError>;
    async fn candidates(
        &self,
        scope: &AccessScope,
        query: &IndexQuery<'_>,
    ) -> Result<Vec<IndexCandidate>, AppError>;
    async fn headers(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<Vec<IndexHeader>, AppError>;
    async fn entries(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
        field_keys: &[String],
    ) -> Result<Vec<(Uuid, IndexEntry)>, AppError>;
}

#[async_trait]
pub trait SearchSource: Send + Sync {
    async fn load(
        &self,
        definition: &EntityDefinition,
        columns: &[(String, String)],
        record_ids: &[Uuid],
    ) -> Result<Vec<SourceRow>, AppError>;
    async fn scan(
        &self,
        definition: &EntityDefinition,
        columns: &[(String, String)],
        after: Option<Uuid>,
        limit: u64,
    ) -> Result<Vec<SourceRow>, AppError>;
}

pub trait SearchPorts: Send + Sync {
    type Tx: Send;
    type Index: SearchIndexRepository;
    type Source: SearchSource;
    type Definitions: FieldDefinitionRepository;
    type Values: CustomFieldValueRepository;
    type Executor: RecordQueryExecutor;
    type Jobs: JobQueue<Self::Tx>;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn index(&self) -> &Self::Index;
    fn source(&self) -> &Self::Source;
    fn definitions(&self) -> &Self::Definitions;
    fn values(&self) -> &Self::Values;
    fn executor(&self) -> &Self::Executor;
    fn restrict<'a>(
        &'a self,
        scope: &'a AccessScope,
        plan: QueryPlan,
    ) -> BoxFuture<'a, Result<QueryPlan, AppError>>;
    fn jobs(&self) -> &Self::Jobs;
    fn contents(&self) -> &SearchContents;
    fn clock(&self) -> &Self::Clock;
}
