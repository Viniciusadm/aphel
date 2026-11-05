use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use super::model::{CalendarConnectionStatus, STATUS_ACTIVE, STATUS_REVOKED};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::sso::errors::internal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Connection {
    pub(crate) user_id: Uuid,
    pub(crate) account_email: Option<String>,
    pub(crate) status: String,
    pub(crate) sync_cursor: Option<String>,
    pub(crate) last_synced_at: Option<NaiveDateTime>,
    pub(crate) last_error_code: Option<String>,
    pub(crate) last_error_message: Option<String>,
    pub(crate) imported_count: u32,
    pub(crate) exported_count: u32,
    pub(crate) created_at: NaiveDateTime,
}

impl Connection {
    pub(crate) fn status(&self) -> CalendarConnectionStatus {
        let instant = |value: NaiveDateTime| value.and_utc().to_rfc3339();
        CalendarConnectionStatus {
            account_email: self.account_email.clone(),
            status: self.status.clone(),
            last_synced_at: self.last_synced_at.map(instant),
            last_error_code: self.last_error_code.clone(),
            last_error_message: self.last_error_message.clone(),
            imported_count: self.imported_count,
            exported_count: self.exported_count,
            connected_at: instant(self.created_at),
        }
    }

    pub(crate) fn syncable(&self) -> bool {
        self.status != STATUS_REVOKED
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Link {
    pub(crate) activity_id: Uuid,
    pub(crate) remote_id: String,
    pub(crate) remote_updated_at: Option<DateTime<Utc>>,
    pub(crate) local_updated_at: Option<DateTime<Utc>>,
}

fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

fn uuid(value: String) -> Result<Uuid, AppError> {
    Uuid::parse_str(&value).map_err(|error| internal(error.to_string()))
}

fn utc(value: Option<NaiveDateTime>) -> Option<DateTime<Utc>> {
    value.map(|value| value.and_utc())
}

fn naive(value: Option<DateTime<Utc>>) -> Option<NaiveDateTime> {
    value.map(|value| value.naive_utc())
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

const CONNECTION_COLUMNS: &str = "user_id, account_email, status, sync_cursor, last_synced_at, \
     last_error_code, last_error_message, imported_count, exported_count, created_at";

fn connection(row: MySqlRow) -> Result<Connection, AppError> {
    Ok(Connection {
        user_id: uuid(row.try_get("user_id")?)?,
        account_email: row.try_get("account_email")?,
        status: row.try_get("status")?,
        sync_cursor: row.try_get("sync_cursor")?,
        last_synced_at: row.try_get("last_synced_at")?,
        last_error_code: row.try_get("last_error_code")?,
        last_error_message: row.try_get("last_error_message")?,
        imported_count: row.try_get("imported_count")?,
        exported_count: row.try_get("exported_count")?,
        created_at: row.try_get("created_at")?,
    })
}

pub(crate) async fn find_connection(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
) -> Result<Option<Connection>, AppError> {
    sqlx::query(AssertSqlSafe(format!(
        "SELECT {CONNECTION_COLUMNS} FROM task_calendar_connections WHERE provider_key = ? AND user_id = ?"
    )))
    .bind(key)
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?
    .map(connection)
    .transpose()
}

pub(crate) async fn list_connections(
    pool: &MySqlPool,
    key: &str,
) -> Result<Vec<Connection>, AppError> {
    sqlx::query(AssertSqlSafe(format!(
        "SELECT {CONNECTION_COLUMNS} FROM task_calendar_connections WHERE provider_key = ? AND status <> ? ORDER BY created_at LIMIT 1000"
    )))
    .bind(key)
    .bind(STATUS_REVOKED)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(connection)
    .collect()
}

pub(crate) async fn upsert_connection(
    tx: &mut MySqlTx,
    key: &str,
    user_id: Uuid,
    account_email: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO task_calendar_connections (id, provider_key, user_id, account_email, status) VALUES (?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE account_email = VALUES(account_email), status = VALUES(status), sync_cursor = NULL, \
         last_error_code = NULL, last_error_message = NULL, updated_at = CURRENT_TIMESTAMP(6)"
    )
    .bind(Uuid::now_v7().to_string())
    .bind(key)
    .bind(user_id.to_string())
    .bind(account_email.map(|email| truncate(email, 320)))
    .bind(STATUS_ACTIVE)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn delete_connection(
    tx: &mut MySqlTx,
    key: &str,
    user_id: Uuid,
) -> Result<bool, AppError> {
    let result =
        sqlx::query("DELETE FROM task_calendar_connections WHERE provider_key = ? AND user_id = ?")
            .bind(key)
            .bind(user_id.to_string())
            .execute(&mut **tx)
            .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SyncOutcome {
    pub(crate) cursor: Option<String>,
    pub(crate) imported: u32,
    pub(crate) error: Option<(String, String)>,
}

pub(crate) async fn finish_sync(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
    outcome: &SyncOutcome,
) -> Result<(), AppError> {
    let (code, message) = outcome
        .error
        .as_ref()
        .map(|(code, message)| (Some(truncate(code, 64)), Some(truncate(message, 500))))
        .unwrap_or((None, None));
    sqlx::query(
        "UPDATE task_calendar_connections SET sync_cursor = COALESCE(?, sync_cursor), last_synced_at = ?, last_error_code = ?, \
         last_error_message = ?, imported_count = imported_count + ?, updated_at = CURRENT_TIMESTAMP(6) \
         WHERE provider_key = ? AND user_id = ?"
    )
    .bind(outcome.cursor.as_deref())
    .bind(now())
    .bind(code)
    .bind(message)
    .bind(outcome.imported)
    .bind(key)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn reset_cursor(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE task_calendar_connections SET sync_cursor = NULL WHERE provider_key = ? AND user_id = ?"
    )
    .bind(key)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn mark_failure(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
    status: &str,
    code: &str,
    message: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE task_calendar_connections SET status = ?, last_error_code = ?, last_error_message = ?, updated_at = CURRENT_TIMESTAMP(6) \
         WHERE provider_key = ? AND user_id = ?"
    )
    .bind(status)
    .bind(truncate(code, 64))
    .bind(truncate(message, 500))
    .bind(key)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn mark_healthy(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE task_calendar_connections SET status = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE provider_key = ? AND user_id = ? AND status <> ?"
    )
    .bind(STATUS_ACTIVE)
    .bind(key)
    .bind(user_id.to_string())
    .bind(STATUS_REVOKED)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn count_export(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE task_calendar_connections SET exported_count = exported_count + 1 WHERE provider_key = ? AND user_id = ?"
    )
    .bind(key)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

const LINK_COLUMNS: &str = "activity_id, remote_id, remote_updated_at, local_updated_at";

fn link(row: MySqlRow) -> Result<Link, AppError> {
    Ok(Link {
        activity_id: uuid(row.try_get("activity_id")?)?,
        remote_id: row.try_get("remote_id")?,
        remote_updated_at: utc(row.try_get("remote_updated_at")?),
        local_updated_at: utc(row.try_get("local_updated_at")?),
    })
}

pub(crate) async fn link_by_remote(
    pool: &MySqlPool,
    key: &str,
    user_id: Uuid,
    remote_id: &str,
) -> Result<Option<Link>, AppError> {
    sqlx::query(AssertSqlSafe(format!(
        "SELECT {LINK_COLUMNS} FROM task_calendar_links WHERE provider_key = ? AND user_id = ? AND remote_id = ?"
    )))
    .bind(key)
    .bind(user_id.to_string())
    .bind(remote_id)
    .fetch_optional(pool)
    .await?
    .map(link)
    .transpose()
}

pub(crate) async fn link_by_activity(
    pool: &MySqlPool,
    key: &str,
    activity_id: Uuid,
) -> Result<Option<(Uuid, Link)>, AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT user_id, {LINK_COLUMNS} FROM task_calendar_links WHERE provider_key = ? AND activity_id = ?"
    )))
    .bind(key)
    .bind(activity_id.to_string())
    .fetch_optional(pool)
    .await?;
    match row {
        Some(row) => {
            let user_id = uuid(row.try_get("user_id")?)?;
            Ok(Some((user_id, link(row)?)))
        }
        None => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkWrite<'a> {
    pub(crate) user_id: Uuid,
    pub(crate) activity_id: Uuid,
    pub(crate) remote_id: &'a str,
    pub(crate) remote_etag: Option<&'a str>,
    pub(crate) remote_updated_at: Option<DateTime<Utc>>,
    pub(crate) local_updated_at: Option<DateTime<Utc>>,
}

pub(crate) async fn save_link(
    pool: &MySqlPool,
    key: &str,
    write: &LinkWrite<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO task_calendar_links (id, provider_key, user_id, activity_id, remote_id, remote_etag, remote_updated_at, local_updated_at, last_synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE user_id = VALUES(user_id), activity_id = VALUES(activity_id), remote_id = VALUES(remote_id), \
         remote_etag = VALUES(remote_etag), remote_updated_at = VALUES(remote_updated_at), \
         local_updated_at = VALUES(local_updated_at), last_synced_at = VALUES(last_synced_at)"
    )
    .bind(Uuid::now_v7().to_string())
    .bind(key)
    .bind(write.user_id.to_string())
    .bind(write.activity_id.to_string())
    .bind(truncate(write.remote_id, 255))
    .bind(write.remote_etag.map(|etag| truncate(etag, 255)))
    .bind(naive(write.remote_updated_at))
    .bind(naive(write.local_updated_at))
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn delete_link(
    pool: &MySqlPool,
    key: &str,
    activity_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM task_calendar_links WHERE provider_key = ? AND activity_id = ?")
        .bind(key)
        .bind(activity_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}
