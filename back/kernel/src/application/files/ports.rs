use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::files::RecordAccessChecker;
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::Clock;
use crate::application::shared::{
    AuditLog, EventPublisher, FileStorage, RecordRef, TransactionManager,
};
use crate::domain::UserId;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserReference {
    pub id: UserId,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    pub id: Uuid,
    pub storage_key: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub checksum: String,
    pub uploaded_by: Option<UserReference>,
    pub created_at: DateTime<Utc>,
}

impl StoredFile {
    pub fn uploaded_by_id(&self) -> Option<&UserId> {
        self.uploaded_by.as_ref().map(|user| &user.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFile {
    pub id: Uuid,
    pub storage_key: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub checksum: String,
    pub uploaded_by: UserId,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentView {
    pub id: Uuid,
    pub record: RecordRef,
    pub file: StoredFile,
    pub created_by: Option<UserReference>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAttachment {
    pub id: Uuid,
    pub file_id: Uuid,
    pub record: RecordRef,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedFileKind {
    OrganizationExport,
    PrivacyPackage,
    RecordExport,
}

#[async_trait]
pub trait FileRepository: Send + Sync {
    type Tx: Send;

    async fn insert_file(&self, tx: &mut Self::Tx, file: &NewFile) -> Result<(), AppError>;
    async fn find_file(&self, file_id: Uuid) -> Result<Option<StoredFile>, AppError>;
    async fn generated_kind(&self, file_id: Uuid) -> Result<Option<GeneratedFileKind>, AppError>;
    async fn attached_records(&self, file_id: Uuid) -> Result<Vec<RecordRef>, AppError>;
    async fn is_attached(&self, file_id: Uuid, record: &RecordRef) -> Result<bool, AppError>;
    async fn insert_attachment(
        &self,
        tx: &mut Self::Tx,
        attachment: &NewAttachment,
    ) -> Result<(), AppError>;
    async fn find_attachment(
        &self,
        attachment_id: Uuid,
    ) -> Result<Option<AttachmentView>, AppError>;
    async fn list_attachments(
        &self,
        record: &RecordRef,
        window: ListWindow,
    ) -> Result<Listing<AttachmentView>, AppError>;
    async fn delete_attachment(
        &self,
        tx: &mut Self::Tx,
        attachment_id: Uuid,
    ) -> Result<bool, AppError>;
}

pub type FilesTx<P> = <<P as FilePorts>::Transactions as TransactionManager>::Tx;

pub trait FilePorts: Send + Sync {
    type Transactions: TransactionManager;
    type Files: FileRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;
    type Storage: FileStorage + ?Sized;
    type Records: RecordAccessChecker;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn files(&self) -> &Self::Files;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn storage(&self) -> &Self::Storage;
    fn records(&self) -> &Self::Records;
    fn clock(&self) -> &Self::Clock;
}
