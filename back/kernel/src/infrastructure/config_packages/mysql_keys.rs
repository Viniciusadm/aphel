use crate::application::config_packages::ConfigReferenceKind;
use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::config_packages::ConfigKeyRepository;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlConfigKeyRepository {
    pool: MySqlPool,
}

impl MySqlConfigKeyRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConfigKeyRepository for MySqlConfigKeyRepository {
    type Tx = MySqlTx;

    async fn keys(
        &self,
        item_type: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let rows = sqlx::query("SELECT item_id, `key` FROM config_item_keys WHERE item_type = ?")
            .bind(item_type.key())
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                let id: String = row.try_get("item_id")?;
                let key: String = row.try_get("key")?;
                let id =
                    Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("item_id", &id))?;
                Ok((id, key))
            })
            .collect()
    }

    async fn assign(
        &self,
        tx: &mut MySqlTx,
        item_type: ConfigReferenceKind,
        item_id: Uuid,
        key: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO config_item_keys (item_type, item_id, `key`) \
             VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE `key` = VALUES(`key`)",
        )
        .bind(item_type.key())
        .bind(item_id.to_string())
        .bind(key)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
