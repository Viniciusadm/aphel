pub mod keys;
pub mod section;
pub mod sections;
pub mod versioning;
pub use aphel_contract_configuration::*;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
pub use keys::*;
pub use section::*;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait ConfigKeyRepository: Send + Sync {
    type Tx: Send;

    async fn keys(&self, item_type: ConfigReferenceKind)
    -> Result<HashMap<Uuid, String>, AppError>;
    async fn assign(
        &self,
        tx: &mut Self::Tx,
        item_type: ConfigReferenceKind,
        item_id: Uuid,
        key: &str,
    ) -> Result<(), AppError>;
}

pub type ConfigSectionBuild = Arc<
    dyn Fn(&MySqlPool, &Arc<RecordTypeRegistry>) -> Arc<dyn ConfigSection<MySqlTx>> + Send + Sync,
>;

pub struct ConfigSectionFactory {
    pub order: u16,
    pub build: ConfigSectionBuild,
}

impl ConfigSectionFactory {
    pub fn new<F>(order: u16, build: F) -> Self
    where
        F: Fn(&MySqlPool, &Arc<RecordTypeRegistry>) -> Arc<dyn ConfigSection<MySqlTx>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            order,
            build: Arc::new(build),
        }
    }
}

#[async_trait]
pub trait ConfigurationKeys: Send + Sync {
    async fn keys(
        &self,
        pool: &MySqlPool,
        kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError>;
    async fn assign(
        &self,
        tx: &mut MySqlTx,
        kind: ConfigReferenceKind,
        id: Uuid,
        key: &str,
    ) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct ConfigKeyClient {
    pool: MySqlPool,
}
impl ConfigKeyClient {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
fn key_service() -> Result<Arc<dyn ConfigurationKeys>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn ConfigurationKeys>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "configuration_keys_unavailable",
                "configuration key service is not enabled",
            )
        })
}
#[async_trait]
impl ConfigKeyRepository for ConfigKeyClient {
    type Tx = MySqlTx;
    async fn keys(&self, kind: ConfigReferenceKind) -> Result<HashMap<Uuid, String>, AppError> {
        key_service()?.keys(&self.pool, kind).await
    }
    async fn assign(
        &self,
        tx: &mut MySqlTx,
        kind: ConfigReferenceKind,
        id: Uuid,
        key: &str,
    ) -> Result<(), AppError> {
        key_service()?.assign(tx, kind, id, key).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationReference {
    pub id: Uuid,
    pub template_key: Option<String>,
}

#[async_trait]
pub trait ConfigurationCatalog: Send + Sync {
    fn template_exists(
        &self,
        ctx: &crate::Ctx<'_>,
        key: &str,
        locale: &str,
    ) -> Result<bool, AppError>;
    async fn installation(
        &self,
        ctx: &crate::Ctx<'_>,
        id: Uuid,
    ) -> Result<Option<InstallationReference>, AppError>;
}

use aphel_kernel::domain::UserId;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigPackageInstall {
    pub id: Uuid,
    pub name: String,
    pub format_version: u32,
    pub template_key: Option<String>,
    pub package: Value,
    pub summary: Value,
    pub installed_by: Option<UserId>,
    pub installed_by_name: Option<String>,
    pub installed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallResult {
    pub install: ConfigPackageInstall,
    pub applied: Vec<PlanItem>,
    pub summary: PlanSummary,
}

#[async_trait]
pub trait ConfigurationPackages: Send + Sync {
    async fn export(
        &self,
        ctx: &crate::Ctx<'_>,
        command: ExportCommand,
        locale: &str,
    ) -> Result<ConfigPackage, AppError>;
    async fn plan(
        &self,
        ctx: &crate::Ctx<'_>,
        source: PackageSource,
    ) -> Result<PackagePlan, AppError>;
    async fn apply(
        &self,
        ctx: &crate::Ctx<'_>,
        command: ApplyCommand,
    ) -> Result<InstallResult, AppError>;
}

pub fn configuration_packages(
    ctx: &crate::Ctx<'_>,
) -> Result<Arc<dyn ConfigurationPackages>, AppError> {
    ctx.services()
        .get::<dyn ConfigurationPackages>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "configuration_packages_unavailable",
                "configuration packages is not enabled",
            )
        })
}

#[async_trait]
pub trait ConfigurationReferences: Send + Sync {
    fn kinds(&self) -> &'static [ConfigReferenceKind];
    async fn keys(
        &self,
        pool: &MySqlPool,
        kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError>;
}

pub async fn reference_keys(
    pool: &MySqlPool,
    kind: ConfigReferenceKind,
) -> Result<HashMap<Uuid, String>, AppError> {
    reference_keys_from(
        pool,
        kind,
        aphel_kernel::modules::app().contributions::<dyn ConfigurationReferences>(),
    )
    .await
}

pub async fn stored_reference_keys(
    pool: &MySqlPool,
    kind: ConfigReferenceKind,
) -> Result<HashMap<Uuid, String>, AppError> {
    reference_keys_from(
        pool,
        kind,
        aphel_kernel::modules::app()
            .all_contributions::<dyn ConfigurationReferences>()
            .into_iter()
            .map(|(_, source)| source)
            .collect(),
    )
    .await
}

async fn reference_keys_from(
    pool: &MySqlPool,
    kind: ConfigReferenceKind,
    sources: Vec<Arc<dyn ConfigurationReferences>>,
) -> Result<HashMap<Uuid, String>, AppError> {
    let mut matching = sources
        .iter()
        .filter(|source| source.kinds().contains(&kind));
    let Some(source) = matching.next() else {
        return Ok(HashMap::new());
    };
    if matching.next().is_some() {
        return Err(AppError::Configuration(format!(
            "multiple configuration reference sources for {}",
            kind.key()
        )));
    }
    source.keys(pool, kind).await
}
