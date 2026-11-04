pub use aphel_sdk::sso::login::{
    ACCESS_DENIED, CallbackQuery, ClientContext, DOMAIN_NOT_ALLOWED, EMAIL_NOT_VERIFIED, Exchanged,
    INVALID_REQUEST, INVALID_STATE, IdentityProvider, LOGIN_HINT, LOGIN_PATH, NO_ACCOUNT,
    NOT_CONFIGURED, RATE_LIMITED, SSO_DISABLED, SSO_FAILED, StartQuery, TWO_FACTOR_PATH,
    callback_url, failure, failure_code, login_error_path, redirect_to, start_path,
};
use std::time::Duration;

use axum::http::{HeaderValue, header};
use axum::response::Response;
use chrono::Utc;
use serde_json::Value;

use super::oauth::{self, AuthorizationRequest, ClientCredentials, IdTokenClaims};
use super::session::{SignInRequest, sign_in};
use super::settings::{self, LoginSettings};
use super::states::{self, PURPOSE_LOGIN, StoredState};
use crate::support;
use aphel_kernel::Ctx;
use aphel_kernel::api::sso::LoginOption;
use aphel_kernel::application::auth::UserRepository;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_users::MySqlUserRepository;
use aphel_kernel::modules::module_settings;
use aphel_kernel::state::AppState;

const LIMIT_PER_MINUTE: u32 = 30;
const MINUTE: Duration = Duration::from_secs(60);
const MAX_REDIRECT_LENGTH: usize = 512;
const MAX_EMAIL_LENGTH: usize = 320;

pub struct Failure(pub(crate) &'static str);

impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        Self(failure_code(&error))
    }
}

pub fn sanitize_redirect(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    let safe = value.starts_with('/')
        && !value.starts_with("//")
        && !value.starts_with("/\\")
        && value.len() <= MAX_REDIRECT_LENGTH
        && !value
            .chars()
            .any(|character| character.is_control() || character == '\\');
    safe.then(|| value.to_owned())
}

pub fn normalize_email(value: &str) -> Option<String> {
    let email = value.trim().to_lowercase();
    let valid = email.len() <= MAX_EMAIL_LENGTH
        && settings::email_domain(&email).is_some()
        && !email.chars().any(char::is_whitespace);
    valid.then_some(email)
}

pub async fn limit(key: &str, bucket: &str, context: &ClientContext) -> Result<(), AppError> {
    support::rate_limit(
        key,
        &format!("{bucket}:{}", context.address()),
        LIMIT_PER_MINUTE,
        MINUTE,
    )
    .await
}

pub async fn credentials(
    ctx: &Ctx<'_>,
    key: &str,
    settings: &Value,
) -> Result<ClientCredentials, AppError> {
    let not_configured = || failure(NOT_CONFIGURED, "the login provider is not configured");
    let client_id = settings::text(settings, settings::CLIENT_ID).ok_or_else(not_configured)?;
    let client_secret = support::secret(ctx, key, settings::CLIENT_SECRET)
        .await?
        .filter(|secret| !secret.trim().is_empty())
        .ok_or_else(not_configured)?;
    Ok(ClientCredentials {
        client_id,
        client_secret,
    })
}

pub async fn discover_options(
    state: &AppState,
    provider: &dyn IdentityProvider,
    email: &str,
) -> Result<Vec<LoginOption>, AppError> {
    let Some(email) = normalize_email(email) else {
        return Ok(Vec::new());
    };
    let key = provider.key();
    let mut options = Vec::new();
    if state.app.is_enabled(key) {
        let raw = module_settings(&state.db()?, key).await?;
        let login = LoginSettings::from_value(&raw);
        if login.client_id.is_none() || !login.matches_email(&email) {
            return Ok(options);
        }
        let mut url = start_path(key);
        url.push('?');
        url.push_str(
            &form_urlencoded::Serializer::new(String::new())
                .append_pair(LOGIN_HINT, &email)
                .finish(),
        );
        options.push(LoginOption {
            provider: key.to_owned(),
            label: provider.label(&raw),
            url,
            required: login.require_sso,
        });
    }
    Ok(options)
}

async fn begin_login(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    query: &StartQuery,
    context: &ClientContext,
) -> Result<String, Failure> {
    let key = provider.key();
    limit(key, "sso_start", context).await?;
    let raw = support::settings(ctx, key).await?;
    let login = LoginSettings::from_value(&raw);
    let client_id = login.client_id.clone().ok_or(Failure(NOT_CONFIGURED))?;
    let email = query.login_hint.as_deref().and_then(normalize_email);
    if let Some(email) = &email
        && !login.domains.is_empty()
        && !login.matches_email(email)
    {
        return Err(Failure(DOMAIN_NOT_ALLOWED));
    }
    let endpoints = provider.endpoints(&raw).await?;
    let redirect = sanitize_redirect(query.redirect.as_deref());
    let fresh = states::create(ctx, key, PURPOSE_LOGIN, None, redirect.as_deref()).await?;
    let redirect_uri = callback_url(ctx, key);
    let mut extra = provider.login_params();
    if let Some(email) = email {
        extra.push((LOGIN_HINT, email));
    }
    Ok(oauth::authorization_url(
        &endpoints.authorization_endpoint,
        &AuthorizationRequest {
            client_id: &client_id,
            redirect_uri: &redirect_uri,
            scopes: provider.login_scopes(),
            state: &fresh.state,
            nonce: &fresh.nonce,
            code_verifier: &fresh.code_verifier,
            extra,
        },
    ))
}

pub async fn start_login(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    query: StartQuery,
    context: ClientContext,
) -> Response {
    match begin_login(ctx, provider, &query, &context).await {
        Ok(url) => redirect_to(&url),
        Err(Failure(code)) => {
            tracing::info!(
                module = provider.key(),
                code,
                "single sign-on start refused"
            );
            redirect_to(&login_error_path(code))
        }
    }
}

pub async fn exchange(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    state: &StoredState,
    code: &str,
) -> Result<Exchanged, AppError> {
    let key = provider.key();
    let raw = support::settings(ctx, key).await?;
    let credentials = credentials(ctx, key, &raw).await?;
    let endpoints = provider.endpoints(&raw).await?;
    let tokens = oauth::exchange_code(
        &endpoints.token_endpoint,
        &credentials,
        code,
        &callback_url(ctx, key),
        &state.code_verifier,
    )
    .await?;
    let claims = match tokens.id_token.as_deref() {
        Some(id_token) => {
            let claims = IdTokenClaims::decode(id_token)?;
            claims.validate(&credentials.client_id, &state.nonce, Utc::now().timestamp())?;
            let issuer_ok = claims
                .issuer()
                .is_some_and(|issuer| provider.issuer_allowed(issuer, &endpoints, &raw));
            if !issuer_ok {
                return Err(failure(
                    oauth::INVALID_TOKEN,
                    "the id token was issued by an unexpected issuer",
                ));
            }
            Some(claims)
        }
        None => None,
    };
    Ok(Exchanged { tokens, claims })
}

async fn finish_login(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    state: &StoredState,
    code: &str,
    context: &ClientContext,
) -> Result<Response, Failure> {
    let key = provider.key();
    let exchanged = exchange(ctx, provider, state, code).await?;
    let claims = exchanged.claims.ok_or_else(|| {
        failure(
            oauth::INVALID_TOKEN,
            "the provider did not return an id token",
        )
    })?;
    let raw = support::settings(ctx, key).await?;
    let email = provider.verified_email(&claims, &raw)?;
    let email = normalize_email(&email).ok_or(Failure(EMAIL_NOT_VERIFIED))?;
    let login = LoginSettings::from_value(&raw);
    if !login.domains.is_empty() && !login.matches_email(&email) {
        return Err(Failure(DOMAIN_NOT_ALLOWED));
    }
    let user = MySqlUserRepository::new(ctx.pool().clone())
        .find_by_email(&email)
        .await?
        .filter(|user| user.is_active())
        .ok_or(Failure(NO_ACCOUNT))?;
    let user_id = uuid::Uuid::parse_str(user.id.as_str()).map_err(|_| Failure(SSO_FAILED))?;
    let signed = sign_in(
        ctx,
        key,
        SignInRequest {
            user_id,
            method: key.to_owned(),
            ip: context.ip.clone(),
            user_agent: context.user_agent.clone(),
        },
    )
    .await?;
    let target = if signed.two_factor_required {
        match &state.redirect_path {
            Some(path) => format!(
                "{TWO_FACTOR_PATH}?{}",
                form_urlencoded::Serializer::new(String::new())
                    .append_pair("redirect", path)
                    .finish()
            ),
            None => TWO_FACTOR_PATH.to_owned(),
        }
    } else {
        state
            .redirect_path
            .clone()
            .unwrap_or_else(|| "/".to_owned())
    };
    let mut response = redirect_to(&target);
    let cookie = HeaderValue::from_str(&signed.set_cookie).map_err(|_| Failure(SSO_FAILED))?;
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

pub async fn handle_callback(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
    query: CallbackQuery,
    context: ClientContext,
) -> Response {
    let key = provider.key();
    if let Err(error) = limit(key, "sso_callback", &context).await {
        return redirect_to(&login_error_path(failure_code(&error)));
    }
    let state = match states::consume(ctx, key, query.state.as_deref().unwrap_or_default()).await {
        Ok(Some(state)) => state,
        Ok(None) => return redirect_to(&login_error_path(INVALID_STATE)),
        Err(error) => return redirect_to(&login_error_path(failure_code(&error))),
    };
    if state.purpose != PURPOSE_LOGIN {
        let code = match (&query.error, &query.code) {
            (None, Some(code)) if !code.is_empty() => code.clone(),
            _ => String::new(),
        };
        return provider.complete_purpose(ctx, &state, &code).await;
    }
    if query.error.is_some() {
        return redirect_to(&login_error_path(ACCESS_DENIED));
    }
    let Some(code) = query
        .code
        .filter(|code| !code.is_empty() && code.len() <= 4096)
    else {
        return redirect_to(&login_error_path(INVALID_REQUEST));
    };
    match finish_login(ctx, provider, &state, &code, &context).await {
        Ok(response) => response,
        Err(Failure(code)) => {
            tracing::info!(module = key, code, "single sign-on refused");
            redirect_to(&login_error_path(code))
        }
    }
}
