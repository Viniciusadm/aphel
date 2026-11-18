use chrono::{NaiveDateTime, SecondsFormat};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{self, AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use crate::model::{
    LeadFormDefinition, LeadFormResponse, LeadFormSubmission, LeadFormSubmissionStatus,
};
use crate::{PUBLIC_FORMS_PATH, form_not_found};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;

#[derive(Debug, Clone)]
pub(crate) struct StoredForm {
    pub(crate) id: Uuid,
    pub(crate) public_id: String,
    pub(crate) name: String,
    pub(crate) is_active: bool,
    pub(crate) acting_user_id: Option<Uuid>,
    pub(crate) definition: LeadFormDefinition,
    pub(crate) created_by: Uuid,
    pub(crate) created_at: NaiveDateTime,
    pub(crate) updated_at: NaiveDateTime,
}

impl StoredForm {
    pub(crate) fn acting_user(&self) -> Uuid {
        self.acting_user_id.unwrap_or(self.created_by)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FormStats {
    pub(crate) accepted: i64,
    pub(crate) spam: i64,
    pub(crate) failed: i64,
    pub(crate) last_submission_at: Option<NaiveDateTime>,
}

pub(crate) struct NewSubmission {
    pub(crate) id: Uuid,
    pub(crate) form_id: Uuid,
    pub(crate) status: LeadFormSubmissionStatus,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) deal_id: Option<Uuid>,
    pub(crate) contact_created: bool,
    pub(crate) data: Value,
    pub(crate) ip_address: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
}

const FORM_COLUMNS: &str = "f.id, f.public_id, f.name, f.is_active, f.acting_user_id, f.definition, f.created_by, f.created_at, f.updated_at";

const STATS_COLUMNS: &str = "(SELECT COUNT(*) FROM forms_submissions s WHERE s.form_id = f.id AND s.status = 'accepted') AS accepted_count, \
     (SELECT COUNT(*) FROM forms_submissions s WHERE s.form_id = f.id AND s.status = 'spam') AS spam_count, \
     (SELECT COUNT(*) FROM forms_submissions s WHERE s.form_id = f.id AND s.status = 'failed') AS failed_count, \
     (SELECT MAX(s.created_at) FROM forms_submissions s WHERE s.form_id = f.id AND s.status = 'accepted') AS last_submission_at";

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value)
        .map_err(|_| AppError::Internal(format!("{column} {value} is not a UUID")))
}

fn optional_uuid_column(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| {
            Uuid::parse_str(&value)
                .map_err(|_| AppError::Internal(format!("{column} {value} is not a UUID")))
        })
        .transpose()
}

fn form_from_row(row: &MySqlRow) -> Result<StoredForm, AppError> {
    let Json(definition): Json<LeadFormDefinition> = row.try_get("definition")?;
    Ok(StoredForm {
        id: uuid_column(row, "id")?,
        public_id: row.try_get("public_id")?,
        name: row.try_get("name")?,
        is_active: row.try_get("is_active")?,
        acting_user_id: optional_uuid_column(row, "acting_user_id")?,
        definition,
        created_by: uuid_column(row, "created_by")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn stats_from_row(row: &MySqlRow) -> Result<FormStats, AppError> {
    Ok(FormStats {
        accepted: row.try_get("accepted_count")?,
        spam: row.try_get("spam_count")?,
        failed: row.try_get("failed_count")?,
        last_submission_at: row.try_get("last_submission_at")?,
    })
}

pub(crate) fn timestamp(value: NaiveDateTime) -> String {
    value.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn public_path(public_id: &str, suffix: &str) -> String {
    format!("{PUBLIC_FORMS_PATH}/{public_id}{suffix}")
}

pub(crate) fn response(form: StoredForm, stats: FormStats) -> LeadFormResponse {
    LeadFormResponse {
        page_path: public_path(&form.public_id, "/page"),
        embed_path: public_path(&form.public_id, "/embed.js"),
        id: form.id,
        public_id: form.public_id,
        name: form.name,
        is_active: form.is_active,
        acting_user_id: form.acting_user_id,
        created_by: form.created_by,
        definition: form.definition,
        accepted_count: stats.accepted,
        spam_count: stats.spam,
        failed_count: stats.failed,
        last_submission_at: stats.last_submission_at.map(timestamp),
        created_at: timestamp(form.created_at),
        updated_at: timestamp(form.updated_at),
    }
}

pub(crate) fn new_public_id() -> String {
    format!("f_{}", Uuid::new_v4().simple())
}

pub(crate) async fn list_forms(pool: &MySqlPool) -> Result<Vec<(StoredForm, FormStats)>, AppError> {
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {FORM_COLUMNS}, {STATS_COLUMNS} FROM forms_forms f ORDER BY f.created_at DESC, f.id DESC"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| Ok((form_from_row(row)?, stats_from_row(row)?)))
        .collect()
}

pub(crate) async fn find_form(
    pool: &MySqlPool,
    id: Uuid,
) -> Result<(StoredForm, FormStats), AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {FORM_COLUMNS}, {STATS_COLUMNS} FROM forms_forms f WHERE f.id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(form_not_found)?;
    Ok((form_from_row(&row)?, stats_from_row(&row)?))
}

pub(crate) async fn find_by_public_id(
    pool: &MySqlPool,
    public_id: &str,
) -> Result<Option<StoredForm>, AppError> {
    if public_id.len() > 64
        || !public_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Ok(None);
    }
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {FORM_COLUMNS} FROM forms_forms f WHERE f.public_id = ?"
    )))
    .bind(public_id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(form_from_row).transpose()
}

pub(crate) async fn insert_form(tx: &mut Tx, form: &StoredForm) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO forms_forms (id, public_id, name, is_active, acting_user_id, definition, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(form.id.to_string())
    .bind(&form.public_id)
    .bind(&form.name)
    .bind(form.is_active)
    .bind(form.acting_user_id.map(|id| id.to_string()))
    .bind(Json(&form.definition))
    .bind(form.created_by.to_string())
    .bind(form.created_at)
    .bind(form.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn update_form(tx: &mut Tx, form: &StoredForm) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE forms_forms SET public_id = ?, name = ?, is_active = ?, acting_user_id = ?, definition = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&form.public_id)
    .bind(&form.name)
    .bind(form.is_active)
    .bind(form.acting_user_id.map(|id| id.to_string()))
    .bind(Json(&form.definition))
    .bind(form.updated_at)
    .bind(form.id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn delete_form(tx: &mut Tx, id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM forms_submissions WHERE form_id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM forms_forms WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn insert_submission<'e, E>(
    executor: E,
    submission: &NewSubmission,
) -> Result<(), AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::MySql>,
{
    sqlx::query(
        "INSERT INTO forms_submissions (id, form_id, status, contact_id, deal_id, contact_created, data, ip_address, origin, error_code, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(submission.id.to_string())
    .bind(submission.form_id.to_string())
    .bind(submission.status.as_str())
    .bind(submission.contact_id.map(|id| id.to_string()))
    .bind(submission.deal_id.map(|id| id.to_string()))
    .bind(submission.contact_created)
    .bind(Json(&submission.data))
    .bind(&submission.ip_address)
    .bind(&submission.origin)
    .bind(&submission.error_code)
    .bind(&submission.error_message)
    .execute(executor)
    .await?;
    Ok(())
}

pub(crate) async fn list_submissions(
    pool: &MySqlPool,
    form_id: Uuid,
    status: Option<LeadFormSubmissionStatus>,
    offset: u64,
    limit: u64,
) -> Result<(Vec<LeadFormSubmission>, u64), AppError> {
    let status_filter = if status.is_some() {
        " AND status = ?"
    } else {
        ""
    };
    let mut count = sqlx::query_scalar::<_, i64>(AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM forms_submissions WHERE form_id = ?{status_filter}"
    )))
    .bind(form_id.to_string());
    if let Some(status) = status {
        count = count.bind(status.as_str());
    }
    let total = count.fetch_one(pool).await?;
    let sql = format!(
        "SELECT id, form_id, status, contact_id, deal_id, contact_created, data, ip_address, origin, error_code, error_message, created_at FROM forms_submissions WHERE form_id = ?{status_filter} ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut query = sqlx::query(AssertSqlSafe(sql)).bind(form_id.to_string());
    if let Some(status) = status {
        query = query.bind(status.as_str());
    }
    let rows = query.bind(limit).bind(offset).fetch_all(pool).await?;
    let items = rows
        .iter()
        .map(|row| {
            let status: String = row.try_get("status")?;
            let Json(data): Json<Value> = row.try_get("data")?;
            let created_at: NaiveDateTime = row.try_get("created_at")?;
            Ok(LeadFormSubmission {
                id: uuid_column(row, "id")?,
                form_id: uuid_column(row, "form_id")?,
                status: LeadFormSubmissionStatus::parse(&status)
                    .unwrap_or(LeadFormSubmissionStatus::Failed),
                contact_id: optional_uuid_column(row, "contact_id")?,
                deal_id: optional_uuid_column(row, "deal_id")?,
                contact_created: row.try_get("contact_created")?,
                data,
                ip_address: row.try_get("ip_address")?,
                origin: row.try_get("origin")?,
                error_code: row.try_get("error_code")?,
                error_message: row.try_get("error_message")?,
                created_at: timestamp(created_at),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok((items, u64::try_from(total).unwrap_or_default()))
}
