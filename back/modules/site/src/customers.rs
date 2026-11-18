use crate::store::customers as store;
use aphel_contract_site::{CUSTOMER_RECORD_TYPE, SiteCustomer};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::security::{PasswordHasher, SecureTokens};
use aphel_sdk::platform::application::shared::{FieldChange, RecordRef, RecordType};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::auth::{Argon2PasswordHasher, Sha256SecureTokens};
use chrono::Duration;
use serde_json::json;
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

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
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
    pool: &Db,
    email: &str,
) -> Result<Option<SiteCustomer>, AppError> {
    Ok(store::by_email(pool, email).await?)
}

pub(crate) async fn export(pool: &Db, id: &str) -> Result<Option<serde_json::Value>, AppError> {
    Ok(store::export(pool, id).await?.map(|row| {
        json!({
            "id": row.id,
            "email": row.email,
            "name": row.name,
            "email_verified_at": row.email_verified_at.map(|value| value.to_rfc3339()),
            "last_login_at": row.last_login_at.map(|value| value.to_rfc3339()),
            "created_at": row.created_at.to_rfc3339(),
        })
    }))
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
    let inserted = store::insert(&mut tx, &id, &email, &name, password_hash).await;
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
    pool: &Db,
    email: &str,
    password: &str,
) -> Result<Option<SiteCustomer>, AppError> {
    let Some(email) = normalize_email(email) else {
        return Ok(None);
    };
    let Some((customer, hash)) = store::credentials(pool, &email).await? else {
        let _ = Argon2PasswordHasher.hash_password(password);
        return Ok(None);
    };
    let Some(hash) = hash else {
        return Ok(None);
    };
    if !Argon2PasswordHasher.verify_password(password, &hash)? {
        return Ok(None);
    }
    Ok(Some(customer))
}

pub(crate) async fn start_session(
    pool: &Db,
    customer_id: &str,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<String, AppError> {
    let issued = Sha256SecureTokens.issue()?;
    store::start_session(
        pool,
        customer_id,
        &issued.hash,
        ip,
        user_agent.map(|agent| agent.chars().take(255).collect::<String>()),
        SESSION_DAYS,
    )
    .await?;
    Ok(issued.token.expose().to_owned())
}

pub(crate) async fn current(pool: &Db, token: &str) -> Result<Option<SiteCustomer>, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    let Some(session) = store::live_session(pool, &hash, TOUCH_MINUTES).await? else {
        return Ok(None);
    };
    if session.stale {
        store::extend_session(pool, &session.session_id, SESSION_DAYS).await?;
    }
    Ok(Some(session.customer))
}

pub(crate) async fn end_session(pool: &Db, token: &str) -> Result<(), AppError> {
    store::revoke_session(pool, &Sha256SecureTokens.digest(token)).await?;
    Ok(())
}

pub(crate) async fn issue_token(
    pool: &Db,
    customer_id: &str,
    purpose: TokenPurpose,
) -> Result<String, AppError> {
    let issued = Sha256SecureTokens.issue()?;
    store::replace_token(
        pool,
        customer_id,
        purpose.as_str(),
        &issued.hash,
        purpose.lifetime().num_minutes(),
    )
    .await?;
    Ok(issued.token.expose().to_owned())
}

pub(crate) async fn peek_token(
    pool: &Db,
    token: &str,
    purpose: TokenPurpose,
) -> Result<Option<String>, AppError> {
    Ok(store::token_customer(pool, &Sha256SecureTokens.digest(token), purpose.as_str()).await?)
}

pub(crate) async fn redeem_token(
    pool: &Db,
    token: &str,
    purpose: TokenPurpose,
) -> Result<Option<String>, AppError> {
    let hash = Sha256SecureTokens.digest(token);
    Ok(store::redeem_token(pool, &hash, purpose.as_str()).await?)
}

pub(crate) async fn mark_verified(pool: &Db, customer_id: &str) -> Result<(), AppError> {
    store::mark_verified(pool, customer_id).await?;
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
    store::replace_password(&mut tx, customer_id, hash).await?;
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
    store::rename(&mut tx, &customer.id, &name).await?;
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
    let Some((email, name)) = store::close(
        &mut tx,
        customer_id,
        &format!("{customer_id}@{CLOSED_EMAIL_DOMAIN}"),
    )
    .await?
    else {
        return Ok(());
    };
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
