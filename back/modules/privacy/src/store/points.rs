use aphel_sdk::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::points::{FileReferenceSource, GeneratedFileKind};
use async_trait::async_trait;

pub(crate) struct PrivacyFiles;

#[async_trait]
impl FileReferenceSource for PrivacyFiles {
    async fn reference_kind(
        &self,
        pool: &Db,
        file_id: &str,
    ) -> Result<Option<GeneratedFileKind>, AppError> {
        let found: Option<i32> =
            sqlx::query_scalar("SELECT 1 FROM privacy_requests WHERE file_id = ? LIMIT 1")
                .bind(file_id)
                .fetch_optional(pool)
                .await?;
        Ok(found.map(|_| GeneratedFileKind::PrivacyPackage))
    }
}
