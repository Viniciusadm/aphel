use sqlx::MySqlPool;

use crate::application::api_tokens::ApiTokenPorts;
use crate::infrastructure::api_tokens::MySqlApiTokenRepository;
use crate::infrastructure::auth::{Sha256SecureTokens, SystemClock};
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;
use crate::infrastructure::transactions::MySqlTransactionManager;

pub struct MySqlApiTokenPorts {
    transactions: MySqlTransactionManager,
    tokens: MySqlApiTokenRepository,
    users: MySqlUserRepository,
    access: MySqlAccessRepository,
    secrets: Sha256SecureTokens,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    clock: SystemClock,
}

impl MySqlApiTokenPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            tokens: MySqlApiTokenRepository::new(pool.clone()),
            users: MySqlUserRepository::new(pool.clone()),
            access: MySqlAccessRepository::new(pool),
            secrets: Sha256SecureTokens,
            events,
            audit,
            clock: SystemClock,
        }
    }
}

impl ApiTokenPorts for MySqlApiTokenPorts {
    type Transactions = MySqlTransactionManager;
    type Tokens = MySqlApiTokenRepository;
    type Users = MySqlUserRepository;
    type Access = MySqlAccessRepository;
    type Secrets = Sha256SecureTokens;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn tokens(&self) -> &Self::Tokens {
        &self.tokens
    }

    fn users(&self) -> &Self::Users {
        &self.users
    }

    fn access(&self) -> &Self::Access {
        &self.access
    }

    fn secrets(&self) -> &Self::Secrets {
        &self.secrets
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
}
