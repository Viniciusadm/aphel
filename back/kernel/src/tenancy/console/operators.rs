use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use uuid::Uuid;

use crate::application::security::{PasswordHasher, SecureTokens};
use crate::database::Db;
use crate::domain::Secret;
use crate::errors::AppError;
use crate::infrastructure::auth::{Argon2PasswordHasher, Sha256SecureTokens};

const PASSWORD_MIN: usize = 8;
const PASSWORD_MAX: usize = 128;
const NAME_MAX: usize = 140;
const EMAIL_MAX: usize = 255;
const LOCKOUT_ATTEMPTS: u32 = 5;
const LOCKOUT_MINUTES: i64 = 15;
const SESSION_HOURS: i64 = 12;
const INVITATION_DAYS: i64 = 7;
const SEEN_REFRESH_SECONDS: i64 = 60;

pub const STATUS_INVITED: &str = "invited";
pub const STATUS_ACTIVE: &str = "active";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operator {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleSession {
    pub id: Uuid,
    pub operator: Operator,
}

pub struct IssuedSession {
    pub token: Secret,
    pub expires_at: DateTime<Utc>,
}

type OperatorRow = (
    String,
    String,
    String,
    String,
    Option<NaiveDateTime>,
    NaiveDateTime,
);

type CredentialRow = (String, String, u32, Option<NaiveDateTime>, String);

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

fn invalid_credentials() -> AppError {
    AppError::unauthorized_code(
        "console_invalid_credentials",
        "the email or the password is wrong",
    )
}

pub fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    let valid = email.len() <= EMAIL_MAX
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if valid {
        Ok(email)
    } else {
        Err(AppError::validation_code(
            "operator_email_invalid",
            "the email is not valid",
        ))
    }
}

pub fn validate_name(name: &str) -> Result<String, AppError> {
    let name = name.trim().to_owned();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(AppError::validation_code(
            "operator_name_invalid",
            format!("the name must have 1 to {NAME_MAX} characters"),
        ));
    }
    Ok(name)
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    if (PASSWORD_MIN..=PASSWORD_MAX).contains(&password.chars().count()) {
        Ok(())
    } else {
        Err(AppError::validation_code(
            "operator_password_invalid",
            format!("the password must have {PASSWORD_MIN} to {PASSWORD_MAX} characters"),
        ))
    }
}

fn unique_email(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::conflict_code("operator_email_taken", "another operator uses this email")
        }
        _ => error.into(),
    }
}

pub async fn find(pool: &Db, id: Uuid) -> Result<Option<Operator>, AppError> {
    let row: Option<OperatorRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OPERATOR_COLUMNS} FROM operators o WHERE o.id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;
    row.map(operator).transpose()
}

pub async fn list(pool: &Db) -> Result<Vec<Operator>, AppError> {
    let rows: Vec<OperatorRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OPERATOR_COLUMNS} FROM operators o ORDER BY o.status = 'deactivated', o.display_name"
    )))
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(operator).collect()
}

pub async fn create(
    pool: &Db,
    email: &str,
    display_name: &str,
    password: &str,
) -> Result<Operator, AppError> {
    let email = normalize_email(email)?;
    let display_name = validate_name(display_name)?;
    validate_password(password)?;
    let id = Uuid::now_v7();
    let hash = Argon2PasswordHasher.hash_password(password)?;
    sqlx::query(
        "INSERT INTO operators (id, email, password_hash, display_name, status) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&email)
    .bind(hash)
    .bind(&display_name)
    .bind(STATUS_ACTIVE)
    .execute(pool)
    .await
    .map_err(unique_email)?;
    find(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("the operator was not stored".to_owned()))
}

pub async fn authenticate(pool: &Db, email: &str, password: &str) -> Result<Operator, AppError> {
    let email = email.trim().to_lowercase();
    let row: Option<CredentialRow> = sqlx::query_as(
        "SELECT id, password_hash, failed_logins, locked_until, status FROM operators WHERE email = ?",
    )
    .bind(&email)
    .fetch_optional(pool)
    .await?;
    let Some((id, hash, failed_logins, locked_until, status)) = row else {
        let _ = Argon2PasswordHasher.verify_password(password, "");
        return Err(invalid_credentials());
    };
    let now = Utc::now();
    if locked_until.is_some_and(|until| until.and_utc() > now) {
        return Err(AppError::too_many_requests_code(
            "console_sign_in_locked",
            "too many wrong passwords; try again in a few minutes",
        ));
    }
    if !Argon2PasswordHasher.verify_password(password, &hash)? || status != STATUS_ACTIVE {
        let failures = failed_logins + 1;
        let (failures, locked) = if failures >= LOCKOUT_ATTEMPTS {
            (0, Some(now + Duration::minutes(LOCKOUT_MINUTES)))
        } else {
            (failures, None)
        };
        sqlx::query("UPDATE operators SET failed_logins = ?, locked_until = ? WHERE id = ?")
            .bind(failures)
            .bind(locked.map(|at| at.naive_utc()))
            .bind(&id)
            .execute(pool)
            .await?;
        return Err(invalid_credentials());
    }
    sqlx::query(
        "UPDATE operators SET failed_logins = 0, locked_until = NULL, last_login_at = UTC_TIMESTAMP(6) WHERE id = ?",
    )
    .bind(&id)
    .execute(pool)
    .await?;
    let id = Uuid::parse_str(&id)
        .map_err(|_| AppError::Persistence(format!("invalid operator id {id:?}")))?;
    find(pool, id).await?.ok_or_else(invalid_credentials)
}

pub async fn issue_session(
    pool: &Db,
    operator_id: Uuid,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<IssuedSession, AppError> {
    let issued = Sha256SecureTokens.issue()?;
    let expires_at = Utc::now() + Duration::hours(SESSION_HOURS);
    sqlx::query(
        "INSERT INTO operator_sessions (id, operator_id, token_hash, ip, user_agent, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(operator_id.to_string())
    .bind(&issued.hash)
    .bind(ip)
    .bind(user_agent.map(|agent| agent.chars().take(255).collect::<String>()))
    .bind(expires_at.naive_utc())
    .execute(pool)
    .await?;
    Ok(IssuedSession {
        token: issued.token,
        expires_at,
    })
}

pub async fn resolve_session(pool: &Db, token: &str) -> Result<Option<ConsoleSession>, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    let row: Option<SessionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT s.id, s.last_seen_at, {OPERATOR_COLUMNS} FROM operator_sessions s \
             JOIN operators o ON o.id = s.operator_id \
             WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > UTC_TIMESTAMP(6) \
             AND o.status = 'active'"
    )))
    .bind(hash)
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
    if (Utc::now() - last_seen_at.and_utc()).num_seconds() > SEEN_REFRESH_SECONDS {
        sqlx::query("UPDATE operator_sessions SET last_seen_at = UTC_TIMESTAMP(6) WHERE id = ?")
            .bind(&session_id)
            .execute(pool)
            .await?;
    }
    Ok(Some(ConsoleSession {
        id: Uuid::parse_str(&session_id)
            .map_err(|_| AppError::Persistence(format!("invalid session id {session_id:?}")))?,
        operator: operator((id, email, display_name, status, last_login_at, created_at))?,
    }))
}

pub async fn revoke_session(pool: &Db, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE operator_sessions SET revoked_at = UTC_TIMESTAMP(6) WHERE id = ? AND revoked_at IS NULL")
        .bind(session_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

async fn revoke_other_sessions(
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

pub async fn change_password(
    pool: &Db,
    session: &ConsoleSession,
    current: &str,
    new: &str,
) -> Result<(), AppError> {
    validate_password(new)?;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM operators WHERE id = ?")
        .bind(session.operator.id.to_string())
        .fetch_one(pool)
        .await?;
    if !Argon2PasswordHasher.verify_password(current, &hash)? {
        return Err(AppError::validation_code(
            "operator_password_wrong",
            "the current password is wrong",
        ));
    }
    sqlx::query("UPDATE operators SET password_hash = ? WHERE id = ?")
        .bind(Argon2PasswordHasher.hash_password(new)?)
        .bind(session.operator.id.to_string())
        .execute(pool)
        .await?;
    revoke_other_sessions(pool, session.operator.id, Some(session.id)).await
}

pub async fn invite(
    pool: &Db,
    email: &str,
    display_name: &str,
) -> Result<(Operator, Secret), AppError> {
    let email = normalize_email(email)?;
    let display_name = validate_name(display_name)?;
    let issued = Sha256SecureTokens.issue()?;
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO operators (id, email, password_hash, display_name, status, invite_token_hash, invite_expires_at) \
         VALUES (?, ?, '', ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&email)
    .bind(&display_name)
    .bind(STATUS_INVITED)
    .bind(&issued.hash)
    .bind((Utc::now() + Duration::days(INVITATION_DAYS)).naive_utc())
    .execute(pool)
    .await
    .map_err(unique_email)?;
    let operator = find(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("the operator was not stored".to_owned()))?;
    Ok((operator, issued.token))
}

pub async fn renew_invitation(pool: &Db, id: Uuid) -> Result<(Operator, Secret), AppError> {
    let issued = Sha256SecureTokens.issue()?;
    let updated = sqlx::query(
        "UPDATE operators SET invite_token_hash = ?, invite_expires_at = ? WHERE id = ? AND status = 'invited'",
    )
    .bind(&issued.hash)
    .bind((Utc::now() + Duration::days(INVITATION_DAYS)).naive_utc())
    .bind(id.to_string())
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::conflict_code(
            "operator_not_invited",
            "only invited operators get a new invitation link",
        ));
    }
    let operator = find(pool, id).await?.ok_or_else(operator_not_found)?;
    Ok((operator, issued.token))
}

pub async fn invitation(pool: &Db, token: &str) -> Result<Operator, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    let row: Option<OperatorRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OPERATOR_COLUMNS} FROM operators o WHERE o.invite_token_hash = ? \
         AND o.status = 'invited' AND o.invite_expires_at > UTC_TIMESTAMP(6)"
    )))
    .bind(hash)
    .fetch_optional(pool)
    .await?;
    row.map(operator).transpose()?.ok_or_else(|| {
        AppError::not_found_code(
            "operator_invitation_invalid",
            "this invitation link is invalid or expired",
        )
    })
}

pub async fn accept_invitation(
    pool: &Db,
    token: &str,
    password: &str,
) -> Result<Operator, AppError> {
    validate_password(password)?;
    let invited = invitation(pool, token).await?;
    sqlx::query(
        "UPDATE operators SET password_hash = ?, status = 'active', invite_token_hash = NULL, \
         invite_expires_at = NULL, last_login_at = UTC_TIMESTAMP(6) WHERE id = ? AND status = 'invited'",
    )
    .bind(Argon2PasswordHasher.hash_password(password)?)
    .bind(invited.id.to_string())
    .execute(pool)
    .await?;
    find(pool, invited.id).await?.ok_or_else(operator_not_found)
}

pub async fn deactivate(pool: &Db, actor: &Operator, id: Uuid) -> Result<Operator, AppError> {
    if actor.id == id {
        return Err(AppError::conflict_code(
            "operator_self_deactivation",
            "you cannot deactivate yourself",
        ));
    }
    let updated = sqlx::query(
        "UPDATE operators SET status = 'deactivated', invite_token_hash = NULL, invite_expires_at = NULL \
         WHERE id = ? AND status <> 'deactivated'",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(operator_not_found());
    }
    revoke_other_sessions(pool, id, None).await?;
    find(pool, id).await?.ok_or_else(operator_not_found)
}

fn operator_not_found() -> AppError {
    AppError::not_found_code("operator_not_found", "the operator was not found")
}
