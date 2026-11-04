use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{ConfigReferenceKind, ConfigurationReferences};
use sqlx::{MySqlPool, Row};
use std::collections::HashMap;
use uuid::Uuid;
const KINDS: &[ConfigReferenceKind] = &[ConfigReferenceKind::new("sequences.sequence")];
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
        sqlx::query("SELECT id, `key` AS reference FROM sequences")
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column(
                        row, "id",
                    )?,
                    row.try_get("reference")?,
                ))
            })
            .collect()
    }
    fn package_keys(
        &self,
        package: &aphel_sdk::configuration::ConfigPackage,
        _kind: ConfigReferenceKind,
    ) -> std::collections::BTreeSet<String> {
        package
            .section(crate::section::SEQUENCES_SECTION)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.get("key").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect()
    }
}
