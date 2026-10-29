use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::fields::RecordTypeRegistry;
use crate::application::notifications::NotificationPorts;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::notifications::{
    MySqlNotificationDirectory, MySqlNotificationPreferenceRepository, MySqlNotificationRepository,
};
use crate::infrastructure::platform::SharedJobQueue;
use crate::infrastructure::transactions::MySqlTransactionManager;

#[derive(Clone)]
pub struct MySqlNotificationPorts {
    transactions: MySqlTransactionManager,
    notifications: MySqlNotificationRepository,
    preferences: MySqlNotificationPreferenceRepository,
    directory: MySqlNotificationDirectory,
    jobs: SharedJobQueue,
}

impl MySqlNotificationPorts {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>, jobs: SharedJobQueue) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            notifications: MySqlNotificationRepository::new(pool.clone()),
            preferences: MySqlNotificationPreferenceRepository::new(pool.clone()),
            directory: MySqlNotificationDirectory::new(pool, registry),
            jobs,
        }
    }
}

impl NotificationPorts for MySqlNotificationPorts {
    type Transactions = MySqlTransactionManager;
    type Notifications = MySqlNotificationRepository;
    type Preferences = MySqlNotificationPreferenceRepository;
    type Directory = MySqlNotificationDirectory;
    type Jobs = SharedJobQueue;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn notifications(&self) -> &Self::Notifications {
        &self.notifications
    }

    fn preferences(&self) -> &Self::Preferences {
        &self.preferences
    }

    fn directory(&self) -> &Self::Directory {
        &self.directory
    }

    fn jobs(&self) -> &Self::Jobs {
        &self.jobs
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
