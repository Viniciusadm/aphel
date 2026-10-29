use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::fields::RecordTypeRegistry;
use crate::application::record_sharing::RecordSharingPorts;
use crate::application::security::SharedSecurityEventRecorder;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::record_sharing::{MySqlRecordDirectory, MySqlRecordShareRepository};
use crate::infrastructure::transactions::MySqlTransactionManager;

pub(crate) struct MySqlRecordSharingAdapters {
    pub(crate) events: SharedEventPublisher,
    pub(crate) audit: SharedAuditLog,
    pub(crate) security_events: SharedSecurityEventRecorder,
}

pub struct MySqlRecordSharingPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    shares: MySqlRecordShareRepository,
    records: MySqlRecordDirectory,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    security_events: SharedSecurityEventRecorder,
    clock: SystemClock,
}

impl MySqlRecordSharingPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        adapters: MySqlRecordSharingAdapters,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            shares: MySqlRecordShareRepository::new(pool.clone()),
            records: MySqlRecordDirectory::new(pool),
            events: adapters.events,
            audit: adapters.audit,
            security_events: adapters.security_events,
            clock: SystemClock,
        }
    }
}

impl RecordSharingPorts for MySqlRecordSharingPorts {
    type Transactions = MySqlTransactionManager;
    type Shares = MySqlRecordShareRepository;
    type Records = MySqlRecordDirectory;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn shares(&self) -> &Self::Shares {
        &self.shares
    }

    fn records(&self) -> &Self::Records {
        &self.records
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn clock(&self) -> &Self::Clock {
        &self.clock
    }

    fn security_events(&self) -> &SharedSecurityEventRecorder {
        &self.security_events
    }
}
