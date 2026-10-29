pub(crate) mod mysql_two_factor;
pub(crate) mod totp;

use std::sync::Arc;

use sqlx::MySqlPool;

pub use mysql_two_factor::{
    MySqlTwoFactorPolicyRepository, MySqlTwoFactorRepository, rotate_two_factor_secrets,
};
pub(crate) use totp::TotpPasswords;

use crate::application::auth::two_factor::{TwoFactorPorts, TwoFactorUseCases};
use crate::application::security::SharedSecurityEventRecorder;
use crate::infrastructure::auth::{Argon2PasswordHasher, Sha256SecureTokens, SystemClock};
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::secrets::EncryptionKeyring;
use crate::infrastructure::transactions::MySqlTransactionManager;

pub(crate) type MySqlTwoFactorUseCases = TwoFactorUseCases<MySqlTwoFactorPorts>;

pub struct MySqlTwoFactorPorts {
    store: MySqlTwoFactorRepository,
    transactions: MySqlTransactionManager,
    policies: MySqlTwoFactorPolicyRepository,
    recorder: SharedSecurityEventRecorder,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlTwoFactorPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        keyring: Arc<EncryptionKeyring>,
        recorder: SharedSecurityEventRecorder,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            store: MySqlTwoFactorRepository::new(pool.clone(), keyring),
            transactions: MySqlTransactionManager::new(pool.clone()),
            policies: MySqlTwoFactorPolicyRepository::new(pool),
            recorder,
            events,
            audit,
        }
    }
}

impl TwoFactorPorts for MySqlTwoFactorPorts {
    type Store = MySqlTwoFactorRepository;
    type Passwords = TotpPasswords;
    type Hasher = Argon2PasswordHasher;
    type Digests = Sha256SecureTokens;
    type Clock = SystemClock;
    type Recorder = SharedSecurityEventRecorder;
    type Transactions = MySqlTransactionManager;
    type Policies = MySqlTwoFactorPolicyRepository;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;

    fn store(&self) -> &Self::Store {
        &self.store
    }

    fn passwords(&self) -> &Self::Passwords {
        &TotpPasswords
    }

    fn hasher(&self) -> &Self::Hasher {
        &Argon2PasswordHasher
    }

    fn digests(&self) -> &Self::Digests {
        &Sha256SecureTokens
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }

    fn recorder(&self) -> &Self::Recorder {
        &self.recorder
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn policies(&self) -> &Self::Policies {
        &self.policies
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }
}
