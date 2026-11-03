use bytes::Bytes;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::files::{
    AttachmentView, ListAttachmentsQuery, StoredFile, UploadedFile, UserReference,
};

pub const FILE_FIELD: &str = "file";
pub const RECORD_TYPE_FIELD: &str = "record_type";
pub const RECORD_ID_FIELD: &str = "record_id";

#[derive(Debug, Clone)]
pub struct FormFile {
    pub name: String,
    pub content: Bytes,
}

#[derive(Debug, ToSchema)]
pub struct UploadFileForm {
    #[schema(value_type = String, format = Binary)]
    pub file: FormFile,
    #[schema(example = "people.person")]
    pub record_type: Option<String>,
    pub record_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttachFileRequest {
    pub file_id: String,
    #[schema(example = "people.person")]
    pub record_type: String,
    pub record_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AttachmentsQueryParams {
    #[param(example = "people.person")]
    pub record_type: String,
    #[param(example = "0190a8f2-7c1e-7b3a-9f00-000000000000")]
    pub record_id: String,
}

impl From<AttachmentsQueryParams> for ListAttachmentsQuery {
    fn from(params: AttachmentsQueryParams) -> Self {
        Self {
            record_type: params.record_type,
            record_id: params.record_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserReferenceResponse {
    pub id: String,
    pub display_name: String,
}

impl From<UserReference> for UserReferenceResponse {
    fn from(user: UserReference) -> Self {
        Self {
            id: user.id.into_inner(),
            display_name: user.display_name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FileResponse {
    pub id: String,
    pub original_name: String,
    #[schema(example = "application/pdf")]
    pub mime_type: String,
    pub size: u64,
    pub checksum: String,
    pub uploaded_by: Option<UserReferenceResponse>,
    pub created_at: String,
    #[schema(example = "/api/v1/files/0190a8f2-7c1e-7b3a-9f00-000000000000")]
    pub download_url: String,
}

impl From<StoredFile> for FileResponse {
    fn from(file: StoredFile) -> Self {
        Self {
            download_url: download_url(&file.id.to_string()),
            id: file.id.to_string(),
            original_name: file.original_name,
            mime_type: file.mime_type,
            size: file.size,
            checksum: file.checksum,
            uploaded_by: file.uploaded_by.map(Into::into),
            created_at: timestamp(file.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AttachmentResponse {
    pub id: String,
    pub record_type: String,
    pub record_id: String,
    pub file: FileResponse,
    pub created_by: Option<UserReferenceResponse>,
    pub created_at: String,
}

impl From<AttachmentView> for AttachmentResponse {
    fn from(attachment: AttachmentView) -> Self {
        Self {
            id: attachment.id.to_string(),
            record_type: attachment.record.record_type.as_str().to_owned(),
            record_id: attachment.record.record_id.to_string(),
            file: attachment.file.into(),
            created_by: attachment.created_by.map(Into::into),
            created_at: timestamp(attachment.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadFileResponse {
    pub file: FileResponse,
    pub attachment: Option<AttachmentResponse>,
}

impl From<UploadedFile> for UploadFileResponse {
    fn from(uploaded: UploadedFile) -> Self {
        Self {
            file: uploaded.file.into(),
            attachment: uploaded.attachment.map(Into::into),
        }
    }
}

pub fn download_url(file_id: &str) -> String {
    format!("/api/v1/files/{file_id}")
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}
