use axum::http::{HeaderMap, HeaderValue, header};
use chrono::{DateTime, Utc};

use crate::domain::Secret;
use crate::errors::AppError;

pub const SESSION_COOKIE: &str = "aphel_session";

pub fn read(headers: &HeaderMap) -> Option<Secret> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == SESSION_COOKIE && !value.is_empty())
        .map(|(_, value)| Secret::new(value))
}

pub fn issue(
    token: &Secret,
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
    secure: bool,
) -> Result<HeaderValue, AppError> {
    let max_age = (expires_at - now).num_seconds().max(0);
    build(token.expose(), max_age, secure)
}

pub fn clear(secure: bool) -> Result<HeaderValue, AppError> {
    build("", 0, secure)
}

fn build(value: &str, max_age: i64, secure: bool) -> Result<HeaderValue, AppError> {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
    ))
    .map_err(|_| AppError::Internal("invalid session cookie".to_owned()))
}
