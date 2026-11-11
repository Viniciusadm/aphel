use aphel_contract_site::{CUSTOMER_RECORD_TYPE, SiteCustomer};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::security::{PasswordHasher, SecureTokens};
use aphel_sdk::platform::application::shared::{FieldChange, RecordRef, RecordType};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::auth::{Argon2PasswordHasher, Sha256SecureTokens};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

pub(crate) const SESSION_COOKIE: &str = "aphel_site_session";
pub(crate) const SESSION_DAYS: i64 = 30;
const TOUCH_MINUTES: i64 = 5;
pub(crate) const MIN_PASSWORD_CHARS: usize = 8;
const MAX_PASSWORD_CHARS: usize = 200;
const MAX_NAME_CHARS: usize = 200;
const CUSTOMER_CREATED: &str = "site.customer.created";
const CUSTOMER_UPDATED: &str = "site.customer.updated";
const CUSTOMER_DELETED: &str = "site.customer.deleted";
const CLOSED_EMAIL_DOMAIN: &str = "closed.invalid";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenPurpose {
    Verify,
    Reset,
    Magic,
}

impl TokenPurpose {
    fn as_str(self) -> &'static str {
        match self {
            Self::Verify => "verify",
            Self::Reset => "reset",
            Self::Magic => "magic",
        }
    }

    fn lifetime(self) -> Duration {
        match self {
            Self::Verify => Duration::days(3),
            Self::Reset => Duration::hours(1),
            Self::Magic => Duration::minutes(20),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SignUpError {
    EmailTaken,
    Invalid(&'static str),
    Failed(AppError),
}

impl From<AppError> for SignUpError {
    fn from(error: AppError) -> Self {
        Self::Failed(error)
    }
}

impl From<sqlx::Error> for SignUpError {
    fn from(error: sqlx::Error) -> Self {
        Self::Failed(error.into())
    }
}

pub(crate) fn record_type() -> RecordType {
    RecordType::from_static(CUSTOMER_RECORD_TYPE)
}

pub(crate) fn normalize_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_ascii_lowercase();
    aphel_kernel::domain::Email::new(email.clone())
        .ok()
        .map(|_| email)
}

pub(crate) fn valid_password(password: &str) -> bool {
    let length = password.chars().count();
    (MIN_PASSWORD_CHARS..=MAX_PASSWORD_CHARS).contains(&length)
}

fn clean_name(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NAME_CHARS)
        .collect()
}

pub(crate) fn customer_scope(customer_id: &str) -> AccessScope {
    AccessScope::system().acting_as("site_customer", customer_id)
}

fn customer_of(row: &sqlx::mysql::MySqlRow) -> Result<SiteCustomer, AppError> {
    Ok(SiteCustomer {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        name: row.try_get("name")?,
        email_verified: row
            .try_get::<Option<DateTime<Utc>>, _>("email_verified_at")?
            .is_some(),
    })
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    customer_id: &str,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let id = Uuid::parse_str(customer_id)
        .map_err(|_| AppError::Internal("invalid customer id".to_owned()))?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(record_type(), id),
            event_type,
            payload: json!({ "changes": changes.iter().map(|change| json!({ "field": change.field, "old": change.old, "new": change.new })).collect::<Vec<_>>() }),
            changes,
            ip,
        },
    )
    .await
}

pub(crate) async fn find_by_email(
    pool: &MySqlPool,
    email: &str,
) -> Result<Option<SiteCustomer>, AppError> {
    let row = sqlx::query(
        "SELECT id, email, name, email_verified_at FROM site_customers WHERE email = ? AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(customer_of).transpose()
}

pub(crate) async fn export(
    pool: &MySqlPool,
    id: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    let row = sqlx::query(
        "SELECT id, email, name, email_verified_at, last_login_at, created_at FROM site_customers WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| -> Result<serde_json::Value, AppError> {
        let moment = |column: &str| -> Result<Option<String>, AppError> {
            Ok(row
                .try_get::<Option<DateTime<Utc>>, _>(column)?
                .map(|value| value.to_rfc3339()))
        };
        Ok(json!({
            "id": row.try_get::<String, _>("id")?,
            "email": row.try_get::<String, _>("email")?,
            "name": row.try_get::<String, _>("name")?,
            "email_verified_at": moment("email_verified_at")?,
            "last_login_at": moment("last_login_at")?,
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at")?.to_rfc3339(),
        }))
    })
    .transpose()
}

pub(crate) async fn sign_up(
    state: &AppState,
    name: &str,
    email: &str,
    password: Option<&str>,
    ip: Option<String>,
) -> Result<SiteCustomer, SignUpError> {
    let name = clean_name(name);
    if name.is_empty() {
        return Err(SignUpError::Invalid("name"));
    }
    let email = normalize_email(email).ok_or(SignUpError::Invalid("email"))?;
    let password_hash = match password {
        Some(password) if !valid_password(password) => {
            return Err(SignUpError::Invalid("password"));
        }
        Some(password) => Some(Argon2PasswordHasher.hash_password(password)?),
        None => None,
    };
    let pool = state.db()?;
    if find_by_email(&pool, &email).await?.is_some() {
        return Err(SignUpError::EmailTaken);
    }
    let id = Uuid::now_v7().to_string();
    let scope = customer_scope(&id);
    let ctx = Ctx::from_state(state, &scope).await?;
    let mut tx = ctx.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO site_customers (id, email, name, password_hash) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&email)
    .bind(&name)
    .bind(password_hash)
    .execute(&mut *tx)
    .await;
    if let Err(error) = inserted {
        return match error.as_database_error() {
            Some(database) if database.is_unique_violation() => Err(SignUpError::EmailTaken),
            _ => Err(SignUpError::Failed(error.into())),
        };
    }
    publish(
        &ctx,
        &mut tx,
        &id,
        CUSTOMER_CREATED,
        vec![
            FieldChange::new("name", serde_json::Value::Null, json!(name)),
            FieldChange::new("email", serde_json::Value::Null, json!(email)),
        ],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(SiteCustomer {
        id,
        email,
        name,
        email_verified: false,
    })
}

pub(crate) async fn authenticate(
    pool: &MySqlPool,
    email: &str,
    password: &str,
) -> Result<Option<SiteCustomer>, AppError> {
    let Some(email) = normalize_email(email) else {
        return Ok(None);
    };
    let row = sqlx::query(
        "SELECT id, email, name, email_verified_at, password_hash FROM site_customers WHERE email = ? AND deleted_at IS NULL",
    )
    .bind(&email)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        let _ = Argon2PasswordHasher.hash_password(password);
        return Ok(None);
    };
    let hash: Option<String> = row.try_get("password_hash")?;
    let Some(hash) = hash else {
        return Ok(None);
    };
    if !Argon2PasswordHasher.verify_password(password, &hash)? {
        return Ok(None);
    }
    customer_of(&row).map(Some)
}

pub(crate) async fn start_session(
    pool: &MySqlPool,
    customer_id: &str,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<String, AppError> {
    let issued = Sha256SecureTokens.issue()?;
    sqlx::query(
        "INSERT INTO site_customer_sessions (id, customer_id, token_hash, ip, user_agent, expires_at) \
         VALUES (?, ?, ?, ?, ?, NOW(6) + INTERVAL ? DAY)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(customer_id)
    .bind(&issued.hash)
    .bind(ip)
    .bind(user_agent.map(|agent| agent.chars().take(255).collect::<String>()))
    .bind(SESSION_DAYS)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE site_customers SET last_login_at = NOW(6) WHERE id = ?")
        .bind(customer_id)
        .execute(pool)
        .await?;
    Ok(issued.token.expose().to_owned())
}

pub(crate) async fn current(
    pool: &MySqlPool,
    token: &str,
) -> Result<Option<SiteCustomer>, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    let row = sqlx::query(
        "SELECT c.id, c.email, c.name, c.email_verified_at, s.id AS session_id, \
         s.last_seen_at < NOW(6) - INTERVAL ? MINUTE AS stale \
         FROM site_customer_sessions s JOIN site_customers c ON c.id = s.customer_id \
         WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > NOW(6) AND c.deleted_at IS NULL",
    )
    .bind(TOUCH_MINUTES)
    .bind(&hash)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<i64, _>("stale")? != 0 {
        sqlx::query(
            "UPDATE site_customer_sessions SET last_seen_at = NOW(6), expires_at = NOW(6) + INTERVAL ? DAY WHERE id = ?",
        )
        .bind(SESSION_DAYS)
        .bind(row.try_get::<String, _>("session_id")?)
        .execute(pool)
        .await?;
    }
    customer_of(&row).map(Some)
}

pub(crate) async fn end_session(pool: &MySqlPool, token: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE site_customer_sessions SET revoked_at = NOW(6) WHERE token_hash = ?")
        .bind(Sha256SecureTokens.digest(token))
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn issue_token(
    pool: &MySqlPool,
    customer_id: &str,
    purpose: TokenPurpose,
) -> Result<String, AppError> {
    let issued = Sha256SecureTokens.issue()?;
    sqlx::query(
        "UPDATE site_customer_tokens SET used_at = NOW(6) WHERE customer_id = ? AND purpose = ? AND used_at IS NULL",
    )
    .bind(customer_id)
    .bind(purpose.as_str())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO site_customer_tokens (id, customer_id, purpose, token_hash, expires_at) \
         VALUES (?, ?, ?, ?, NOW(6) + INTERVAL ? MINUTE)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(customer_id)
    .bind(purpose.as_str())
    .bind(&issued.hash)
    .bind(purpose.lifetime().num_minutes())
    .execute(pool)
    .await?;
    Ok(issued.token.expose().to_owned())
}

pub(crate) async fn peek_token(
    pool: &MySqlPool,
    token: &str,
    purpose: TokenPurpose,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT customer_id FROM site_customer_tokens \
         WHERE token_hash = ? AND purpose = ? AND used_at IS NULL AND expires_at > NOW(6)",
    )
    .bind(Sha256SecureTokens.digest(token))
    .bind(purpose.as_str())
    .fetch_optional(pool)
    .await?)
}

pub(crate) async fn redeem_token(
    pool: &MySqlPool,
    token: &str,
    purpose: TokenPurpose,
) -> Result<Option<String>, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    let mut tx = pool.begin().await?;
    let customer: Option<String> = sqlx::query_scalar(
        "SELECT customer_id FROM site_customer_tokens \
         WHERE token_hash = ? AND purpose = ? AND used_at IS NULL AND expires_at > NOW(6) FOR UPDATE",
    )
    .bind(&hash)
    .bind(purpose.as_str())
    .fetch_optional(&mut *tx)
    .await?;
    if customer.is_some() {
        sqlx::query("UPDATE site_customer_tokens SET used_at = NOW(6) WHERE token_hash = ?")
            .bind(&hash)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(customer)
}

pub(crate) async fn mark_verified(pool: &MySqlPool, customer_id: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE site_customers SET email_verified_at = COALESCE(email_verified_at, NOW(6)), updated_at = NOW(6) WHERE id = ?",
    )
    .bind(customer_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn set_password(
    state: &AppState,
    customer_id: &str,
    password: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let hash = Argon2PasswordHasher.hash_password(password)?;
    let scope = customer_scope(customer_id);
    let ctx = Ctx::from_state(state, &scope).await?;
    let mut tx = ctx.begin().await?;
    sqlx::query("UPDATE site_customers SET password_hash = ?, updated_at = NOW(6) WHERE id = ?")
        .bind(hash)
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE site_customer_sessions SET revoked_at = NOW(6) WHERE customer_id = ? AND revoked_at IS NULL",
    )
    .bind(customer_id)
    .execute(&mut *tx)
    .await?;
    publish(
        &ctx,
        &mut tx,
        customer_id,
        CUSTOMER_UPDATED,
        vec![FieldChange::new(
            "password",
            json!("[redacted]"),
            json!("[redacted]"),
        )],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn rename(
    state: &AppState,
    customer: &SiteCustomer,
    name: &str,
    ip: Option<String>,
) -> Result<bool, AppError> {
    let name = clean_name(name);
    if name.is_empty() || name == customer.name {
        return Ok(!name.is_empty());
    }
    let scope = customer_scope(&customer.id);
    let ctx = Ctx::from_state(state, &scope).await?;
    let mut tx = ctx.begin().await?;
    sqlx::query("UPDATE site_customers SET name = ?, updated_at = NOW(6) WHERE id = ?")
        .bind(&name)
        .bind(&customer.id)
        .execute(&mut *tx)
        .await?;
    publish(
        &ctx,
        &mut tx,
        &customer.id,
        CUSTOMER_UPDATED,
        vec![FieldChange::new("name", json!(customer.name), json!(name))],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(true)
}

pub(crate) async fn close(
    ctx: &Ctx<'_>,
    customer_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT email, name FROM site_customers WHERE id = ? AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(customer_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(());
    };
    let email: String = row.try_get("email")?;
    let name: String = row.try_get("name")?;
    sqlx::query(
        "UPDATE site_customers SET email = ?, name = '', password_hash = NULL, deleted_at = NOW(6), updated_at = NOW(6) WHERE id = ?",
    )
    .bind(format!("{customer_id}@{CLOSED_EMAIL_DOMAIN}"))
    .bind(customer_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE site_customer_sessions SET revoked_at = NOW(6) WHERE customer_id = ? AND revoked_at IS NULL")
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM site_customer_tokens WHERE customer_id = ?")
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;
    publish(
        ctx,
        &mut tx,
        customer_id,
        CUSTOMER_DELETED,
        vec![
            FieldChange::new("email", json!(email), serde_json::Value::Null),
            FieldChange::new("name", json!(name), serde_json::Value::Null),
        ],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
