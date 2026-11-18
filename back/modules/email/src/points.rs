use aphel_sdk::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::points::{FileReferenceSource, GeneratedFileKind};
use async_trait::async_trait;

pub(crate) struct EmailFiles;

#[async_trait]
impl FileReferenceSource for EmailFiles {
    async fn reference_kind(
        &self,
        pool: &Db,
        file_id: &str,
    ) -> Result<Option<GeneratedFileKind>, AppError> {
        let found: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM email_messages WHERE body_file_id = ? \
             UNION ALL SELECT 1 FROM email_attachments ea WHERE ea.file_id = ? \
             AND NOT EXISTS (SELECT 1 FROM attachments ra WHERE ra.file_id = ea.file_id) \
             LIMIT 1",
        )
        .bind(file_id)
        .bind(file_id)
        .fetch_optional(pool)
        .await?;
        Ok(found.map(|_| GeneratedFileKind::RecordExport))
    }
}
