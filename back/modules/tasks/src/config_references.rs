use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{ConfigReferenceKind, ConfigurationReferences};
use sqlx::{MySqlPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

const KINDS: &[ConfigReferenceKind] = &[ConfigReferenceKind::new("tasks.activity_type")];

pub(crate) struct References;

#[async_trait::async_trait]
impl ConfigurationReferences for References {
    fn kinds(&self) -> &'static [ConfigReferenceKind] {
        KINDS
    }
    async fn keys(
        &self,
        pool: &MySqlPool,
        _kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        sqlx::query("SELECT id, `key` FROM task_types")
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column(
                        row, "id",
                    )?,
                    row.try_get("key")?,
                ))
            })
            .collect()
    }
}
