use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{ConfigReferenceKind, ConfigurationReferences};
use sqlx::{MySqlPool, Row};
use std::collections::HashMap;
use uuid::Uuid;
const KINDS: &[ConfigReferenceKind] = &[ConfigReferenceKind::new("segments.segment")];
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
        sqlx::query("SELECT id, CONCAT(record_type, '/', name) AS reference FROM segments")
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column(
                        row, "id",
                    )?,
                    row.try_get("reference")?,
                ))
            })
            .collect()
    }
}
