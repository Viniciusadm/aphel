use chrono::{Duration, NaiveDateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use super::oauth::{random_token, sha256_hex};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

pub const STATE_TTL_MINUTES: i64 = 10;
pub const PURPOSE_LOGIN: &str = "login";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewState {
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredState {
    pub purpose: String,
    pub nonce: String,
    pub code_verifier: String,
    pub user_id: Option<Uuid>,
    pub redirect_path: Option<String>,
}

fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

pub async fn create(
    ctx: &Ctx<'_>,
    key: &str,
    purpose: &str,
    user_id: Option<Uuid>,
    redirect_path: Option<&str>,
) -> Result<NewState, AppError> {
    let fresh = NewState {
        state: random_token(32)?,
        nonce: random_token(24)?,
        code_verifier: random_token(48)?,
    };
    let pool = ctx.pool();
    sqlx::query("DELETE FROM sso_oauth_states WHERE expires_at < ? LIMIT 500")
        .bind(now())
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO sso_oauth_states (id, provider_key, purpose, state_hash, nonce, code_verifier, user_id, redirect_path, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
    .bind(Uuid::now_v7().to_string())
    .bind(key)
    .bind(purpose)
    .bind(sha256_hex(&fresh.state))
    .bind(&fresh.nonce)
    .bind(&fresh.code_verifier)
    .bind(user_id.map(|id| id.to_string()))
    .bind(redirect_path)
    .bind(now() + Duration::minutes(STATE_TTL_MINUTES))
    .execute(pool)
    .await?;
    Ok(fresh)
}

pub async fn consume(
    ctx: &Ctx<'_>,
    key: &str,
    state: &str,
) -> Result<Option<StoredState>, AppError> {
    if state.is_empty() || state.len() > 128 {
        return Ok(None);
    }
    let hash = sha256_hex(state);
    let mut tx = ctx.begin().await?;
    let row = sqlx::query("SELECT purpose, nonce, code_verifier, user_id, redirect_path, expires_at FROM sso_oauth_states WHERE provider_key = ? AND state_hash = ? FOR UPDATE")
    .bind(key).bind(&hash)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    sqlx::query("DELETE FROM sso_oauth_states WHERE provider_key = ? AND state_hash = ?")
        .bind(key)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let expires_at: NaiveDateTime = row.try_get("expires_at")?;
    if expires_at < now() {
        return Ok(None);
    }
    let user_id: Option<String> = row.try_get("user_id")?;
    Ok(Some(StoredState {
        purpose: row.try_get("purpose")?,
        nonce: row.try_get("nonce")?,
        code_verifier: row.try_get("code_verifier")?,
        user_id: user_id.and_then(|id| Uuid::parse_str(&id).ok()),
        redirect_path: row.try_get("redirect_path")?,
    }))
}
