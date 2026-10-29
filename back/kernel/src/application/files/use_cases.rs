use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::{Authorizer, PERMISSION_DENIED};
use crate::application::files::{
    AttachFileCommand, AttachmentView, DetectedContent, FilePorts, FileRepository, FilesTx,
    GeneratedFileKind, ListAttachmentsQuery, NewAttachment, NewFile, RecordAccessChecker,
    StoredFile, UploadFileCommand, detect_mime_type, sanitize_file_name, sha256_hex, storage_key,
};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, ByteStream, DomainEvent, EventPublisher, FieldChange, FileStorage,
    RecordRef, RecordType, TransactionManager,
};
use crate::domain::access::{AccessScope, Permission};
use crate::errors::AppError;

pub const FILE_NOT_FOUND: &str = "file_not_found";
pub const ATTACHMENT_NOT_FOUND: &str = "attachment_not_found";
pub const FILE_TOO_LARGE: &str = "file_too_large";
pub const EMPTY_FILE: &str = "empty_file";
pub const FILE_TYPE_NOT_ALLOWED: &str = "file_type_not_allowed";
pub const ALREADY_ATTACHED: &str = "already_attached";

pub const FILE_UPLOADED: &str = "file.uploaded";
pub const ATTACHMENT_ADDED: &str = "attachment.added";
pub const ATTACHMENT_REMOVED: &str = "attachment.removed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSettings {
    pub max_upload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadedFile {
    pub file: StoredFile,
    pub attachment: Option<AttachmentView>,
}

pub struct FileDownload {
    pub file: StoredFile,
    pub content: ByteStream,
}

pub struct FileUseCases<P> {
    ports: P,
    settings: FileSettings,
}

pub fn file_too_large(max_upload_bytes: u64) -> AppError {
    AppError::payload_too_large_code(
        FILE_TOO_LARGE,
        format!("the file exceeds the limit of {max_upload_bytes} bytes"),
    )
}

impl<P> FileUseCases<P>
where
    P: FilePorts,
{
    pub fn new(ports: P, settings: FileSettings) -> Self {
        Self { ports, settings }
    }

    pub fn settings(&self) -> FileSettings {
        self.settings
    }

    pub async fn upload(
        &self,
        scope: &AccessScope,
        command: UploadFileCommand,
    ) -> Result<UploadedFile, AppError> {
        if let Some(record) = &command.attach_to {
            self.require_update(scope, record).await?;
        }
        let size = command.content.len() as u64;
        if size == 0 {
            return Err(AppError::validation_code(EMPTY_FILE, "the file is empty"));
        }
        if size > self.settings.max_upload_bytes {
            return Err(file_too_large(self.settings.max_upload_bytes));
        }

        let original_name = sanitize_file_name(&command.file_name);
        let mime_type = match detect_mime_type(&command.content, &original_name) {
            DetectedContent::Allowed(mime_type) => mime_type,
            DetectedContent::Rejected(_) | DetectedContent::Unknown => {
                return Err(AppError::validation_code(
                    FILE_TYPE_NOT_ALLOWED,
                    "this file type is not allowed",
                ));
            }
        };

        let now = self.ports.clock().now();
        let file_id = Uuid::now_v7();
        let file = NewFile {
            id: file_id,
            storage_key: storage_key(file_id, now),
            original_name,
            mime_type,
            size,
            checksum: sha256_hex(&command.content),
            uploaded_by: scope.user_id().clone(),
            created_at: now,
        };

        self.ports
            .storage()
            .put(&file.storage_key, command.content)
            .await?;
        let attachment = match self
            .record_upload(scope, &file, command.attach_to, command.ip)
            .await
        {
            Ok(attachment) => attachment,
            Err(error) => {
                if let Err(cleanup) = self.ports.storage().delete(&file.storage_key).await {
                    tracing::warn!(error = %cleanup, "failed to remove orphan upload");
                }
                return Err(error);
            }
        };

        let stored = self.require_file(file_id).await?;
        let attachment = match attachment {
            Some(attachment) => Some(self.require_attachment(attachment.id).await?),
            None => None,
        };
        Ok(UploadedFile {
            file: stored,
            attachment,
        })
    }

    pub async fn download(
        &self,
        scope: &AccessScope,
        file_id: &str,
    ) -> Result<FileDownload, AppError> {
        let file = self.readable_file(scope, file_id).await?;
        let content = self.ports.storage().get_stream(&file.storage_key).await?;
        Ok(FileDownload { file, content })
    }

    pub async fn list_attachments(
        &self,
        scope: &AccessScope,
        query: ListAttachmentsQuery,
        window: ListWindow,
    ) -> Result<Listing<AttachmentView>, AppError> {
        let record = RecordRef::parse(&query.record_type, &query.record_id)?;
        self.require_read(scope, &record).await?;
        self.ports.files().list_attachments(&record, window).await
    }

    pub async fn attach(
        &self,
        scope: &AccessScope,
        command: AttachFileCommand,
    ) -> Result<AttachmentView, AppError> {
        let record = RecordRef::parse(&command.record_type, &command.record_id)?;
        self.require_update(scope, &record).await?;
        let file = self.readable_file(scope, &command.file_id).await?;
        if self.ports.files().is_attached(file.id, &record).await? {
            return Err(AppError::conflict_code(
                ALREADY_ATTACHED,
                "the file is already attached to this record",
            ));
        }

        let now = self.ports.clock().now();
        let attachment = NewAttachment {
            id: Uuid::now_v7(),
            file_id: file.id,
            record,
            created_by: scope.user_id().clone(),
            created_at: now,
        };
        let mut tx = self.ports.transactions().begin().await?;
        self.add_attachment(
            &mut tx,
            scope,
            &attachment,
            FileSummary::of_stored(&file),
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_attachment(attachment.id).await
    }

    pub async fn remove_attachment(
        &self,
        scope: &AccessScope,
        attachment_id: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let attachment_id = Uuid::parse_str(attachment_id).map_err(|_| attachment_not_found())?;
        let attachment = self
            .ports
            .files()
            .find_attachment(attachment_id)
            .await?
            .ok_or_else(attachment_not_found)?;
        self.require_update(scope, &attachment.record).await?;

        let now = self.ports.clock().now();
        let actor = Actor::from_scope(scope);
        let mut tx = self.ports.transactions().begin().await?;
        if !self
            .ports
            .files()
            .delete_attachment(&mut tx, attachment.id)
            .await?
        {
            return Err(attachment_not_found());
        }
        let payload = attachment_payload(attachment.id, &FileSummary::of_stored(&attachment.file));
        self.ports
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), ATTACHMENT_REMOVED, now)
                    .with_record(attachment.record.clone())
                    .with_payload(payload),
            )
            .await?;
        self.ports
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(actor, ATTACHMENT_REMOVED, now)
                    .with_record(attachment.record.clone())
                    .with_changes(vec![FieldChange::new(
                        "attachment",
                        file_change(attachment.file.id, &attachment.file.original_name),
                        Value::Null,
                    )])
                    .with_ip(ip),
            )
            .await?;
        self.ports.transactions().commit(tx).await
    }

    async fn record_upload(
        &self,
        scope: &AccessScope,
        file: &NewFile,
        attach_to: Option<RecordRef>,
        ip: Option<String>,
    ) -> Result<Option<NewAttachment>, AppError> {
        let actor = Actor::from_scope(scope);
        let file_record = RecordRef::new(RecordType::FILE, file.id);
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.files().insert_file(&mut tx, file).await?;
        self.ports
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), FILE_UPLOADED, file.created_at)
                    .with_record(file_record.clone())
                    .with_payload(json!({
                        "file_id": file.id.to_string(),
                        "file_name": file.original_name,
                        "mime_type": file.mime_type,
                        "size": file.size,
                        "checksum": file.checksum,
                    })),
            )
            .await?;
        self.ports
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(actor, FILE_UPLOADED, file.created_at)
                    .with_record(file_record)
                    .with_changes(vec![FieldChange::new(
                        "file",
                        Value::Null,
                        file_change(file.id, &file.original_name),
                    )])
                    .with_ip(ip.clone()),
            )
            .await?;

        let attachment = match attach_to {
            Some(record) => {
                let attachment = NewAttachment {
                    id: Uuid::now_v7(),
                    file_id: file.id,
                    record,
                    created_by: file.uploaded_by.clone(),
                    created_at: file.created_at,
                };
                self.add_attachment(&mut tx, scope, &attachment, FileSummary::of_new(file), ip)
                    .await?;
                Some(attachment)
            }
            None => None,
        };
        self.ports.transactions().commit(tx).await?;
        Ok(attachment)
    }

    async fn add_attachment(
        &self,
        tx: &mut FilesTx<P>,
        scope: &AccessScope,
        attachment: &NewAttachment,
        file: FileSummary<'_>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let actor = Actor::from_scope(scope);
        self.ports.files().insert_attachment(tx, attachment).await?;
        self.ports
            .events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), ATTACHMENT_ADDED, attachment.created_at)
                    .with_record(attachment.record.clone())
                    .with_payload(attachment_payload(attachment.id, &file)),
            )
            .await?;
        self.ports
            .audit()
            .record(
                tx,
                AuditEntry::new(actor, ATTACHMENT_ADDED, attachment.created_at)
                    .with_record(attachment.record.clone())
                    .with_changes(vec![FieldChange::new(
                        "attachment",
                        Value::Null,
                        file_change(file.id, file.name),
                    )])
                    .with_ip(ip),
            )
            .await
    }

    async fn readable_file(
        &self,
        scope: &AccessScope,
        file_id: &str,
    ) -> Result<StoredFile, AppError> {
        let file_id = Uuid::parse_str(file_id).map_err(|_| file_not_found())?;
        let file = self
            .ports
            .files()
            .find_file(file_id)
            .await?
            .ok_or_else(file_not_found)?;
        match self.ports.files().generated_kind(file.id).await? {
            Some(GeneratedFileKind::OrganizationExport | GeneratedFileKind::PrivacyPackage) => {
                return Err(file_not_found());
            }
            Some(GeneratedFileKind::RecordExport) => {
                return if file.uploaded_by_id() == Some(scope.user_id()) {
                    Ok(file)
                } else {
                    Err(file_not_found())
                };
            }
            None => {}
        }
        if file.uploaded_by_id() == Some(scope.user_id())
            || scope.require(&Permission::SETTINGS_MANAGE).is_ok()
        {
            return Ok(file);
        }
        for record in self.ports.files().attached_records(file.id).await? {
            if self
                .ports
                .records()
                .can_read(scope, &record)
                .await
                .unwrap_or(false)
            {
                return Ok(file);
            }
        }
        Err(file_not_found())
    }

    async fn require_read(&self, scope: &AccessScope, record: &RecordRef) -> Result<(), AppError> {
        if self.ports.records().can_read(scope, record).await? {
            return Ok(());
        }
        Err(record_denied(record))
    }

    async fn require_update(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        if self.ports.records().can_update(scope, record).await? {
            return Ok(());
        }
        Err(record_denied(record))
    }

    async fn require_file(&self, file_id: Uuid) -> Result<StoredFile, AppError> {
        self.ports
            .files()
            .find_file(file_id)
            .await?
            .ok_or_else(file_not_found)
    }

    async fn require_attachment(&self, attachment_id: Uuid) -> Result<AttachmentView, AppError> {
        self.ports
            .files()
            .find_attachment(attachment_id)
            .await?
            .ok_or_else(attachment_not_found)
    }
}

fn file_not_found() -> AppError {
    AppError::not_found_code(FILE_NOT_FOUND, "file was not found")
}

fn attachment_not_found() -> AppError {
    AppError::not_found_code(ATTACHMENT_NOT_FOUND, "attachment was not found")
}

fn record_denied(record: &RecordRef) -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        format!(
            "missing permission to access {} records",
            record.record_type
        ),
    )
}

fn file_change(file_id: Uuid, file_name: &str) -> Value {
    json!({ "file_id": file_id.to_string(), "file_name": file_name })
}

struct FileSummary<'a> {
    id: Uuid,
    name: &'a str,
    mime_type: &'a str,
    size: u64,
}

impl<'a> FileSummary<'a> {
    fn of_new(file: &'a NewFile) -> Self {
        Self {
            id: file.id,
            name: &file.original_name,
            mime_type: &file.mime_type,
            size: file.size,
        }
    }

    fn of_stored(file: &'a StoredFile) -> Self {
        Self {
            id: file.id,
            name: &file.original_name,
            mime_type: &file.mime_type,
            size: file.size,
        }
    }
}

fn attachment_payload(attachment_id: Uuid, file: &FileSummary<'_>) -> Value {
    json!({
        "attachment_id": attachment_id.to_string(),
        "file_id": file.id.to_string(),
        "file_name": file.name,
        "mime_type": file.mime_type,
        "size": file.size,
    })
}
