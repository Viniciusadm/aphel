use bytes::Bytes;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::application::files::{
    FileRepository, NewFile, sanitize_file_name, sha256_hex, storage_key,
};
use aphel_kernel::application::shared::FileStorage;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub mime_type: String,
    pub content: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredGeneratedFile {
    pub file_id: Uuid,
    pub storage_key: String,
}

pub async fn store_generated_file<F, S>(
    files: &F,
    storage: &S,
    tx: &mut F::Tx,
    scope: &AccessScope,
    file: GeneratedFile,
    now: DateTime<Utc>,
) -> Result<StoredGeneratedFile, AppError>
where
    F: FileRepository,
    S: FileStorage + ?Sized,
{
    let file_id = Uuid::now_v7();
    let record = NewFile {
        id: file_id,
        storage_key: storage_key(file_id, now),
        original_name: sanitize_file_name(&file.name),
        mime_type: file.mime_type,
        size: file.content.len() as u64,
        checksum: sha256_hex(&file.content),
        uploaded_by: scope.user_id().clone(),
        created_at: now,
    };
    storage.put(&record.storage_key, file.content).await?;
    if let Err(error) = files.insert_file(tx, &record).await {
        discard_generated_file(storage, &record.storage_key).await;
        return Err(error);
    }
    Ok(StoredGeneratedFile {
        file_id,
        storage_key: record.storage_key,
    })
}

pub async fn discard_generated_file<S>(storage: &S, storage_key: &str)
where
    S: FileStorage + ?Sized,
{
    if let Err(error) = storage.delete(storage_key).await {
        tracing::warn!(error = %error, "failed to remove an orphan generated file");
    }
}

pub async fn read_file_content<S>(storage: &S, storage_key: &str) -> Result<Bytes, AppError>
where
    S: FileStorage + ?Sized,
{
    use futures::StreamExt;

    let mut stream = storage.get_stream(storage_key).await?;
    let mut content = Vec::new();
    while let Some(chunk) = stream.next().await {
        content.extend_from_slice(&chunk?);
    }
    Ok(Bytes::from(content))
}
