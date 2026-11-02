use async_trait::async_trait;
use chrono_tz::Tz;
use uuid::Uuid;

use crate::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use crate::application::organization_admin::Listing;
use crate::application::query::{FilterSource, QueryPlan};
use crate::application::security::Clock;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[async_trait]
pub trait RecordQueryExecutor: Send + Sync {
    async fn timezone(&self) -> Result<Tz, AppError>;
    async fn execute(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
    ) -> Result<Listing<Uuid>, AppError>;
    async fn filter_source(&self, _filter_id: Uuid) -> Result<Option<FilterSource>, AppError> {
        Ok(None)
    }
}

pub trait RecordQueryPorts: Send + Sync {
    type Definitions: FieldDefinitionRepository;
    type Executor: RecordQueryExecutor;
    type Clock: Clock;

    fn registry(&self) -> &RecordTypeRegistry;
    fn definitions(&self) -> &Self::Definitions;
    fn executor(&self) -> &Self::Executor;
    fn clock(&self) -> &Self::Clock;
}
