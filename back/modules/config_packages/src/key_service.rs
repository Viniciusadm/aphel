use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{ConfigKeyRepository, ConfigReferenceKind, ConfigurationKeys};
use sqlx::MySqlPool;
use std::collections::HashMap;
use uuid::Uuid;
pub(crate) struct Keys;
#[async_trait::async_trait]
impl ConfigurationKeys for Keys {
    async fn keys(
        &self,
        pool: &MySqlPool,
        kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        crate::infrastructure::MySqlConfigKeyRepository::new(pool.clone())
            .keys(kind)
            .await
    }
    async fn assign(
        &self,
        tx: &mut MySqlTx,
        kind: ConfigReferenceKind,
        id: Uuid,
        key: &str,
    ) -> Result<(), AppError> {
        crate::infrastructure::mysql_keys::assign_key(tx, kind, id, key).await
    }
}
