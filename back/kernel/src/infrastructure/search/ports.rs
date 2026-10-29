use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::custom_objects::{
    CatalogEventConsumer, CatalogJobHandler, RecordTypeCatalog,
};
use crate::application::fields::RecordTypeRegistry;
use crate::application::query::QueryPlan;
use crate::application::search::{
    SearchContents, SearchIndexConsumer, SearchPorts, SearchReindexHandler,
};
use crate::application::shared::{EventConsumer, JobHandler};
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::fields::{
    MySqlCustomFieldValueRepository, MySqlFieldDefinitionRepository,
};
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::query::MySqlRecordQueryExecutor;
use crate::infrastructure::search::{MySqlSearchIndex, MySqlSearchSource};
use crate::modules::BoxFuture;
use crate::records::filter_search_plan;

pub struct MySqlSearchPorts {
    registry: Arc<RecordTypeRegistry>,
    index: MySqlSearchIndex,
    source: MySqlSearchSource,
    definitions: MySqlFieldDefinitionRepository,
    values: MySqlCustomFieldValueRepository,
    executor: MySqlRecordQueryExecutor,
    pool: MySqlPool,
    jobs: MySqlJobQueue,
    contents: SearchContents,
}

impl MySqlSearchPorts {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            registry,
            index: MySqlSearchIndex::new(pool.clone()),
            source: MySqlSearchSource::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool.clone()),
            values: MySqlCustomFieldValueRepository::new(pool.clone()),
            executor: MySqlRecordQueryExecutor::new(pool.clone()),
            jobs: MySqlJobQueue::new(pool.clone()),
            contents: SearchContents::from_app(pool.clone()),
            pool,
        }
    }
}

impl SearchPorts for MySqlSearchPorts {
    type Tx = MySqlTx;
    type Index = MySqlSearchIndex;
    type Source = MySqlSearchSource;
    type Definitions = MySqlFieldDefinitionRepository;
    type Values = MySqlCustomFieldValueRepository;
    type Executor = MySqlRecordQueryExecutor;
    type Jobs = MySqlJobQueue;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn index(&self) -> &Self::Index {
        &self.index
    }

    fn source(&self) -> &Self::Source {
        &self.source
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
    }

    fn values(&self) -> &Self::Values {
        &self.values
    }

    fn executor(&self) -> &Self::Executor {
        &self.executor
    }

    fn restrict<'a>(
        &'a self,
        scope: &'a AccessScope,
        plan: QueryPlan,
    ) -> BoxFuture<'a, Result<QueryPlan, AppError>> {
        Box::pin(filter_search_plan(&self.pool, scope, plan))
    }

    fn jobs(&self) -> &Self::Jobs {
        &self.jobs
    }

    fn contents(&self) -> &SearchContents {
        &self.contents
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}

pub fn search_index_consumer(
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
) -> Arc<dyn EventConsumer> {
    CatalogEventConsumer::shared(
        catalog,
        Arc::new(move |registry| {
            Arc::new(SearchIndexConsumer::new(MySqlSearchPorts::new(
                pool.clone(),
                registry,
            ))) as Arc<dyn EventConsumer>
        }),
    )
}

pub fn search_reindex_handler(
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
) -> Arc<dyn JobHandler> {
    CatalogJobHandler::shared(
        catalog,
        Arc::new(move |registry| {
            Arc::new(SearchReindexHandler::new(MySqlSearchPorts::new(
                pool.clone(),
                registry,
            ))) as Arc<dyn JobHandler>
        }),
    )
}
