use chrono::{DateTime, NaiveDateTime, Utc};
use uuid::Uuid;

use crate::database::Db;
use crate::errors::AppError;
use crate::tenancy::console::operators::Operator;

type OperatorRow = (
    String,
    String,
    String,
    String,
    Option<NaiveDateTime>,
    NaiveDateTime,
);

type SessionRow = (
    String,
    NaiveDateTime,
    String,
    String,
    String,
    String,
    Option<NaiveDateTime>,
    NaiveDateTime,
);

pub(crate) struct Credentials {
    pub id: String,
    pub password_hash: String,
    pub failed_logins: u32,
    pub locked_until: Option<NaiveDateTime>,
    pub status: String,
}

pub(crate) struct SessionOwner {
    pub session_id: String,
    pub last_seen_at: NaiveDateTime,
    pub operator: Operator,
}

const OPERATOR_COLUMNS: &str =
    "o.id, o.email, o.display_name, o.status, o.last_login_at, o.created_at";

fn operator(row: OperatorRow) -> Result<Operator, AppError> {
    let (id, email, display_name, status, last_login_at, created_at) = row;
    Ok(Operator {
        id: Uuid::parse_str(&id)
            .map_err(|_| AppError::Persistence(format!("invalid operator id {id:?}")))?,
        email,
        display_name,
        status,
        last_login_at: last_login_at.map(|at| at.and_utc()),
        created_at: created_at.and_utc(),
    })
}

pub(crate) async fn find(pool: &Db, id: Uuid) -> Result<Option<Operator>, AppError> {
    let row: Option<OperatorRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OPERATOR_COLUMNS} FROM operators o WHERE o.id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;
    row.map(operator).transpose()
}

pub(crate) async fn list(pool: &Db) -> Result<Vec<Operator>, AppError> {
    let rows: Vec<OperatorRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OPERATOR_COLUMNS} FROM operators o ORDER BY o.status = 'deactivated', o.display_name"
    )))
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(operator).collect()
}

pub(crate) async fn insert_active(
    pool: &Db,
    id: Uuid,
    email: &str,
    password_hash: String,
    display_name: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO operators (id, email, password_hash, display_name, status) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn insert_invited(
    pool: &Db,
    id: Uuid,
    email: &str,
    display_name: &str,
    status: &str,
    invite_token_hash: &str,
    invite_expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO operators (id, email, password_hash, display_name, status, invite_token_hash, invite_expires_at) \
         VALUES (?, ?, '', ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(email)
    .bind(display_name)
    .bind(status)
    .bind(invite_token_hash)
    .bind(invite_expires_at.naive_utc())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn credentials(pool: &Db, email: &str) -> Result<Option<Credentials>, AppError> {
    let row: Option<(String, String, u32, Option<NaiveDateTime>, String)> = sqlx::query_as(
        "SELECT id, password_hash, failed_logins, locked_until, status FROM operators WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(id, password_hash, failed_logins, locked_until, status)| Credentials {
            id,
            password_hash,
            failed_logins,
            locked_until,
            status,
        },
    ))
}

pub(crate) async fn record_failed_login(
    pool: &Db,
    id: &str,
    failed_logins: u32,
    locked_until: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE operators SET failed_logins = ?, locked_until = ? WHERE id = ?")
        .bind(failed_logins)
        .bind(locked_until.map(|at| at.naive_utc()))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn record_login(pool: &Db, id: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE operators SET failed_logins = 0, locked_until = NULL, last_login_at = UTC_TIMESTAMP(6) WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn insert_session(
    pool: &Db,
    operator_id: Uuid,
    token_hash: &str,
    ip: Option<&str>,
    user_agent: Option<String>,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO operator_sessions (id, operator_id, token_hash, ip, user_agent, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(operator_id.to_string())
    .bind(token_hash)
    .bind(ip)
    .bind(user_agent)
    .bind(expires_at.naive_utc())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn live_session(
    pool: &Db,
    token_hash: &str,
) -> Result<Option<SessionOwner>, AppError> {
    let row: Option<SessionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT s.id, s.last_seen_at, {OPERATOR_COLUMNS} FROM operator_sessions s \
             JOIN operators o ON o.id = s.operator_id \
             WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > UTC_TIMESTAMP(6) \
             AND o.status = 'active'"
    )))
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;
    let Some((
        session_id,
        last_seen_at,
        id,
        email,
        display_name,
        status,
        last_login_at,
        created_at,
    )) = row
    else {
        return Ok(None);
    };
    Ok(Some(SessionOwner {
        session_id,
        last_seen_at,
        operator: operator((id, email, display_name, status, last_login_at, created_at))?,
    }))
}

pub(crate) async fn touch_session(pool: &Db, session_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE operator_sessions SET last_seen_at = UTC_TIMESTAMP(6) WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn revoke_session(pool: &Db, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE operator_sessions SET revoked_at = UTC_TIMESTAMP(6) WHERE id = ? AND revoked_at IS NULL")
        .bind(session_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn revoke_sessions_except(
    pool: &Db,
    operator_id: Uuid,
    keep: Option<Uuid>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE operator_sessions SET revoked_at = UTC_TIMESTAMP(6) \
         WHERE operator_id = ? AND revoked_at IS NULL AND id <> ?",
    )
    .bind(operator_id.to_string())
    .bind(keep.map(|id| id.to_string()).unwrap_or_default())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn password_hash(pool: &Db, id: Uuid) -> Result<String, AppError> {
    Ok(
        sqlx::query_scalar("SELECT password_hash FROM operators WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(pool)
            .await?,
    )
}

pub(crate) async fn set_password_hash(
    pool: &Db,
    id: Uuid,
    password_hash: String,
) -> Result<(), AppError> {
    sqlx::query("UPDATE operators SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn renew_invitation(
    pool: &Db,
    id: Uuid,
    invite_token_hash: &str,
    invite_expires_at: DateTime<Utc>,
) -> Result<bool, AppError> {
    let updated = sqlx::query(
        "UPDATE operators SET invite_token_hash = ?, invite_expires_at = ? WHERE id = ? AND status = 'invited'",
    )
    .bind(invite_token_hash)
    .bind(invite_expires_at.naive_utc())
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(updated.rows_affected() > 0)
}

pub(crate) async fn pending_invitation(
    pool: &Db,
    invite_token_hash: &str,
) -> Result<Option<Operator>, AppError> {
    let row: Option<OperatorRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OPERATOR_COLUMNS} FROM operators o WHERE o.invite_token_hash = ? \
         AND o.status = 'invited' AND o.invite_expires_at > UTC_TIMESTAMP(6)"
    )))
    .bind(invite_token_hash)
    .fetch_optional(pool)
    .await?;
    row.map(operator).transpose()
}

pub(crate) async fn activate_invited(
    pool: &Db,
    id: Uuid,
    password_hash: String,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE operators SET password_hash = ?, status = 'active', invite_token_hash = NULL, \
         invite_expires_at = NULL, last_login_at = UTC_TIMESTAMP(6) WHERE id = ? AND status = 'invited'",
    )
    .bind(password_hash)
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn deactivate(pool: &Db, id: Uuid) -> Result<bool, AppError> {
    let updated = sqlx::query(
        "UPDATE operators SET status = 'deactivated', invite_token_hash = NULL, invite_expires_at = NULL \
         WHERE id = ? AND status <> 'deactivated'",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(updated.rows_affected() > 0)
}
