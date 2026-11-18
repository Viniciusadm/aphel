use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::application::security::{PasswordHasher, SecureTokens};
use crate::database::Db;
use crate::domain::Secret;
use crate::errors::AppError;
use crate::infrastructure::auth::{Argon2PasswordHasher, Sha256SecureTokens};
use crate::tenancy::store::operators as store;

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
    store::find(pool, id).await
}

pub async fn list(pool: &Db) -> Result<Vec<Operator>, AppError> {
    store::list(pool).await
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
    store::insert_active(pool, id, &email, hash, &display_name, STATUS_ACTIVE)
        .await
        .map_err(unique_email)?;
    find(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("the operator was not stored".to_owned()))
}

pub async fn authenticate(pool: &Db, email: &str, password: &str) -> Result<Operator, AppError> {
    let email = email.trim().to_lowercase();
    let Some(credentials) = store::credentials(pool, &email).await? else {
        let _ = Argon2PasswordHasher.verify_password(password, "");
        return Err(invalid_credentials());
    };
    let now = Utc::now();
    if credentials
        .locked_until
        .is_some_and(|until| until.and_utc() > now)
    {
        return Err(AppError::too_many_requests_code(
            "console_sign_in_locked",
            "too many wrong passwords; try again in a few minutes",
        ));
    }
    if !Argon2PasswordHasher.verify_password(password, &credentials.password_hash)?
        || credentials.status != STATUS_ACTIVE
    {
        let failures = credentials.failed_logins + 1;
        let (failures, locked) = if failures >= LOCKOUT_ATTEMPTS {
            (0, Some(now + Duration::minutes(LOCKOUT_MINUTES)))
        } else {
            (failures, None)
        };
        store::record_failed_login(pool, &credentials.id, failures, locked).await?;
        return Err(invalid_credentials());
    }
    store::record_login(pool, &credentials.id).await?;
    let id = Uuid::parse_str(&credentials.id)
        .map_err(|_| AppError::Persistence(format!("invalid operator id {:?}", credentials.id)))?;
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
    store::insert_session(
        pool,
        operator_id,
        &issued.hash,
        ip,
        user_agent.map(|agent| agent.chars().take(255).collect::<String>()),
        expires_at,
    )
    .await?;
    Ok(IssuedSession {
        token: issued.token,
        expires_at,
    })
}

pub async fn resolve_session(pool: &Db, token: &str) -> Result<Option<ConsoleSession>, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    let Some(owner) = store::live_session(pool, &hash).await? else {
        return Ok(None);
    };
    if (Utc::now() - owner.last_seen_at.and_utc()).num_seconds() > SEEN_REFRESH_SECONDS {
        store::touch_session(pool, &owner.session_id).await?;
    }
    Ok(Some(ConsoleSession {
        id: Uuid::parse_str(&owner.session_id).map_err(|_| {
            AppError::Persistence(format!("invalid session id {:?}", owner.session_id))
        })?,
        operator: owner.operator,
    }))
}

pub async fn revoke_session(pool: &Db, session_id: Uuid) -> Result<(), AppError> {
    store::revoke_session(pool, session_id).await
}

pub async fn change_password(
    pool: &Db,
    session: &ConsoleSession,
    current: &str,
    new: &str,
) -> Result<(), AppError> {
    validate_password(new)?;
    let hash = store::password_hash(pool, session.operator.id).await?;
    if !Argon2PasswordHasher.verify_password(current, &hash)? {
        return Err(AppError::validation_code(
            "operator_password_wrong",
            "the current password is wrong",
        ));
    }
    store::set_password_hash(
        pool,
        session.operator.id,
        Argon2PasswordHasher.hash_password(new)?,
    )
    .await?;
    store::revoke_sessions_except(pool, session.operator.id, Some(session.id)).await
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
    store::insert_invited(
        pool,
        id,
        &email,
        &display_name,
        STATUS_INVITED,
        &issued.hash,
        Utc::now() + Duration::days(INVITATION_DAYS),
    )
    .await
    .map_err(unique_email)?;
    let operator = find(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("the operator was not stored".to_owned()))?;
    Ok((operator, issued.token))
}

pub async fn renew_invitation(pool: &Db, id: Uuid) -> Result<(Operator, Secret), AppError> {
    let issued = Sha256SecureTokens.issue()?;
    if !store::renew_invitation(
        pool,
        id,
        &issued.hash,
        Utc::now() + Duration::days(INVITATION_DAYS),
    )
    .await?
    {
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
    store::pending_invitation(pool, &hash)
        .await?
        .ok_or_else(|| {
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
    store::activate_invited(
        pool,
        invited.id,
        Argon2PasswordHasher.hash_password(password)?,
    )
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
    if !store::deactivate(pool, id).await? {
        return Err(operator_not_found());
    }
    store::revoke_sessions_except(pool, id, None).await?;
    find(pool, id).await?.ok_or_else(operator_not_found)
}

fn operator_not_found() -> AppError {
    AppError::not_found_code("operator_not_found", "the operator was not found")
}
