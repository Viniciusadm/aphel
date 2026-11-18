use crate::application::ConfigReferenceKind;
use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::ConfigKeyRepository;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

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
    type Tx = Tx;

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
        tx: &mut Tx,
        item_type: ConfigReferenceKind,
        item_id: Uuid,
        key: &str,
    ) -> Result<(), AppError> {
        assign_key(tx, item_type, item_id, key).await
    }
}

pub(crate) async fn assign_key(
    tx: &mut Tx,
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

pub(crate) async fn stable_key(
    pool: &aphel_sdk::platform::database::Db,
    item_type: &str,
    id: &str,
) -> Result<Option<String>, aphel_kernel::errors::AppError> {
    Ok(
        sqlx::query_scalar(
            "SELECT `key` FROM config_item_keys WHERE item_type = ? AND item_id = ?",
        )
        .bind(item_type)
        .bind(id)
        .fetch_optional(pool)
        .await?,
    )
}

pub(crate) async fn item_of_key(
    pool: &aphel_sdk::platform::database::Db,
    item_type: &str,
    key: &str,
) -> Result<Option<String>, aphel_kernel::errors::AppError> {
    Ok(
        sqlx::query_scalar(
            "SELECT item_id FROM config_item_keys WHERE item_type = ? AND `key` = ?",
        )
        .bind(item_type)
        .bind(key)
        .fetch_optional(pool)
        .await?,
    )
}
