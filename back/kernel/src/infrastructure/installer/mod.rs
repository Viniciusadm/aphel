use std::sync::Arc;

use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::application::fields::RecordTypeRegistry;
use crate::application::installer::{FirstOrganization, InstallationRepository, InstallerPorts};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::auth::{Argon2PasswordHasher, SystemClock};
use crate::infrastructure::organizations::settings::write_configuration;
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::mysql_organizations::insert_registration;
use crate::infrastructure::transactions::MySqlTransactionManager;

#[derive(Clone)]
pub struct MySqlInstallationRepository {
    pool: MySqlPool,
    record_types: Arc<RecordTypeRegistry>,
}

impl MySqlInstallationRepository {
    pub(crate) fn new(pool: MySqlPool, record_types: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, record_types }
    }
}

#[async_trait]
impl InstallationRepository for MySqlInstallationRepository {
    type Tx = MySqlTx;

    async fn is_available(&self) -> Result<bool, AppError> {
        let available: bool =
            sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM organization_settings)")
                .fetch_one(&self.pool)
                .await?;
        Ok(available)
    }

    async fn claim(&self, tx: &mut MySqlTx) -> Result<bool, AppError> {
        let available: bool =
            sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM organization_settings FOR UPDATE)")
                .fetch_one(&mut **tx)
                .await?;
        Ok(available)
    }

    async fn create_first_organization(
        &self,
        tx: &mut MySqlTx,
        organization: &FirstOrganization,
    ) -> Result<(), AppError> {
        let registration = &organization.registration;
        insert_registration(tx, registration, &self.record_types).await?;
        write_configuration(tx, &organization.configuration).await?;
        sqlx::query("UPDATE users SET email_verified_at = ? WHERE id = ?")
            .bind(mapper::naive(organization.email_verified_at))
            .bind(registration.owner.id.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}

pub struct MySqlInstallerPorts {
    transactions: MySqlTransactionManager,
    installation: MySqlInstallationRepository,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlInstallerPorts {
    pub fn new(
        pool: MySqlPool,
        record_types: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            installation: MySqlInstallationRepository::new(pool, record_types),
            events,
            audit,
        }
    }
}

impl InstallerPorts for MySqlInstallerPorts {
    type Transactions = MySqlTransactionManager;
    type Installation = MySqlInstallationRepository;
    type Hasher = Argon2PasswordHasher;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn installation(&self) -> &Self::Installation {
        &self.installation
    }

    fn hasher(&self) -> &Self::Hasher {
        &Argon2PasswordHasher
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
