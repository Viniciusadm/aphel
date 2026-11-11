use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{PublicationRequest, Revision, Subject};
use aphel_contract_publication::{RequestAction, RequestStatus};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{RecordRef, RecordType};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper::{invalid_column, naive, utc};

const REVISION_COLUMNS: &str = "SELECT id, record_type, record_id, number, format, payload, \
     payload_hash, draft_version, note, created_by, created_at FROM publication_revisions WHERE TRUE";

const REVISION_SUMMARY_COLUMNS: &str = "SELECT id, record_type, record_id, number, format, \
     JSON_OBJECT() AS payload, payload_hash, draft_version, note, created_by, created_at, \
     JSON_UNQUOTE(JSON_EXTRACT(payload, ";
const REVISION_SUMMARY_FROM: &str = ")) AS title FROM publication_revisions WHERE TRUE";

const REQUEST_COLUMNS: &str = "SELECT id, record_type, record_id, action, revision_id, generation, \
     expected_active_revision_id, status, run_at, requested_by, idempotency_key, input_hash, \
     attempts, reason_code, created_at, completed_at FROM publication_requests WHERE TRUE";

fn uuid_of(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| invalid_column(column, &value))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| Uuid::parse_str(&value).map_err(|_| invalid_column(column, &value)))
        .transpose()
}

fn record_of(row: &MySqlRow) -> Result<RecordRef, AppError> {
    let record_type: String = row.try_get("record_type")?;
    Ok(RecordRef::new(
        RecordType::parse(record_type)?,
        uuid_of(row, "record_id")?,
    ))
}

fn optional_time(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row.try_get::<Option<NaiveDateTime>, _>(column)?.map(utc))
}

fn revision_from_row(row: &MySqlRow) -> Result<Revision, AppError> {
    let Json(payload): Json<Value> = row.try_get("payload")?;
    Ok(Revision {
        id: uuid_of(row, "id")?,
        record: record_of(row)?,
        number: row.try_get("number")?,
        format: row.try_get("format")?,
        payload,
        payload_hash: row.try_get("payload_hash")?,
        draft_version: utc(row.try_get("draft_version")?),
        note: row.try_get("note")?,
        created_by: row.try_get("created_by")?,
        created_at: utc(row.try_get("created_at")?),
        title: row
            .try_get::<Option<String>, _>("title")
            .ok()
            .flatten()
            .filter(|title| !title.is_empty() && title != "null"),
    })
}

fn request_from_row(row: &MySqlRow) -> Result<PublicationRequest, AppError> {
    let action: String = row.try_get("action")?;
    let status: String = row.try_get("status")?;
    Ok(PublicationRequest {
        id: uuid_of(row, "id")?,
        record: record_of(row)?,
        action: RequestAction::parse(&action)
            .ok_or_else(|| invalid_column("publication_requests.action", &action))?,
        revision_id: optional_uuid(row, "revision_id")?,
        generation: row.try_get("generation")?,
        expected_active_revision_id: optional_uuid(row, "expected_active_revision_id")?,
        status: RequestStatus::parse(&status)
            .ok_or_else(|| invalid_column("publication_requests.status", &status))?,
        run_at: optional_time(row, "run_at")?,
        requested_by: row.try_get("requested_by")?,
        idempotency_key: row.try_get("idempotency_key")?,
        input_hash: row.try_get("input_hash")?,
        attempts: row.try_get("attempts")?,
        reason_code: row.try_get("reason_code")?,
        created_at: utc(row.try_get("created_at")?),
        completed_at: optional_time(row, "completed_at")?,
    })
}

pub(crate) async fn lock_subject(
    tx: &mut MySqlTx,
    record: &RecordRef,
) -> Result<Subject, AppError> {
    sqlx::query("INSERT IGNORE INTO publication_subjects (record_type, record_id) VALUES (?, ?)")
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .execute(&mut **tx)
        .await?;
    let row = sqlx::query(
        "SELECT active_revision_id, generation, first_published_at FROM publication_subjects \
         WHERE record_type = ? AND record_id = ? FOR UPDATE",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .fetch_one(&mut **tx)
    .await?;
    Ok(Subject {
        active_revision_id: optional_uuid(&row, "active_revision_id")?,
        generation: row.try_get("generation")?,
        first_published_at: optional_time(&row, "first_published_at")?,
    })
}

pub(crate) async fn subject(
    pool: &MySqlPool,
    record: &RecordRef,
) -> Result<Option<Subject>, AppError> {
    let row = sqlx::query(
        "SELECT active_revision_id, generation, first_published_at FROM publication_subjects \
         WHERE record_type = ? AND record_id = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(Subject {
            active_revision_id: optional_uuid(&row, "active_revision_id")?,
            generation: row.try_get("generation")?,
            first_published_at: optional_time(&row, "first_published_at")?,
        })
    })
    .transpose()
}

pub(crate) async fn bump_generation(
    tx: &mut MySqlTx,
    record: &RecordRef,
    at: DateTime<Utc>,
) -> Result<u64, AppError> {
    sqlx::query(
        "UPDATE publication_subjects SET generation = generation + 1, updated_at = ? \
         WHERE record_type = ? AND record_id = ?",
    )
    .bind(naive(at))
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(sqlx::query_scalar(
        "SELECT generation FROM publication_subjects WHERE record_type = ? AND record_id = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .fetch_one(&mut **tx)
    .await?)
}

pub(crate) async fn set_active(
    tx: &mut MySqlTx,
    record: &RecordRef,
    revision_id: Option<Uuid>,
    first_published_at: Option<DateTime<Utc>>,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE publication_subjects SET active_revision_id = ?, \
         first_published_at = COALESCE(first_published_at, ?), updated_at = ? \
         WHERE record_type = ? AND record_id = ?",
    )
    .bind(revision_id.map(|id| id.to_string()))
    .bind(first_published_at.map(naive))
    .bind(naive(at))
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn next_revision_number(
    tx: &mut MySqlTx,
    record: &RecordRef,
) -> Result<u32, AppError> {
    let current: Option<u32> = sqlx::query_scalar(
        "SELECT MAX(number) FROM publication_revisions WHERE record_type = ? AND record_id = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .fetch_one(&mut **tx)
    .await?;
    Ok(current.unwrap_or(0) + 1)
}

pub(crate) async fn insert_revision(
    tx: &mut MySqlTx,
    revision: &Revision,
    files: &[String],
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO publication_revisions (id, record_type, record_id, number, format, payload, \
         payload_hash, draft_version, note, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(revision.id.to_string())
    .bind(revision.record.record_type.as_str())
    .bind(revision.record.record_id.to_string())
    .bind(revision.number)
    .bind(revision.format)
    .bind(Json(&revision.payload))
    .bind(&revision.payload_hash)
    .bind(naive(revision.draft_version))
    .bind(&revision.note)
    .bind(&revision.created_by)
    .bind(naive(revision.created_at))
    .execute(&mut **tx)
    .await?;
    if files.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "INSERT IGNORE INTO publication_revision_files (revision_id, file_id) ",
    );
    builder.push_values(files, |mut row, file| {
        row.push_bind(revision.id.to_string())
            .push_bind(file.clone());
    });
    builder.build().execute(&mut **tx).await?;
    Ok(())
}

pub(crate) async fn revision(pool: &MySqlPool, id: Uuid) -> Result<Option<Revision>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REVISION_COLUMNS);
    builder.push(" AND id = ").push_bind(id.to_string());
    let row = builder.build().fetch_optional(pool).await?;
    row.as_ref().map(revision_from_row).transpose()
}

pub(crate) async fn revision_in(tx: &mut MySqlTx, id: Uuid) -> Result<Option<Revision>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REVISION_COLUMNS);
    builder.push(" AND id = ").push_bind(id.to_string());
    let row = builder.build().fetch_optional(&mut **tx).await?;
    row.as_ref().map(revision_from_row).transpose()
}

pub(crate) async fn revisions(
    pool: &MySqlPool,
    record: &RecordRef,
    title_field: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<(Vec<Revision>, u64), AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REVISION_SUMMARY_COLUMNS);
    builder
        .push_bind(match title_field {
            Some(field) => format!("$.fields.\"{field}\""),
            None => "$.title_unavailable".to_owned(),
        })
        .push(REVISION_SUMMARY_FROM)
        .push(" AND record_type = ")
        .push_bind(record.record_type.as_str().to_owned())
        .push(" AND record_id = ")
        .push_bind(record.record_id.to_string())
        .push(" ORDER BY number DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows = builder.build().fetch_all(pool).await?;
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM publication_revisions WHERE record_type = ? AND record_id = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok((
        rows.iter()
            .map(revision_from_row)
            .collect::<Result<_, _>>()?,
        u64::try_from(total).unwrap_or_default(),
    ))
}

pub(crate) async fn insert_request(
    tx: &mut MySqlTx,
    request: &PublicationRequest,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO publication_requests (id, record_type, record_id, action, revision_id, \
         generation, expected_active_revision_id, status, run_at, requested_by, idempotency_key, \
         input_hash, attempts, reason_code, created_at, completed_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(request.id.to_string())
    .bind(request.record.record_type.as_str())
    .bind(request.record.record_id.to_string())
    .bind(request.action.as_str())
    .bind(request.revision_id.map(|id| id.to_string()))
    .bind(request.generation)
    .bind(request.expected_active_revision_id.map(|id| id.to_string()))
    .bind(request.status.as_str())
    .bind(request.run_at.map(naive))
    .bind(&request.requested_by)
    .bind(&request.idempotency_key)
    .bind(&request.input_hash)
    .bind(request.attempts)
    .bind(&request.reason_code)
    .bind(naive(request.created_at))
    .bind(request.completed_at.map(naive))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn lock_request(
    tx: &mut MySqlTx,
    id: Uuid,
) -> Result<Option<PublicationRequest>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder
        .push(" AND id = ")
        .push_bind(id.to_string())
        .push(" FOR UPDATE");
    let row = builder.build().fetch_optional(&mut **tx).await?;
    row.as_ref().map(request_from_row).transpose()
}

pub(crate) async fn request(
    pool: &MySqlPool,
    id: Uuid,
) -> Result<Option<PublicationRequest>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder.push(" AND id = ").push_bind(id.to_string());
    let row = builder.build().fetch_optional(pool).await?;
    row.as_ref().map(request_from_row).transpose()
}

pub(crate) async fn request_by_key(
    tx: &mut MySqlTx,
    requested_by: &str,
    key: &str,
) -> Result<Option<PublicationRequest>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder
        .push(" AND requested_by = ")
        .push_bind(requested_by.to_owned())
        .push(" AND idempotency_key = ")
        .push_bind(key.to_owned())
        .push(" FOR UPDATE");
    let row = builder.build().fetch_optional(&mut **tx).await?;
    row.as_ref().map(request_from_row).transpose()
}

pub(crate) async fn open_requests(
    tx: &mut MySqlTx,
    record: &RecordRef,
) -> Result<Vec<PublicationRequest>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder
        .push(" AND record_type = ")
        .push_bind(record.record_type.as_str().to_owned())
        .push(" AND record_id = ")
        .push_bind(record.record_id.to_string())
        .push(" AND status IN ('awaiting_approval', 'scheduled', 'ready') ORDER BY created_at, id FOR UPDATE");
    let rows = builder.build().fetch_all(&mut **tx).await?;
    rows.iter().map(request_from_row).collect()
}

pub(crate) async fn latest_requests(
    pool: &MySqlPool,
    record: &RecordRef,
    limit: u32,
) -> Result<Vec<PublicationRequest>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder
        .push(" AND record_type = ")
        .push_bind(record.record_type.as_str().to_owned())
        .push(" AND record_id = ")
        .push_bind(record.record_id.to_string())
        .push(" ORDER BY created_at DESC, id DESC LIMIT ")
        .push_bind(limit);
    let rows = builder.build().fetch_all(pool).await?;
    rows.iter().map(request_from_row).collect()
}

pub(crate) async fn awaiting_for_revision(
    tx: &mut MySqlTx,
    record: &RecordRef,
    revision_id: Uuid,
) -> Result<Vec<PublicationRequest>, AppError> {
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder
        .push(" AND record_type = ")
        .push_bind(record.record_type.as_str().to_owned())
        .push(" AND record_id = ")
        .push_bind(record.record_id.to_string())
        .push(" AND revision_id = ")
        .push_bind(revision_id.to_string())
        .push(" AND status = 'awaiting_approval' FOR UPDATE");
    let rows = builder.build().fetch_all(&mut **tx).await?;
    rows.iter().map(request_from_row).collect()
}

pub(crate) async fn save_request(
    tx: &mut MySqlTx,
    request: &PublicationRequest,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE publication_requests SET status = ?, run_at = ?, attempts = ?, reason_code = ?, \
         completed_at = ? WHERE id = ?",
    )
    .bind(request.status.as_str())
    .bind(request.run_at.map(naive))
    .bind(request.attempts)
    .bind(&request.reason_code)
    .bind(request.completed_at.map(naive))
    .bind(request.id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn collectable_revisions(
    pool: &MySqlPool,
    before: DateTime<Utc>,
    limit: u32,
) -> Result<Vec<(RecordRef, Uuid)>, AppError> {
    let rows = sqlx::query(
        "SELECT r.id, r.record_type, r.record_id FROM publication_revisions r \
         WHERE r.created_at < ? \
         AND NOT EXISTS (SELECT 1 FROM publication_subjects s WHERE s.active_revision_id = r.id) \
         AND NOT EXISTS (SELECT 1 FROM publication_requests q WHERE q.revision_id = r.id \
           AND q.status IN ('awaiting_approval', 'scheduled', 'ready', 'failed')) \
         AND (SELECT COUNT(*) FROM publication_revisions n WHERE n.record_type = r.record_type \
           AND n.record_id = r.record_id AND n.number > r.number) >= ? \
         ORDER BY r.created_at LIMIT ?",
    )
    .bind(naive(before))
    .bind(super::domain::RETAIN_COUNT)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| Ok((record_of(row)?, uuid_of(row, "id")?)))
        .collect()
}

pub(crate) async fn delete_revision(tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM publication_revisions WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn subjects_of(
    pool: &MySqlPool,
    record_type: &str,
    ids: &[Uuid],
) -> Result<Vec<(Uuid, Subject)>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT record_id, active_revision_id, generation, first_published_at FROM publication_subjects WHERE record_type = ",
    );
    builder
        .push_bind(record_type.to_owned())
        .push(" AND record_id IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
    let rows = builder.build().fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            Ok((
                uuid_of(row, "record_id")?,
                Subject {
                    active_revision_id: optional_uuid(row, "active_revision_id")?,
                    generation: row.try_get("generation")?,
                    first_published_at: optional_time(row, "first_published_at")?,
                },
            ))
        })
        .collect()
}

pub(crate) async fn open_requests_of(
    pool: &MySqlPool,
    record_type: &str,
    ids: &[Uuid],
) -> Result<Vec<PublicationRequest>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
    builder
        .push(" AND status IN ('awaiting_approval', 'scheduled', 'ready') AND record_type = ")
        .push_bind(record_type.to_owned())
        .push(" AND record_id IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(") ORDER BY created_at, id");
    let rows = builder.build().fetch_all(pool).await?;
    rows.iter().map(request_from_row).collect()
}
