use std::sync::Arc;

use sqlx::MySqlPool;

use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::infrastructure::auth::SystemClock;
use aphel_kernel::infrastructure::custom_objects::{
    MySqlCustomObjectRepository, MySqlRelationshipRepository,
};
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_kernel::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use aphel_kernel::infrastructure::transactions::MySqlTransactionManager;

use crate::application::CustomObjectAdminPorts;
pub struct MySqlCustomObjectAdminPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    objects: MySqlCustomObjectRepository,
    relationships: MySqlRelationshipRepository,
    definitions: MySqlFieldDefinitionRepository,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlCustomObjectAdminPorts {
    pub fn new(
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
