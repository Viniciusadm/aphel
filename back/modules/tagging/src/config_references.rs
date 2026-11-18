use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{
    ConfigKeyClient, ConfigKeyRepository, ConfigReferenceKind, ConfigurationReferences, assign_keys,
};
use aphel_sdk::platform::database::Db;
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

const KINDS: &[ConfigReferenceKind] = &[ConfigReferenceKind::new("tagging.tag")];

pub(crate) struct References;

#[async_trait::async_trait]
impl ConfigurationReferences for References {
    fn kinds(&self) -> &'static [ConfigReferenceKind] {
        KINDS
    }
    fn package_keys(
        &self,
        package: &aphel_sdk::configuration::ConfigPackage,
        _kind: ConfigReferenceKind,
    ) -> std::collections::BTreeSet<String> {
        package
            .section(crate::section::TAGS_SECTION)
            .and_then(|content| {
                aphel_sdk::configuration::parse_section::<Vec<crate::section::PackageTag>>(content)
                    .ok()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|entry| entry.key)
            .collect()
    }
    async fn keys(
        &self,
        pool: &Db,
        kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let rows = sqlx::query("SELECT id, name FROM tags ORDER BY created_at, id")
            .fetch_all(pool)
            .await?;
        let tags: Vec<(Uuid, String)> = rows
            .iter()
            .map(|row| {
                Ok((
                    aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column(
                        row, "id",
                    )?,
                    row.try_get("name")?,
                ))
            })
            .collect::<Result<_, AppError>>()?;
        let stored = ConfigKeyClient::new(pool.clone()).keys(kind).await?;
        Ok(
            assign_keys(tags.iter().map(|(id, name)| (*id, name.as_str())), &stored)
                .into_iter()
                .map(|key| (key.id, key.key))
                .collect(),
        )
    }
}
