use axum::extract::Request;
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;

use crate::api::error::app_error;
use crate::errors::AppError;
use crate::infrastructure::demo::demo_settings;

pub const DEMO_RESTRICTED: &str = "demo_restricted";

const RESTRICTED_PREFIXES: [&str; 7] = [
    "/api/v1/auth/forgot-password",
    "/api/v1/auth/reset-password",
    "/api/v1/auth/logout-all",
    "/api/v1/api-tokens",
    "/api/v1/security/password",
    "/api/v1/security/sessions/revoke-others",
    "/api/v1/two-factor",
];

pub async fn guard(request: Request, next: Next) -> Response {
    let restricted = demo_settings().enabled
        && !matches!(
            *request.method(),
            Method::GET | Method::HEAD | Method::OPTIONS
        )
        && RESTRICTED_PREFIXES
            .iter()
            .any(|prefix| request.uri().path().starts_with(prefix));
    if restricted {
        app_error(AppError::forbidden_code(
            DEMO_RESTRICTED,
            "this action is disabled in the demonstration environment",
        ))
    } else {
        next.run(request).await
    }
}
