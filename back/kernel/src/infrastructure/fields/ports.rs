use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::fields::{CustomFieldPorts, FieldPorts, RecordTypeRegistry};
use crate::application::layouts::LayoutConditionalRules;
use crate::application::shared::{AuditLog, EventPublisher};
use crate::database::transaction::MySqlTx;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::custom_objects::MySqlRecordLinkRepository;
use crate::infrastructure::fields::{
    MySqlCustomFieldValueRepository, MySqlFieldDefinitionRepository, MySqlReferenceChecker,
};
use crate::infrastructure::layouts::MySqlFormLayoutRepository;
use crate::infrastructure::transactions::MySqlTransactionManager;
use crate::records::MySqlRecordPipeline;

pub(crate) type MySqlConditionalRules =
    LayoutConditionalRules<MySqlFormLayoutRepository, SystemClock>;

pub struct MySqlCustomFieldPorts {
    registry: Arc<RecordTypeRegistry>,
    definitions: MySqlFieldDefinitionRepository,
    values: MySqlCustomFieldValueRepository,
    references: MySqlReferenceChecker,
    rules: MySqlConditionalRules,
    links: MySqlRecordLinkRepository,
    pipeline: MySqlRecordPipeline,
}

impl MySqlCustomFieldPorts {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            pipeline: MySqlRecordPipeline::new(pool.clone(), registry.clone()),
            links: MySqlRecordLinkRepository::new(pool.clone(), registry.clone()),
            registry,
            definitions: MySqlFieldDefinitionRepository::new(pool.clone()),
            values: MySqlCustomFieldValueRepository::new(pool.clone()),
            references: MySqlReferenceChecker::new(pool.clone()),
            rules: LayoutConditionalRules::new(MySqlFormLayoutRepository::new(pool), SystemClock),
        }
    }
}

impl CustomFieldPorts for MySqlCustomFieldPorts {
    type Tx = MySqlTx;
    type Definitions = MySqlFieldDefinitionRepository;
    type Values = MySqlCustomFieldValueRepository;
    type References = MySqlReferenceChecker;
    type Rules = MySqlConditionalRules;
    type Links = MySqlRecordLinkRepository;
    type Pipeline = MySqlRecordPipeline;

    fn pipeline(&self) -> &Self::Pipeline {
        &self.pipeline
    }

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn links(&self) -> &Self::Links {
        &self.links
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

    fn rules(&self) -> &Self::Rules {
        &self.rules
    }
}

pub struct MySqlFieldPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    definitions: MySqlFieldDefinitionRepository,
    values: MySqlCustomFieldValueRepository,
    events: Arc<dyn EventPublisher<MySqlTx>>,
    audit: Arc<dyn AuditLog<MySqlTx>>,
}

impl MySqlFieldPorts {
    pub fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: Arc<dyn EventPublisher<MySqlTx>>,
        audit: Arc<dyn AuditLog<MySqlTx>>,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool.clone()),
            values: MySqlCustomFieldValueRepository::new(pool),
            events,
            audit,
        }
    }
}

impl FieldPorts for MySqlFieldPorts {
    type Transactions = MySqlTransactionManager;
    type Definitions = MySqlFieldDefinitionRepository;
    type Values = MySqlCustomFieldValueRepository;
    type Events = Arc<dyn EventPublisher<MySqlTx>>;
    type Audit = Arc<dyn AuditLog<MySqlTx>>;
    type Clock = SystemClock;

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

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
