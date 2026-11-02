use aphel_sdk::AppError;
use aphel_sdk::points::{FileReferenceSource, GeneratedFileKind};
use async_trait::async_trait;
use sqlx::MySqlPool;

pub(crate) struct OrganizationExportFiles;

#[async_trait]
impl FileReferenceSource for OrganizationExportFiles {
    async fn reference_kind(
        &self,
        pool: &MySqlPool,
        file_id: &str,
    ) -> Result<Option<GeneratedFileKind>, AppError> {
        let found: Option<i32> =
            sqlx::query_scalar("SELECT 1 FROM organization_exports WHERE file_id = ? LIMIT 1")
                .bind(file_id)
                .fetch_optional(pool)
                .await?;
        Ok(found.map(|_| GeneratedFileKind::OrganizationExport))
    }
}
