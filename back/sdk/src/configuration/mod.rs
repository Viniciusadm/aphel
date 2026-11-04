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
