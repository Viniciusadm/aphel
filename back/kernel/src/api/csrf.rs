use axum::extract::Request;
use axum::http::{HeaderName, Method};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::error::app_error;
use crate::api::extractors::auth::bearer_token;
use crate::errors::AppError;

pub const CSRF_HEADER: HeaderName = HeaderName::from_static("x-requested-with");
pub const CSRF_HEADER_VALUE: &str = "aphel";
pub const CSRF_HEADER_MISSING: &str = "csrf_header_missing";
pub const PUBLIC_HOOKS_PREFIX: &str = "/api/v1/hooks/";
pub const PUBLIC_ROUTES_PREFIX: &str = "/api/v1/public/";

pub fn is_public_path(path: &str) -> bool {
    path.starts_with(PUBLIC_ROUTES_PREFIX) || path.starts_with(PUBLIC_HOOKS_PREFIX)
}

pub async fn require_header(request: Request, next: Next) -> Response {
    let safe_method = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let has_header = request
        .headers()
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(CSRF_HEADER_VALUE));

    let api_token = bearer_token(request.headers()).is_some();
    let public_route = is_public_path(request.uri().path());

    if safe_method || has_header || api_token || public_route {
        next.run(request).await
    } else {
        app_error(AppError::forbidden_code(
            CSRF_HEADER_MISSING,
            "requests that change data must send the X-Requested-With: aphel header",
        ))
    }
}
