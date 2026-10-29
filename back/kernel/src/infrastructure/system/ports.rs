use sqlx::MySqlPool;

use crate::application::system::ports::SystemAdminPorts;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::system::audit::MySqlSystemAuditLog;
use crate::infrastructure::system::migrations::{CompiledModuleInventory, MySqlMigrationInspector};
use crate::infrastructure::system::monitor::{MySqlFailedJobRepository, MySqlQueueMonitor};
use crate::infrastructure::transactions::MySqlTransactionManager;

pub struct MySqlSystemAdminPorts {
    transactions: MySqlTransactionManager,
    monitor: MySqlQueueMonitor,
    failed_jobs: MySqlFailedJobRepository,
    audit: MySqlSystemAuditLog,
    migrations: MySqlMigrationInspector,
    modules: CompiledModuleInventory,
    clock: SystemClock,
}

impl MySqlSystemAdminPorts {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            monitor: MySqlQueueMonitor::new(pool.clone()),
            failed_jobs: MySqlFailedJobRepository,
            audit: MySqlSystemAuditLog,
            migrations: MySqlMigrationInspector::new(pool.clone()),
            modules: CompiledModuleInventory::new(Some(pool)),
            clock: SystemClock,
        }
    }
}

impl SystemAdminPorts for MySqlSystemAdminPorts {
    type Transactions = MySqlTransactionManager;
    type Monitor = MySqlQueueMonitor;
    type FailedJobs = MySqlFailedJobRepository;
    type Audit = MySqlSystemAuditLog;
    type Migrations = MySqlMigrationInspector;
    type Modules = CompiledModuleInventory;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn monitor(&self) -> &Self::Monitor {
        &self.monitor
    }

    fn failed_jobs(&self) -> &Self::FailedJobs {
        &self.failed_jobs
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn migrations(&self) -> &Self::Migrations {
        &self.migrations
    }

    fn modules(&self) -> &Self::Modules {
        &self.modules
    }

    fn clock(&self) -> &Self::Clock {
        &self.clock
    }
}
