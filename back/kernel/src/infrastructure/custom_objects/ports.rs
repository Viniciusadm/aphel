use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::custom_objects::{
    CustomObjectAdminPorts, CustomRecordPorts, RelationPorts,
};
use crate::application::fields::RecordTypeRegistry;
use crate::domain::custom_objects::{CustomObject, RelationshipDefinition};
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::custom_objects::{
    MySqlCustomObjectRepository, MySqlCustomRecordRepository, MySqlRecordLinkRepository,
    MySqlRelationshipRepository,
};
use crate::infrastructure::fields::{MySqlCustomFieldPorts, MySqlFieldDefinitionRepository};
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::query::MySqlRecordQueryPorts;
use crate::infrastructure::records::MySqlRecordAccessChecker;
use crate::infrastructure::transactions::MySqlTransactionManager;

pub(crate) struct MySqlCustomObjectAdminPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    objects: MySqlCustomObjectRepository,
    relationships: MySqlRelationshipRepository,
    definitions: MySqlFieldDefinitionRepository,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlCustomObjectAdminPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            objects: MySqlCustomObjectRepository::new(pool.clone()),
            relationships: MySqlRelationshipRepository::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool),
            events,
            audit,
        }
    }
}

impl CustomObjectAdminPorts for MySqlCustomObjectAdminPorts {
    type Transactions = MySqlTransactionManager;
    type Objects = MySqlCustomObjectRepository;
    type Relationships = MySqlRelationshipRepository;
    type Definitions = MySqlFieldDefinitionRepository;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn objects(&self) -> &Self::Objects {
        &self.objects
    }

    fn relationships(&self) -> &Self::Relationships {
        &self.relationships
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
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

pub struct MySqlRelationPorts {
    registry: Arc<RecordTypeRegistry>,
    relationships: Vec<RelationshipDefinition>,
    transactions: MySqlTransactionManager,
    links: MySqlRecordLinkRepository,
    access: MySqlRecordAccessChecker,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlRelationPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            relationships: registry.custom_schema().relationships.clone(),
            transactions: MySqlTransactionManager::new(pool.clone()),
            links: MySqlRecordLinkRepository::new(pool.clone(), registry.clone()),
            access: MySqlRecordAccessChecker::new(pool, registry.clone()),
            registry,
            events,
            audit,
        }
    }
}

impl RelationPorts for MySqlRelationPorts {
    type Transactions = MySqlTransactionManager;
    type Links = MySqlRecordLinkRepository;
    type Access = MySqlRecordAccessChecker;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn relationships(&self) -> &[RelationshipDefinition] {
        &self.relationships
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn links(&self) -> &Self::Links {
        &self.links
    }

    fn access(&self) -> &Self::Access {
        &self.access
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

pub struct MySqlCustomRecordPorts {
    objects: Vec<CustomObject>,
    relationships: Vec<RelationshipDefinition>,
    transactions: MySqlTransactionManager,
    records: MySqlCustomRecordRepository,
    links: MySqlRecordLinkRepository,
    fields: MySqlCustomFieldPorts,
    query: MySqlRecordQueryPorts,
    access: MySqlRecordAccessChecker,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlCustomRecordPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        let schema = registry.custom_schema();
        Self {
            objects: schema.objects.clone(),
            relationships: schema.relationships.clone(),
            transactions: MySqlTransactionManager::new(pool.clone()),
            records: MySqlCustomRecordRepository::new(pool.clone()),
            links: MySqlRecordLinkRepository::new(pool.clone(), registry.clone()),
            fields: MySqlCustomFieldPorts::new(pool.clone(), registry.clone()),
            query: MySqlRecordQueryPorts::new(pool.clone(), registry.clone()),
            access: MySqlRecordAccessChecker::new(pool, registry),
            events,
            audit,
        }
    }
}

impl CustomRecordPorts for MySqlCustomRecordPorts {
    type Transactions = MySqlTransactionManager;
    type Records = MySqlCustomRecordRepository;
    type Links = MySqlRecordLinkRepository;
    type Fields = MySqlCustomFieldPorts;
    type Query = MySqlRecordQueryPorts;
    type Access = MySqlRecordAccessChecker;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn objects(&self) -> &[CustomObject] {
        &self.objects
    }

    fn relationships(&self) -> &[RelationshipDefinition] {
        &self.relationships
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn records(&self) -> &Self::Records {
        &self.records
    }

    fn links(&self) -> &Self::Links {
        &self.links
    }

    fn fields(&self) -> &Self::Fields {
        &self.fields
    }

    fn query(&self) -> &Self::Query {
        &self.query
    }

    fn access(&self) -> &Self::Access {
        &self.access
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
