use crate::modules::points::FileReferenceSource;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::mysql::MySqlRow;
use sqlx::{AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use crate::application::fields::RecordTypeRegistry;
use crate::application::files::{
    ALREADY_ATTACHED, AttachmentView, FILE_NOT_FOUND, FilePorts, FileRepository, GeneratedFileKind,
    NewAttachment, NewFile, StoredFile, UserReference,
};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::shared::{AuditLog, EventPublisher, FileStorage, RecordRef};
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::records::MySqlRecordAccessChecker;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::transactions::MySqlTransactionManager;

const FILE_COLUMNS: &str = "f.id AS file_id, f.storage_key, f.original_name, f.mime_type, f.size, \
     f.checksum, f.uploaded_by, fu.display_name AS uploaded_by_name, f.created_at AS file_created_at";

#[derive(Clone)]
pub struct MySqlFileRepository {
    pool: MySqlPool,
}

impl MySqlFileRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn user_reference(
    row: &MySqlRow,
    id_column: &str,
    name_column: &str,
) -> Result<Option<UserReference>, AppError> {
    let id: Option<String> = row.try_get(id_column)?;
    let name: Option<String> = row.try_get(name_column)?;
    id.map(|id| {
        Ok(UserReference {
            id: mapper::typed_id(id, UserId::new)?,
            display_name: name.unwrap_or_default(),
        })
    })
    .transpose()
}

fn file_from_row(row: &MySqlRow) -> Result<StoredFile, AppError> {
    let created_at: NaiveDateTime = row.try_get("file_created_at")?;
    Ok(StoredFile {
        id: uuid_column(row, "file_id")?,
        storage_key: row.try_get("storage_key")?,
        original_name: row.try_get("original_name")?,
        mime_type: row.try_get("mime_type")?,
        size: row.try_get("size")?,
        checksum: row.try_get("checksum")?,
        uploaded_by: user_reference(row, "uploaded_by", "uploaded_by_name")?,
        created_at: mapper::utc(created_at),
    })
}

fn attachment_from_row(row: &MySqlRow) -> Result<AttachmentView, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let record_id: String = row.try_get("record_id")?;
    let created_at: NaiveDateTime = row.try_get("attached_at")?;
    Ok(AttachmentView {
        id: uuid_column(row, "attachment_id")?,
        record: RecordRef::parse(&record_type, &record_id)
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        file: file_from_row(row)?,
        created_by: user_reference(row, "created_by", "created_by_name")?,
        created_at: mapper::utc(created_at),
    })
}

fn attachment_query(filter: &str) -> String {
    format!(
        "SELECT a.id AS attachment_id, a.record_type, a.record_id, a.created_by, \
         cu.display_name AS created_by_name, a.created_at AS attached_at, {FILE_COLUMNS} \
         FROM attachments a \
         JOIN files f ON f.id = a.file_id AND f.deleted_at IS NULL \
         LEFT JOIN users fu ON fu.id = f.uploaded_by \
         LEFT JOIN users cu ON cu.id = a.created_by \
         WHERE 1 = 1 {filter}"
    )
}

#[async_trait]
impl FileRepository for MySqlFileRepository {
    type Tx = MySqlTx;

    async fn insert_file(&self, tx: &mut MySqlTx, file: &NewFile) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO files (id, storage_key, original_name, mime_type, size, checksum, uploaded_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(file.id.to_string())
        .bind(&file.storage_key)
        .bind(&file.original_name)
        .bind(&file.mime_type)
        .bind(file.size)
        .bind(&file.checksum)
        .bind(file.uploaded_by.as_str())
        .bind(mapper::naive(file.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn find_file(&self, file_id: Uuid) -> Result<Option<StoredFile>, AppError> {
        let row = sqlx::query(AssertSqlSafe(format!(
            "SELECT {FILE_COLUMNS} FROM files f LEFT JOIN users fu ON fu.id = f.uploaded_by \
             WHERE f.id = ? AND f.deleted_at IS NULL"
        )))
        .bind(file_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(file_from_row).transpose()
    }

    async fn generated_kind(&self, file_id: Uuid) -> Result<Option<GeneratedFileKind>, AppError> {
        let file_id = file_id.to_string();
        for source in crate::modules::app().contributions::<dyn FileReferenceSource>() {
            if let Some(kind) = source.reference_kind(&self.pool, &file_id).await? {
                return Ok(Some(kind));
            }
        }
        Ok(None)
    }

    async fn attached_records(&self, file_id: Uuid) -> Result<Vec<RecordRef>, AppError> {
        let rows = sqlx::query(
            "SELECT record_type, record_id FROM attachments \
             WHERE file_id = ? ORDER BY created_at",
        )
        .bind(file_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let record_type: String = row.try_get("record_type")?;
                let record_id: String = row.try_get("record_id")?;
                RecordRef::parse(&record_type, &record_id)
                    .map_err(|_| mapper::invalid_column("record_type", &record_type))
            })
            .collect()
    }

    async fn is_attached(&self, file_id: Uuid, record: &RecordRef) -> Result<bool, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM attachments \
             WHERE file_id = ? AND record_type = ? AND record_id = ?)",
        )
        .bind(file_id.to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn insert_attachment(
        &self,
        tx: &mut MySqlTx,
        attachment: &NewAttachment,
    ) -> Result<(), AppError> {
        let inserted = sqlx::query(
            "INSERT INTO attachments (id, file_id, record_type, record_id, created_by, created_at) \
             SELECT ?, f.id, ?, ?, ?, ? FROM files f \
             WHERE f.id = ? AND f.deleted_at IS NULL",
        )
        .bind(attachment.id.to_string())
        .bind(attachment.record.record_type.as_str())
        .bind(attachment.record.record_id.to_string())
        .bind(attachment.created_by.as_str())
        .bind(mapper::naive(attachment.created_at))
        .bind(attachment.file_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(database) if database.is_unique_violation() => {
                AppError::conflict_code(
                    ALREADY_ATTACHED,
                    "the file is already attached to this record",
                )
            }
            _ => AppError::from(error),
        })?;
        if inserted.rows_affected() != 1 {
            return Err(AppError::not_found_code(
                FILE_NOT_FOUND,
                "file was not found",
            ));
        }
        Ok(())
    }

    async fn find_attachment(
        &self,
        attachment_id: Uuid,
    ) -> Result<Option<AttachmentView>, AppError> {
        let row = sqlx::query(AssertSqlSafe(attachment_query("AND a.id = ?")))
            .bind(attachment_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(attachment_from_row).transpose()
    }

    async fn list_attachments(
        &self,
        record: &RecordRef,
        window: ListWindow,
    ) -> Result<Listing<AttachmentView>, AppError> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM attachments a \
             JOIN files f ON f.id = a.file_id AND f.deleted_at IS NULL \
             WHERE a.record_type = ? AND a.record_id = ?",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query(AssertSqlSafe(attachment_query(
            "AND a.record_type = ? AND a.record_id = ? ORDER BY a.created_at DESC, a.id DESC LIMIT ? OFFSET ?",
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(attachment_from_row)
                .collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn delete_attachment(
        &self,
        tx: &mut MySqlTx,
        attachment_id: Uuid,
    ) -> Result<bool, AppError> {
        let deleted = sqlx::query("DELETE FROM attachments WHERE id = ?")
            .bind(attachment_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(deleted.rows_affected() == 1)
    }
}

pub struct MySqlFilePorts {
    transactions: MySqlTransactionManager,
    files: MySqlFileRepository,
    events: Arc<dyn EventPublisher<MySqlTx>>,
    audit: Arc<dyn AuditLog<MySqlTx>>,
    storage: Arc<dyn FileStorage>,
    records: MySqlRecordAccessChecker,
}

impl MySqlFilePorts {
    pub fn new(
        pool: MySqlPool,
        storage: Arc<dyn FileStorage>,
        events: Arc<dyn EventPublisher<MySqlTx>>,
        audit: Arc<dyn AuditLog<MySqlTx>>,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            records: MySqlRecordAccessChecker::new(
                pool.clone(),
                Arc::new(RecordTypeRegistry::builtin()),
            ),
            files: MySqlFileRepository::new(pool),
            events,
            audit,
            storage,
        }
    }

    pub fn with_records(mut self, records: MySqlRecordAccessChecker) -> Self {
        self.records = records;
        self
    }
}

impl FilePorts for MySqlFilePorts {
    type Transactions = MySqlTransactionManager;
    type Files = MySqlFileRepository;
    type Events = Arc<dyn EventPublisher<MySqlTx>>;
    type Audit = Arc<dyn AuditLog<MySqlTx>>;
    type Storage = dyn FileStorage;
    type Records = MySqlRecordAccessChecker;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn files(&self) -> &Self::Files {
        &self.files
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn storage(&self) -> &Self::Storage {
        self.storage.as_ref()
    }

    fn records(&self) -> &Self::Records {
        &self.records
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
