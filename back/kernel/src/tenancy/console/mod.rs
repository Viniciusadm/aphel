mod dto;
mod operators;
mod plugins;
mod session;
mod tenants;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::api::error::{ApiError, app_error};
use crate::api::rate_limit::SlidingWindowLimiter;
use crate::config::AppConfig;
use crate::domain::Secret;
use crate::errors::AppError;
use crate::tenancy::{ControlPlane, TenantDirectory, TenantRuntimes};

pub use operators::{Operator, create as create_operator};

pub(crate) const API_PREFIX: &str = "/api/console/v1";
const RESERVED_PREFIX: &str = "/_aphel/";
const COOKIE: &str = "aphel_console_session";
const UI_STYLESHEET_PREFIX: &str = "/api/v1/public/ui/";
const SIGN_IN_ATTEMPTS_PER_MINUTE: u32 = 10;

pub(crate) struct ConsoleState {
    control: ControlPlane,
    config: AppConfig,
    directory: Arc<TenantDirectory>,
    runtimes: Arc<TenantRuntimes>,
    secure_cookies: bool,
    export_dir: PathBuf,
    sign_in_limiter: SlidingWindowLimiter,
}

type Shared = Arc<ConsoleState>;

impl ConsoleState {
    async fn refresh_directory(&self) {
        match self.directory.refresh().await {
            Ok(()) => self.runtimes.retain(&self.directory),
            Err(error) => tracing::warn!("could not refresh the tenant directory: {error}"),
        }
    }
}

#[derive(OpenApi)]
#[openapi(info(
    title = "Aphel console API",
    description = "Operator console of a multi-tenant Aphel installation, served only on CONSOLE_HOST. Sessions use the `aphel_console_session` cookie; requests that change data send `X-Requested-With: aphel`."
))]
struct ConsoleDocument;

fn api() -> OpenApiRouter<Shared> {
    OpenApiRouter::with_openapi(ConsoleDocument::openapi()).nest(
        API_PREFIX,
        session::router()
            .merge(tenants::router())
            .merge(plugins::router()),
    )
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    api().into_openapi()
}

pub fn openapi_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&openapi()).map(|json| json + "\n")
}

pub(crate) fn router(
    control: ControlPlane,
    config: AppConfig,
    directory: Arc<TenantDirectory>,
    runtimes: Arc<TenantRuntimes>,
    console_host: &str,
) -> Router {
    let state = Arc::new(ConsoleState {
        export_dir: PathBuf::from(&config.tenant_export_dir),
        control,
        config,
        directory,
        runtimes,
        secure_cookies: !is_local_host(console_host),
        sign_in_limiter: SlidingWindowLimiter::new(
            SIGN_IN_ATTEMPTS_PER_MINUTE,
            Duration::from_secs(60),
        ),
    });
    let (api, _) = api().split_for_parts();
    api.layer(middleware::from_fn(crate::api::csrf::require_header))
        .fallback(fallback)
        .with_state(state)
}

async fn fallback(State(_): State<Shared>, request: Request) -> Response {
    let path = request.uri().path();
    if let Some(asset) = path.strip_prefix(RESERVED_PREFIX) {
        return crate::api::web::serve_console_asset(asset);
    }
    if path.starts_with(UI_STYLESHEET_PREFIX) {
        return (
            [
                (header::CONTENT_TYPE, "text/css; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            "",
        )
            .into_response();
    }
    if path == "/api" || path.starts_with("/api/") {
        return app_error(AppError::NotFound("route was not found".to_owned()));
    }
    crate::api::web::serve_console_shell(path)
}

fn is_local_host(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host.ends_with(".localhost")
}

fn read_cookie(headers: &HeaderMap) -> Option<Secret> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == COOKIE && !value.is_empty())
        .map(|(_, value)| Secret::new(value))
}

fn session_cookie(
    token: &str,
    expires_at: Option<DateTime<Utc>>,
    secure: bool,
) -> Result<HeaderValue, AppError> {
    let max_age = expires_at.map_or(0, |at| (at - Utc::now()).num_seconds().max(0));
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{secure}"
    ))
    .map_err(|_| AppError::Internal("invalid console session cookie".to_owned()))
}

fn request_origin(headers: &HeaderMap, secure: bool) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("localhost");
    format!("{}://{host}", if secure { "https" } else { "http" })
}

pub(crate) struct OperatorSession(pub operators::ConsoleSession);

impl FromRequestParts<Shared> for OperatorSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Shared,
    ) -> Result<Self, Self::Rejection> {
        let unauthorized = || {
            ApiError::from(AppError::unauthorized_code(
                "console_session_required",
                "sign in to the console",
            ))
        };
        let token = read_cookie(&parts.headers).ok_or_else(unauthorized)?;
        operators::resolve_session(state.control.pool(), token.expose())
            .await?
            .map(Self)
            .ok_or_else(unauthorized)
    }
}
