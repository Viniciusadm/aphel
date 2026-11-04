use serde_json::Value;

use super::{
    MAIL_ACCESS_DENIED, MAIL_API_FAILED, MAIL_CURSOR_EXPIRED, MAIL_NOT_FOUND, MAIL_TOKEN_REJECTED,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::sso::errors::unavailable;

pub(crate) const MAX_MESSAGE_BYTES: usize = 40 * 1024 * 1024;

pub(crate) fn status_error(status: reqwest::StatusCode) -> AppError {
    let code = status.as_u16();
    tracing::warn!(status = code, "mail provider request refused");
    match code {
        401 => AppError::unauthorized_code(
            MAIL_TOKEN_REJECTED,
            "the mail provider rejected the access token",
        ),
        403 => AppError::forbidden_code(
            MAIL_ACCESS_DENIED,
            "the mail provider refused the access; check the granted permissions",
        ),
        404 => AppError::not_found_code(MAIL_NOT_FOUND, "the mail item no longer exists"),
        410 => AppError::validation_code(MAIL_CURSOR_EXPIRED, "the sync cursor expired"),
        429 | 500..=599 => unavailable("the mail provider is unavailable, try again later"),
        _ => AppError::validation_code(MAIL_API_FAILED, "the mail provider refused the request"),
    }
}

async fn checked(request: reqwest::RequestBuilder) -> Result<reqwest::Response, AppError> {
    let response = request
        .send()
        .await
        .map_err(|_| unavailable("the mail provider could not be reached"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status));
    }
    Ok(response)
}

pub async fn bytes(request: reqwest::RequestBuilder) -> Result<Vec<u8>, AppError> {
    let mut response = checked(request).await?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MESSAGE_BYTES as u64)
    {
        return Err(AppError::validation_code(
            MAIL_API_FAILED,
            "the mail provider answered with a body that is too large",
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| unavailable("the mail provider answered with an unreadable body"))?
    {
        if chunk.len() > MAX_MESSAGE_BYTES - body.len() {
            return Err(AppError::validation_code(
                MAIL_API_FAILED,
                "the mail provider answered with a body that is too large",
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub async fn json(request: reqwest::RequestBuilder) -> Result<Value, AppError> {
    let body = bytes(request).await?;
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&body).map_err(|_| {
        AppError::validation_code(
            MAIL_API_FAILED,
            "the mail provider answered with invalid JSON",
        )
    })
}

pub fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}
