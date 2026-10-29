use bytes::Bytes;

use crate::application::shared::RecordRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadFileCommand {
    pub file_name: String,
    pub content: Bytes,
    pub attach_to: Option<RecordRef>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachFileCommand {
    pub file_id: String,
    pub record_type: String,
    pub record_id: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAttachmentsQuery {
    pub record_type: String,
    pub record_id: String,
}
