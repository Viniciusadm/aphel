use std::sync::Arc;

use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::application::fields::RecordTypeRegistry;
use crate::application::installer::{FirstOrganization, InstallationRepository, InstallerPorts};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::auth::{Argon2PasswordHasher, SystemClock};
use crate::infrastructure::organization_profile::settings::write_configuration;
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher, SharedSecretVault};
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::mysql_organization_profile::insert_registration;
use crate::infrastructure::transactions::MySqlTransactionManager;

#[derive(Clone)]
pub struct MySqlInstallationRepository {
    pool: MySqlPool,
    record_types: Arc<RecordTypeRegistry>,
    secrets: SharedSecretVault,
}

impl MySqlInstallationRepository {
    pub(crate) fn new(
        pool: MySqlPool,
        record_types: Arc<RecordTypeRegistry>,
        secrets: SharedSecretVault,
    ) -> Self {
        Self {
            pool,
            record_types,
            secrets,
        }
    }
}

#[async_trait]
impl InstallationRepository for MySqlInstallationRepository {
    type Tx = MySqlTx;

    async fn configure_modules(
        &self,
        tx: &mut MySqlTx,
        modules: &[crate::application::installer::modules::PreparedModule],
    ) -> Result<(), AppError> {
        use crate::application::secrets::SecretVault;
        let app = crate::modules::app();
        let scope = crate::domain::access::AccessScope::system();
        for entry in app.modules().iter().filter(|entry| {
            !entry.manifest.required && !modules.iter().any(|module| module.key == entry.key())
        }) {
            sqlx::query("INSERT INTO installed_modules (module_key, enabled, settings, installed_version) VALUES (?, FALSE, JSON_OBJECT(), ?) ON DUPLICATE KEY UPDATE enabled = FALSE")
                .bind(entry.key()).bind(entry.manifest.version).execute(&mut **tx).await?;
        }
        for module in modules {
            let entry = app
                .module(&module.key)
                .ok_or_else(crate::application::module_admin::module_not_found)?;
            sqlx::query("INSERT INTO installed_modules (module_key, enabled, settings, installed_version, enabled_at) VALUES (?, TRUE, ?, ?, CURRENT_TIMESTAMP(6)) ON DUPLICATE KEY UPDATE enabled = TRUE, settings = VALUES(settings), installed_version = VALUES(installed_version), enabled_at = VALUES(enabled_at)")
                .bind(&module.key).bind(sqlx::types::Json(&module.values)).bind(entry.manifest.version).execute(&mut **tx).await?;
            crate::modules::migrations::apply_grants(tx, app, &module.key).await?;
            for (name, value) in &module.secrets {
                self.secrets
                    .store(
                        tx,
                        &scope,
                        &crate::modules::module_secret_key(&module.key, name),
                        value,
                    )
                    .await?;
            }
        }
        let enabled: Vec<String> = modules.iter().map(|module| module.key.clone()).collect();
        crate::infrastructure::defaults::seed_defaults(&mut **tx, &enabled).await?;
        Ok(())
    }

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
        if let Some(business) = &organization.business {
            let document = serde_json::to_string(business).map_err(|error| {
                AppError::Internal(format!("cannot serialize business answers: {error}"))
            })?;
            sqlx::query(
                "UPDATE organization_settings \
                 SET settings = JSON_SET(COALESCE(settings, JSON_OBJECT()), '$.installer', CAST(? AS JSON)) \
                 WHERE id = 1",
            )
            .bind(document)
            .execute(&mut **tx)
            .await?;
        }
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
        secrets: SharedSecretVault,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            installation: MySqlInstallationRepository::new(pool, record_types, secrets),
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

    fn app(&self) -> &crate::modules::App {
        crate::modules::app()
    }

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
