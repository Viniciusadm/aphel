use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};

use crate::application::organization_profile::{
    DEFAULT_SITE_BASE_PATH, OrganizationConfig, OrganizationConfigCatalog, OrganizationConfigPorts,
    OrganizationConfigRepository, OrganizationConfigSource, OrganizationConfigUseCases, ROOT_PATH,
    SecretChange, UpdateOrganizationConfig,
};
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::mail::SmtpMailerProvider;
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher, SharedSecretVault};
use crate::infrastructure::storage::ObjectStorageProbe;
use crate::infrastructure::transactions::MySqlTransactionManager;
use crate::tenancy::TenantLocal;

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

fn version_of(updated_at: NaiveDateTime) -> i64 {
    updated_at.and_utc().timestamp_micros()
}

fn parse_config(value: Option<Json<Value>>) -> OrganizationConfig {
    match value {
        Some(Json(Value::Null)) | None => OrganizationConfig::default(),
        Some(Json(value)) => serde_json::from_value(value).unwrap_or_else(|error| {
            tracing::warn!(%error, "invalid organization settings; using the defaults");
            OrganizationConfig::default()
        }),
    }
}

#[derive(Clone)]
pub struct MySqlOrganizationConfigSource {
    pool: MySqlPool,
}

impl MySqlOrganizationConfigSource {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationConfigSource for MySqlOrganizationConfigSource {
    async fn version(&self) -> Result<Option<i64>, AppError> {
        let updated_at: Option<NaiveDateTime> =
            sqlx::query_scalar("SELECT updated_at FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(updated_at.map(version_of))
    }

    async fn load(&self) -> Result<Option<(i64, OrganizationConfig)>, AppError> {
        let row = sqlx::query(
            "SELECT JSON_EXTRACT(settings, '$.configuration') AS configuration, updated_at \
             FROM organization_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let updated_at: NaiveDateTime = row.try_get("updated_at")?;
        let configuration: Option<Json<Value>> = row.try_get("configuration")?;
        Ok(Some((version_of(updated_at), parse_config(configuration))))
    }
}

pub fn organization_config_catalog(pool: &MySqlPool) -> Arc<OrganizationConfigCatalog> {
    static CATALOG: TenantLocal<OnceLock<Arc<OrganizationConfigCatalog>>> =
        TenantLocal::new(OnceLock::new);
    let create = || {
        Arc::new(OrganizationConfigCatalog::new(Arc::new(
            MySqlOrganizationConfigSource::new(pool.clone()),
        )))
    };
    match CATALOG.get() {
        Some(catalog) => catalog.get_or_init(create).clone(),
        None => create(),
    }
}

pub async fn reset_app_mount(
    pool: MySqlPool,
    secrets: SharedSecretVault,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    catalog: Arc<OrganizationConfigCatalog>,
) -> Result<String, AppError> {
    let current = MySqlOrganizationConfigRepository::new(pool.clone())
        .find()
        .await?
        .unwrap_or_default();
    let site_on_root =
        current.mounts.site.host.is_none() && current.mounts.site.prefix().is_empty();
    let use_cases = OrganizationConfigUseCases::new(MySqlOrganizationConfigPorts::new(
        pool, secrets, events, audit, catalog,
    ));
    let view = use_cases
        .update(
            &AccessScope::system(),
            UpdateOrganizationConfig {
                public_base_url: None,
                app_base_path: Some(ROOT_PATH.to_owned()),
                site_base_path: site_on_root.then(|| DEFAULT_SITE_BASE_PATH.to_owned()),
                site_host: None,
                site_redirect_hosts: None,
                max_upload_mb: None,
                storage: None,
                storage_secret: SecretChange::Keep,
                mail: None,
                mail_secret: SecretChange::Keep,
                api_tokens: None,
                extensions: Default::default(),
                ip: None,
            },
        )
        .await?;
    Ok(view.config.app_base_url())
}

pub fn spawn_config_refresh(
    catalog: Arc<OrganizationConfigCatalog>,
) -> tokio::task::JoinHandle<()> {
    crate::tenancy::spawn(async move {
        let mut interval = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            interval.tick().await;
            catalog.refresh().await;
        }
    })
}

pub(crate) async fn write_configuration(
    tx: &mut MySqlTx,
    config: &OrganizationConfig,
) -> Result<(), AppError> {
    let value = serde_json::to_value(config)
        .map_err(|error| AppError::Internal(format!("invalid organization settings: {error}")))?;
    sqlx::query(
        "UPDATE organization_settings \
         SET settings = JSON_SET(settings, '$.configuration', CAST(? AS JSON)), \
         updated_at = CURRENT_TIMESTAMP(6) WHERE id = 1",
    )
    .bind(Json(value))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(Clone)]
pub struct MySqlOrganizationConfigRepository {
    pool: MySqlPool,
}

impl MySqlOrganizationConfigRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationConfigRepository for MySqlOrganizationConfigRepository {
    type Tx = MySqlTx;

    async fn find(&self) -> Result<Option<OrganizationConfig>, AppError> {
        let row: Option<Option<Json<Value>>> = sqlx::query_scalar(
            "SELECT JSON_EXTRACT(settings, '$.configuration') FROM organization_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(parse_config))
    }

    async fn find_for_update(
        &self,
        tx: &mut MySqlTx,
    ) -> Result<Option<OrganizationConfig>, AppError> {
        let row: Option<Option<Json<Value>>> = sqlx::query_scalar(
            "SELECT JSON_EXTRACT(settings, '$.configuration') FROM organization_settings \
             WHERE id = 1 FOR UPDATE",
        )
        .fetch_optional(&mut **tx)
        .await?;
        Ok(row.map(parse_config))
    }

    async fn save(&self, tx: &mut MySqlTx, config: &OrganizationConfig) -> Result<(), AppError> {
        write_configuration(tx, config).await
    }
}

pub struct MySqlOrganizationConfigPorts {
    transactions: MySqlTransactionManager,
    settings: MySqlOrganizationConfigRepository,
    secrets: SharedSecretVault,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    storage: ObjectStorageProbe,
    mail: SmtpMailerProvider,
    catalog: Arc<OrganizationConfigCatalog>,
}

impl MySqlOrganizationConfigPorts {
    pub fn new(
        pool: MySqlPool,
        secrets: SharedSecretVault,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
        catalog: Arc<OrganizationConfigCatalog>,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            settings: MySqlOrganizationConfigRepository::new(pool.clone()),
            mail: SmtpMailerProvider::new(pool, secrets.clone()),
            secrets,
            events,
            audit,
            storage: ObjectStorageProbe,
            catalog,
        }
    }
}

impl OrganizationConfigPorts for MySqlOrganizationConfigPorts {
    type Transactions = MySqlTransactionManager;
    type Settings = MySqlOrganizationConfigRepository;
    type Secrets = SharedSecretVault;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Storage = ObjectStorageProbe;
    type Mail = SmtpMailerProvider;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn settings(&self) -> &Self::Settings {
        &self.settings
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

    fn storage(&self) -> &Self::Storage {
        &self.storage
    }

    fn mail(&self) -> &Self::Mail {
        &self.mail
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }

    fn catalog(&self) -> &OrganizationConfigCatalog {
        &self.catalog
    }
}
