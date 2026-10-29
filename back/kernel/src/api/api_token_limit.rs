use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::error::app_error;
use crate::api::extractors::auth::bearer_token;
use crate::api::rate_limit::{WindowDecision, shared_hit};
use crate::api::state::AppState;
use crate::application::security::SecureTokens;
use crate::infrastructure::auth::Sha256SecureTokens;

pub const LIMIT_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-limit");
pub const REMAINING_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const SCOPE: &str = "api_token";

pub const RESET_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-reset");

pub async fn limit(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(token) = bearer_token(request.headers()) else {
        return next.run(request).await;
    };
    let key = Sha256SecureTokens.digest(&token);
    let limits = state.configuration().api_tokens;
    let fallback = match state.api_token_limiter() {
        Ok(limiter) => limiter,
        Err(error) => return app_error(error),
    };
    let decision = match shared_hit(SCOPE, &key, limits.limit, limits.window(), &fallback).await {
        Ok(decision) => decision,
        Err(error) => return app_error(error),
    };
    let mut response = if decision.allowed {
        next.run(request).await
    } else {
        app_error(decision.rejection("too many requests for this API token, try again later"))
    };
    let headers = response.headers_mut();
    insert(headers, LIMIT_HEADER, u64::from(decision.limit));
    insert(headers, REMAINING_HEADER, u64::from(decision.remaining));
    insert(headers, RESET_HEADER, reset_seconds(&decision));
    response
}

fn reset_seconds(decision: &WindowDecision) -> u64 {
    let seconds = decision.reset_after.as_secs();
    if decision.reset_after.subsec_nanos() > 0 {
        seconds + 1
    } else {
        seconds
    }
}

fn insert(headers: &mut HeaderMap, name: HeaderName, value: u64) {
    headers.insert(name, HeaderValue::from(value));
}
