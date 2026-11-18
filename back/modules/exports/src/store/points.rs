use aphel_sdk::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::points::{FileReferenceSource, GeneratedFileKind};
use async_trait::async_trait;

pub(crate) struct ExportFiles;

#[async_trait]
impl FileReferenceSource for ExportFiles {
    async fn reference_kind(
        &self,
        pool: &Db,
        file_id: &str,
    ) -> Result<Option<GeneratedFileKind>, AppError> {
        let found: Option<i32> =
            sqlx::query_scalar("SELECT 1 FROM export_jobs WHERE file_id = ? LIMIT 1")
                .bind(file_id)
                .fetch_optional(pool)
                .await?;
        Ok(found.map(|_| GeneratedFileKind::RecordExport))
    }
}
