use std::sync::Arc;

use sqlx::MySqlPool;

use aphel_kernel::application::custom_objects::{
    CatalogEventConsumer, CatalogJobHandler, RecordTypeCatalog,
};

use crate::application::{
    FormulaDailyHandler, FormulaEventConsumer, FormulaPorts, FormulaRecalculateHandler,
};
use crate::infrastructure::MySqlFormulaSource;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::{EventConsumer, JobHandler};
use aphel_kernel::infrastructure::fields::{
    MySqlCustomFieldValueRepository, MySqlFieldDefinitionRepository, MySqlReferenceChecker,
};
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::infrastructure::transactions::MySqlTransactionManager;

pub struct MySqlFormulaPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    definitions: MySqlFieldDefinitionRepository,
    values: MySqlCustomFieldValueRepository,
    references: MySqlReferenceChecker,
    source: MySqlFormulaSource,
    jobs: MySqlJobQueue,
}

impl MySqlFormulaPorts {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool.clone()),
            values: MySqlCustomFieldValueRepository::new(pool.clone()),
            references: MySqlReferenceChecker::new(pool.clone()),
            source: MySqlFormulaSource::new(pool.clone()),
            jobs: MySqlJobQueue::new(pool),
        }
    }
}

impl FormulaPorts for MySqlFormulaPorts {
    type Transactions = MySqlTransactionManager;
    type Definitions = MySqlFieldDefinitionRepository;
    type Values = MySqlCustomFieldValueRepository;
    type References = MySqlReferenceChecker;
    type Source = MySqlFormulaSource;
    type Jobs = MySqlJobQueue;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
    }

    fn values(&self) -> &Self::Values {
        &self.values
    }

    fn references(&self) -> &Self::References {
        &self.references
    }

    fn source(&self) -> &Self::Source {
        &self.source
    }

    fn jobs(&self) -> &Self::Jobs {
        &self.jobs
    }
}

pub fn formula_events_consumer(
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
) -> Arc<dyn EventConsumer> {
    CatalogEventConsumer::shared(
        catalog,
        Arc::new(move |registry| {
            Arc::new(FormulaEventConsumer::new(MySqlFormulaPorts::new(
                pool.clone(),
                registry,
            ))) as Arc<dyn EventConsumer>
        }),
    )
}

pub fn formula_recalculate_handler(
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
) -> Arc<dyn JobHandler> {
    CatalogJobHandler::shared(
        catalog,
        Arc::new(move |registry| {
            Arc::new(FormulaRecalculateHandler::new(MySqlFormulaPorts::new(
                pool.clone(),
                registry,
            ))) as Arc<dyn JobHandler>
        }),
    )
}

pub fn formula_daily_handler(pool: MySqlPool) -> Arc<dyn JobHandler> {
    Arc::new(FormulaDailyHandler::new(MySqlFormulaPorts::new(
        pool,
        Arc::new(RecordTypeRegistry::builtin()),
    )))
}
