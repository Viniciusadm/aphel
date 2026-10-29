use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::error::app_error;
use crate::errors::AppError;

pub const PAYLOAD_TOO_LARGE: &str = "payload_too_large";

pub fn payload_too_large() -> AppError {
    AppError::payload_too_large_code(
        PAYLOAD_TOO_LARGE,
        "the request body is larger than this route accepts",
    )
}

pub async fn envelope(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    if response.status() != StatusCode::PAYLOAD_TOO_LARGE || is_json(&response) {
        return response;
    }
    app_error(payload_too_large())
}

fn is_json(response: &Response) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
}
