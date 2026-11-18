use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{ConfigKeyRepository, ConfigReferenceKind, ConfigurationKeys};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use std::collections::HashMap;
use uuid::Uuid;
pub(crate) struct Keys;
#[async_trait::async_trait]
impl ConfigurationKeys for Keys {
    async fn keys(
        &self,
        pool: &Db,
        kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        crate::infrastructure::MySqlConfigKeyRepository::new(pool.clone())
            .keys(kind)
            .await
    }
    async fn assign(
        &self,
        tx: &mut Tx,
        kind: ConfigReferenceKind,
        id: Uuid,
        key: &str,
    ) -> Result<(), AppError> {
        crate::infrastructure::mysql_keys::assign_key(tx, kind, id, key).await
    }
}
