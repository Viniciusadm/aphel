use serde_json::Value;

use super::model::{CALENDAR_API_FAILED, CURSOR_EXPIRED, REMOTE_NOT_FOUND};
use aphel_kernel::errors::AppError;
use sso::errors::unavailable;

pub fn encode_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

pub(crate) fn status_error(status: reqwest::StatusCode, cursor_request: bool) -> AppError {
    let code = status.as_u16();
    tracing::warn!(status = code, "calendar provider request refused");
    match code {
        410 if cursor_request => {
            AppError::validation_code(CURSOR_EXPIRED, "the sync cursor expired")
        }
        404 | 410 => {
            AppError::not_found_code(REMOTE_NOT_FOUND, "the calendar event no longer exists")
        }
        401 | 403 => AppError::forbidden_code(
            CALENDAR_API_FAILED,
            "the calendar provider refused the access; check the granted permissions",
        ),
        429 | 500..=599 => unavailable("the calendar provider is unavailable, try again later"),
        _ => AppError::validation_code(
            CALENDAR_API_FAILED,
            "the calendar provider refused the request",
        ),
    }
}

pub async fn send(
    request: reqwest::RequestBuilder,
    cursor_request: bool,
) -> Result<Value, AppError> {
    let response = request
        .send()
        .await
        .map_err(|_| unavailable("the calendar provider could not be reached"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status, cursor_request));
    }
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| unavailable("the calendar provider answered with an unreadable body"))?;
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        AppError::validation_code(
            CALENDAR_API_FAILED,
            "the calendar provider answered with invalid JSON",
        )
    })
}
