use bytes::Bytes;
use futures::TryStreamExt;
use serde_json::json;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::UploadFileCommand;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::domain::access::AccessScope;

const MAX_READ_BYTES: usize = 10 * 1024 * 1024;

fn file_json(file: &aphel_sdk::platform::application::files::StoredFile) -> serde_json::Value {
    json!({
        "id": file.id.to_string(),
        "name": file.original_name,
        "content_type": file.mime_type,
        "size": file.size,
        "checksum": file.checksum,
        "created_at": file.created_at.to_rfc3339(),
        "download_path": format!("/api/v1/files/{}/download", file.id),
    })
}

pub async fn upload(
    scope: &AccessScope,
    record_type: &str,
    record_id: &str,
    filename: String,
    body: Vec<u8>,
) -> Result<String, AppError> {
    let state = aphel_sdk::runtime()?;
    let uploaded = aphel_sdk::platform::api::routes::v1::files::file_use_cases(&state)
        .await?
        .upload(
            scope,
            UploadFileCommand {
                file_name: filename,
                content: Bytes::from(body),
                attach_to: Some(RecordRef::parse(record_type, record_id)?),
                ip: None,
            },
        )
        .await?;
    Ok(file_json(&uploaded.file).to_string())
}

pub async fn read(scope: &AccessScope, file_id: &str) -> Result<(String, Vec<u8>), AppError> {
    let state = aphel_sdk::runtime()?;
    let download = aphel_sdk::platform::api::routes::v1::files::file_use_cases(&state)
        .await?
        .download(scope, file_id)
        .await?;
    if download.file.size as usize > MAX_READ_BYTES {
        return Err(AppError::payload_too_large_code(
            "plugin_file_too_large",
            format!("plugins can read files of up to {MAX_READ_BYTES} bytes"),
        ));
    }
    let mut content = Vec::with_capacity(download.file.size as usize);
    let mut stream = download.content;
    while let Some(chunk) = stream.try_next().await? {
        if content.len() + chunk.len() > MAX_READ_BYTES {
            return Err(AppError::payload_too_large_code(
                "plugin_file_too_large",
                format!("plugins can read files of up to {MAX_READ_BYTES} bytes"),
            ));
        }
        content.extend_from_slice(&chunk);
    }
    Ok((file_json(&download.file).to_string(), content))
}
