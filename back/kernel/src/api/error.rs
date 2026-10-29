use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::api::dto::common::{ErrorBody, ErrorEnvelope};
use crate::errors::{AppError, CodedKind, FieldViolation};

const INTERNAL_ERROR_MESSAGE: &str = "internal server error";

#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        app_error(self.0)
    }
}

pub fn app_error(error: AppError) -> Response {
    let (status, code) = match &error {
        AppError::Configuration(_) => (StatusCode::SERVICE_UNAVAILABLE, "configuration_error"),
        AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
        AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
        AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
        AppError::RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
        AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
        AppError::Validation(_) | AppError::InvalidFields { .. } => {
            (StatusCode::UNPROCESSABLE_ENTITY, "validation_error")
        }
        AppError::Persistence(_) | AppError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
        }
        AppError::CodedFields { code, .. } => (StatusCode::UNPROCESSABLE_ENTITY, *code),
        // Client-facing errors carry their own machine code and HTTP semantics.
        AppError::Coded { kind, code, .. } | AppError::CodedDetails { kind, code, .. } => {
            (coded_status(*kind), *code)
        }
    };

    let message = if status.is_server_error() {
        tracing::error!(error = %error, "request failed");
        match &error {
            AppError::Configuration(message)
            | AppError::Coded {
                kind: CodedKind::Unavailable,
                message,
                ..
            } => message.clone(),
            _ => INTERNAL_ERROR_MESSAGE.to_owned(),
        }
    } else {
        // The body carries the message without the technical type prefix;
        // `error.to_string()` (with prefix) is reserved for logs/tracing.
        error.client_message().to_owned()
    };

    let details = details(&error);
    let retry_after = (status == StatusCode::TOO_MANY_REQUESTS)
        .then(|| {
            details
                .as_ref()
                .and_then(|details| details.get("retry_after"))
                .and_then(Value::as_u64)
        })
        .flatten();
    let mut response = (
        status,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: code.to_owned(),
                message,
                details,
            },
        }),
    )
        .into_response();
    if let Some(seconds) = retry_after {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
    }
    response
}

fn details(error: &AppError) -> Option<Value> {
    match error {
        AppError::InvalidFields { fields, .. } | AppError::CodedFields { fields, .. } => {
            Some(json!({ "fields": field_details(fields) }))
        }
        AppError::CodedDetails { details, .. } => Some(details.clone()),
        _ => None,
    }
}

fn coded_status(kind: CodedKind) -> StatusCode {
    match kind {
        CodedKind::Conflict => StatusCode::CONFLICT,
        CodedKind::Forbidden => StatusCode::FORBIDDEN,
        CodedKind::NotFound => StatusCode::NOT_FOUND,
        CodedKind::Unauthorized => StatusCode::UNAUTHORIZED,
        CodedKind::Validation => StatusCode::UNPROCESSABLE_ENTITY,
        CodedKind::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
        CodedKind::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        CodedKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn field_details(fields: &[FieldViolation]) -> Vec<Value> {
    fields
        .iter()
        .map(|violation| match violation.code {
            Some(code) => {
                json!({ "field": violation.field, "message": violation.message, "code": code })
            }
            None => json!({ "field": violation.field, "message": violation.message }),
        })
        .collect()
}
