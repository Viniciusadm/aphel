use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{self, MySqlPool, Row};
use uuid::Uuid;

use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;

pub(crate) const STATUS_SENT: &str = "sent";
pub(crate) const STATUS_COMPLETED: &str = "completed";
pub(crate) const STATUS_REFUSED: &str = "refused";
pub(crate) const STATUS_CANCELLED: &str = "cancelled";
pub(crate) const STATUS_EXPIRED: &str = "expired";
pub(crate) const SIGNER_PENDING: &str = "pending";
pub(crate) const SIGNER_SIGNED: &str = "signed";
pub(crate) const SIGNER_REFUSED: &str = "refused";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestRow {
    pub(crate) id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) file_id: Uuid,
    pub(crate) file_name: String,
    pub(crate) provider: String,
    pub(crate) environment: String,
    pub(crate) provider_envelope_id: String,
    pub(crate) provider_document_id: Option<String>,
    pub(crate) status: String,
    pub(crate) message: Option<String>,
    pub(crate) signed_file_id: Option<Uuid>,
    pub(crate) created_by: Uuid,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
}

impl RequestRow {
    pub(crate) fn is_open(&self) -> bool {
        self.status == STATUS_SENT
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignerRow {
    pub(crate) id: Uuid,
    pub(crate) request_id: Uuid,
    pub(crate) position: u32,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) provider_signer_id: Option<String>,
    pub(crate) status: String,
    pub(crate) signed_at: Option<DateTime<Utc>>,
    pub(crate) refused_at: Option<DateTime<Utc>>,
}

const REQUEST_COLUMNS: &str = "id, deal_id, file_id, file_name, provider, environment, provider_envelope_id, provider_document_id, status, message, signed_file_id, created_by, created_at, updated_at, completed_at";
const SIGNER_COLUMNS: &str = "id, request_id, position, contact_id, name, email, provider_signer_id, status, signed_at, refused_at";

fn uuid(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let raw: String = row.try_get(column)?;
    Uuid::parse_str(&raw).map_err(|error| AppError::Internal(error.to_string()))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let raw: Option<String> = row.try_get(column)?;
    raw.map(|raw| Uuid::parse_str(&raw))
        .transpose()
        .map_err(|error| AppError::Internal(error.to_string()))
}

fn utc(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    let value: NaiveDateTime = row.try_get(column)?;
    Ok(value.and_utc())
}

fn optional_utc(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    let value: Option<NaiveDateTime> = row.try_get(column)?;
    Ok(value.map(|value| value.and_utc()))
}

fn request(row: &MySqlRow) -> Result<RequestRow, AppError> {
    Ok(RequestRow {
        id: uuid(row, "id")?,
        deal_id: uuid(row, "deal_id")?,
        file_id: uuid(row, "file_id")?,
        file_name: row.try_get("file_name")?,
        provider: row.try_get("provider")?,
        environment: row.try_get("environment")?,
        provider_envelope_id: row.try_get("provider_envelope_id")?,
        provider_document_id: row.try_get("provider_document_id")?,
        status: row.try_get("status")?,
        message: row.try_get("message")?,
        signed_file_id: optional_uuid(row, "signed_file_id")?,
        created_by: uuid(row, "created_by")?,
        created_at: utc(row, "created_at")?,
        updated_at: utc(row, "updated_at")?,
        completed_at: optional_utc(row, "completed_at")?,
    })
}

fn signer(row: &MySqlRow) -> Result<SignerRow, AppError> {
    Ok(SignerRow {
        id: uuid(row, "id")?,
        request_id: uuid(row, "request_id")?,
        position: row.try_get("position")?,
        contact_id: optional_uuid(row, "contact_id")?,
        name: row.try_get("name")?,
        email: row.try_get("email")?,
        provider_signer_id: row.try_get("provider_signer_id")?,
        status: row.try_get("status")?,
        signed_at: optional_utc(row, "signed_at")?,
        refused_at: optional_utc(row, "refused_at")?,
    })
}

pub(crate) async fn insert_request(
    tx: &mut MySqlTx,
    row: &RequestRow,
    signers: &[SignerRow],
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO esign_requests (id, deal_id, file_id, file_name, provider, environment, provider_envelope_id, provider_document_id, status, message, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(row.id.to_string())
    .bind(row.deal_id.to_string())
    .bind(row.file_id.to_string())
    .bind(&row.file_name)
    .bind(&row.provider)
    .bind(&row.environment)
    .bind(&row.provider_envelope_id)
    .bind(&row.provider_document_id)
    .bind(&row.status)
    .bind(&row.message)
    .bind(row.created_by.to_string())
    .bind(row.created_at.naive_utc())
    .bind(row.updated_at.naive_utc())
    .execute(&mut **tx)
    .await?;
    for signer in signers {
        sqlx::query(
            "INSERT INTO esign_signers (id, request_id, position, contact_id, name, email, provider_signer_id, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signer.id.to_string())
        .bind(row.id.to_string())
        .bind(signer.position)
        .bind(signer.contact_id.map(|id| id.to_string()))
        .bind(&signer.name)
        .bind(&signer.email)
        .bind(&signer.provider_signer_id)
        .bind(&signer.status)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub(crate) async fn find_request(
    pool: &MySqlPool,
    id: Uuid,
) -> Result<Option<RequestRow>, AppError> {
    let sql = format!("SELECT {REQUEST_COLUMNS} FROM esign_requests WHERE id = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(request)
        .transpose()
}

pub(crate) async fn lock_request(
    tx: &mut MySqlTx,
    id: Uuid,
) -> Result<Option<RequestRow>, AppError> {
    let sql = format!("SELECT {REQUEST_COLUMNS} FROM esign_requests WHERE id = ? FOR UPDATE");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id.to_string())
        .fetch_optional(&mut **tx)
        .await?
        .as_ref()
        .map(request)
        .transpose()
}

pub(crate) async fn find_by_reference(
    pool: &MySqlPool,
    references: &[String],
) -> Result<Option<RequestRow>, AppError> {
    if references.is_empty() {
        return Ok(None);
    }
    let placeholders = vec!["?"; references.len()].join(", ");
    let sql = format!(
        "SELECT {REQUEST_COLUMNS} FROM esign_requests WHERE (provider_envelope_id IN ({placeholders}) OR provider_document_id IN ({placeholders})) ORDER BY created_at DESC LIMIT 1"
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for reference in references.iter().chain(references.iter()) {
        query = query.bind(reference.clone());
    }
    query
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(request)
        .transpose()
}

pub(crate) async fn list_for_deal(
    pool: &MySqlPool,
    deal_id: Uuid,
) -> Result<Vec<RequestRow>, AppError> {
    let sql = format!(
        "SELECT {REQUEST_COLUMNS} FROM esign_requests WHERE deal_id = ? ORDER BY created_at DESC LIMIT 100"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(deal_id.to_string())
        .fetch_all(pool)
        .await?
        .iter()
        .map(request)
        .collect()
}

pub(crate) async fn signers_of(
    pool: &MySqlPool,
    request_ids: &[Uuid],
) -> Result<Vec<SignerRow>, AppError> {
    if request_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; request_ids.len()].join(", ");
    let sql = format!(
        "SELECT {SIGNER_COLUMNS} FROM esign_signers WHERE request_id IN ({placeholders}) ORDER BY position"
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for id in request_ids {
        query = query.bind(id.to_string());
    }
    query.fetch_all(pool).await?.iter().map(signer).collect()
}

pub(crate) async fn signers_in_tx(
    tx: &mut MySqlTx,
    request_id: Uuid,
) -> Result<Vec<SignerRow>, AppError> {
    let sql = format!(
        "SELECT {SIGNER_COLUMNS} FROM esign_signers WHERE request_id = ? ORDER BY position FOR UPDATE"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(request_id.to_string())
        .fetch_all(&mut **tx)
        .await?
        .iter()
        .map(signer)
        .collect()
}

pub(crate) async fn set_request_status(
    tx: &mut MySqlTx,
    id: Uuid,
    status: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let completed_at = (status == STATUS_COMPLETED).then(|| now.naive_utc());
    sqlx::query(
        "UPDATE esign_requests SET status = ?, updated_at = ?, completed_at = COALESCE(?, completed_at) WHERE id = ?",
    )
    .bind(status)
    .bind(now.naive_utc())
    .bind(completed_at)
    .bind(id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn set_signer_status(
    tx: &mut MySqlTx,
    id: Uuid,
    status: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let column = if status == SIGNER_SIGNED {
        "signed_at"
    } else {
        "refused_at"
    };
    let sql = format!("UPDATE esign_signers SET status = ?, {column} = ? WHERE id = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(status)
        .bind(now.naive_utc())
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn set_signed_file(
    tx: &mut MySqlTx,
    id: Uuid,
    file_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("UPDATE esign_requests SET signed_file_id = ?, updated_at = ? WHERE id = ?")
        .bind(file_id.to_string())
        .bind(Utc::now().naive_utc())
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn record_receipt(
    tx: &mut MySqlTx,
    body_hash: &str,
    request_id: Option<Uuid>,
    event_name: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "INSERT IGNORE INTO esign_webhook_receipts (id, body_hash, request_id, event_name) VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(body_hash)
    .bind(request_id.map(|id| id.to_string()))
    .bind(event_name)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn recent_senders(pool: &MySqlPool) -> Result<Vec<Uuid>, AppError> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT created_by FROM esign_requests GROUP BY created_by ORDER BY MAX(created_at) DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .filter_map(|raw| Uuid::parse_str(raw).ok())
        .collect())
}

pub(crate) async fn count_requests(pool: &MySqlPool) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM esign_requests")
        .fetch_one(pool)
        .await?)
}

pub(crate) async fn purge(tx: &mut MySqlTx) -> Result<(), AppError> {
    for sql in [
        "DELETE FROM esign_webhook_receipts",
        "DELETE FROM esign_signers",
        "DELETE FROM esign_requests",
    ] {
        sqlx::query(sql).execute(&mut **tx).await?;
    }
    Ok(())
}
