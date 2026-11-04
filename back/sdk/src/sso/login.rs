use super::errors::{error_code, is_too_many_requests, is_unavailable};
use super::{
    oauth::{self, ClientCredentials, IdTokenClaims, ProviderEndpoints, TokenResponse},
    states::StoredState,
    support,
};
use crate::{AppError, Ctx};
use aphel_kernel::api::extractors::ClientInfo;
use aphel_kernel::modules::MODULE_DISABLED;
use async_trait::async_trait;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::Value;
use utoipa::IntoParams;
pub const NO_ACCOUNT: &str = "no_account";
pub const INVALID_STATE: &str = "invalid_state";
pub const INVALID_REQUEST: &str = "invalid_request";
pub const ACCESS_DENIED: &str = "access_denied";
pub const SSO_DISABLED: &str = "sso_disabled";
pub const NOT_CONFIGURED: &str = "sso_not_configured";
pub const EMAIL_NOT_VERIFIED: &str = "email_not_verified";
pub const DOMAIN_NOT_ALLOWED: &str = "domain_not_allowed";
pub const RATE_LIMITED: &str = "rate_limited";
pub const SSO_FAILED: &str = "sso_failed";
pub const LOGIN_PATH: &str = "/login";
pub const TWO_FACTOR_PATH: &str = "/login/two-factor";
pub const LOGIN_HINT: &str = "login_hint";

const MAX_USER_AGENT_LENGTH: usize = 255;
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StartQuery {
    pub redirect: Option<String>,
    pub login_hint: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CallbackQuery {
    pub state: Option<String>,
    pub code: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

impl ClientContext {
    pub fn new(client: ClientInfo) -> Self {
        Self {
            ip: client.0.ip,
            user_agent: client
                .0
                .user_agent
                .map(|value| value.chars().take(MAX_USER_AGENT_LENGTH).collect()),
        }
    }

    pub fn address(&self) -> &str {
        self.ip.as_deref().unwrap_or("unknown")
    }
}

pub fn failure_code(error: &AppError) -> &'static str {
    const KNOWN: [&str; 13] = [
        NO_ACCOUNT,
        INVALID_STATE,
        INVALID_REQUEST,
        ACCESS_DENIED,
        SSO_DISABLED,
        NOT_CONFIGURED,
        EMAIL_NOT_VERIFIED,
        DOMAIN_NOT_ALLOWED,
        RATE_LIMITED,
        oauth::TOKEN_EXCHANGE_FAILED,
        oauth::INVALID_GRANT,
        oauth::INVALID_TOKEN,
        oauth::PROVIDER_UNAVAILABLE,
    ];
    let code = error_code(error);
    if code == MODULE_DISABLED {
        return SSO_DISABLED;
    }
    if code == "not_a_member" {
        return NO_ACCOUNT;
    }
    if is_too_many_requests(error) {
        return RATE_LIMITED;
    }
    if is_unavailable(error) {
        return oauth::PROVIDER_UNAVAILABLE;
    }
    KNOWN
        .into_iter()
        .find(|known| *known == code)
        .unwrap_or(SSO_FAILED)
}

pub fn failure(code: &'static str, message: &str) -> AppError {
    AppError::validation_code(code, message.to_owned())
}

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    fn key(&self) -> &'static str;

    fn label(&self, settings: &Value) -> String;

    async fn endpoints(&self, settings: &Value) -> Result<ProviderEndpoints, AppError>;

    fn login_scopes(&self) -> &'static [&'static str] {
        &["openid", "email", "profile"]
    }

    fn login_params(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn issuer_allowed(
        &self,
        issuer: &str,
        endpoints: &ProviderEndpoints,
        _settings: &Value,
    ) -> bool {
        endpoints.issuer.as_deref() == Some(issuer)
    }

    fn verified_email(&self, claims: &IdTokenClaims, settings: &Value) -> Result<String, AppError>;

    async fn complete_purpose(
        &self,
        _ctx: &Ctx<'_>,
        _state: &StoredState,
        _code: &str,
    ) -> Response {
        redirect_to(&login_error_path(INVALID_STATE))
    }
}

pub fn callback_url(ctx: &Ctx<'_>, key: &str) -> String {
    format!(
        "{}{}/callback",
        support::app_base_url(ctx).trim_end_matches('/'),
        support::public_path(key)
    )
}

pub fn start_path(key: &str) -> String {
    format!("{}/login", support::public_path(key))
}

pub fn login_error_path(code: &str) -> String {
    format!("{LOGIN_PATH}?sso_error={code}")
}

pub fn redirect_to(location: &str) -> Response {
    let mut response = StatusCode::FOUND.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(location) {
        headers.insert(header::LOCATION, value);
    } else {
        headers.insert(header::LOCATION, HeaderValue::from_static("/"));
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}

pub struct Exchanged {
    pub tokens: TokenResponse,
    pub claims: Option<IdTokenClaims>,
}

#[async_trait]
pub trait IdentityFlows: Send + Sync {
    async fn credentials(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        settings: &Value,
    ) -> Result<ClientCredentials, AppError>;
    async fn exchange(
        &self,
        ctx: &Ctx<'_>,
        provider: &dyn IdentityProvider,
        state: &StoredState,
        code: &str,
    ) -> Result<Exchanged, AppError>;
    async fn discover(
        &self,
        state: &aphel_kernel::state::AppState,
        provider: &dyn IdentityProvider,
        email: &str,
    ) -> Result<Vec<aphel_kernel::api::sso::LoginOption>, AppError>;
    async fn start(
        &self,
        ctx: &Ctx<'_>,
        provider: &dyn IdentityProvider,
        query: StartQuery,
        context: ClientContext,
    ) -> Response;
    async fn callback(
        &self,
        ctx: &Ctx<'_>,
        provider: &dyn IdentityProvider,
        query: CallbackQuery,
        context: ClientContext,
    ) -> Response;
    async fn status(
        &self,
        ctx: &Ctx<'_>,
        provider: &dyn IdentityProvider,
    ) -> Result<super::status::SsoLoginStatus, AppError>;
}
fn flows() -> Result<std::sync::Arc<dyn IdentityFlows>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn IdentityFlows>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "identity_flows_unavailable",
                "identity flows are not enabled",
            )
        })
}
pub async fn credentials(
    ctx: &Ctx<'_>,
    key: &str,
    settings: &Value,
) -> Result<ClientCredentials, AppError> {
    flows()?.credentials(ctx, key, settings).await
}
pub async fn exchange(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    state: &StoredState,
    code: &str,
) -> Result<Exchanged, AppError> {
    flows()?.exchange(ctx, provider, state, code).await
}
pub async fn discover_options(
    state: &aphel_kernel::state::AppState,
    provider: &dyn IdentityProvider,
    email: &str,
) -> Result<Vec<aphel_kernel::api::sso::LoginOption>, AppError> {
    let Ok(flows) = flows() else {
        return Ok(Vec::new());
    };
    flows.discover(state, provider, email).await
}
pub async fn start_login(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    query: StartQuery,
    context: ClientContext,
) -> Response {
    match flows() {
        Ok(flows) => flows.start(ctx, provider, query, context).await,
        Err(_) => redirect_to(&login_error_path(SSO_DISABLED)),
    }
}
pub async fn handle_callback(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    query: CallbackQuery,
    context: ClientContext,
) -> Response {
    match flows() {
        Ok(flows) => flows.callback(ctx, provider, query, context).await,
        Err(_) => redirect_to(&login_error_path(SSO_DISABLED)),
    }
}
pub async fn login_status(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
) -> Result<super::status::SsoLoginStatus, AppError> {
    flows()?.status(ctx, provider).await
}
