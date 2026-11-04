pub use aphel_sdk::sso::oauth::{
    AuthorizationRequest, CLOCK_SKEW_SECONDS, ClientCredentials, INVALID_GRANT, INVALID_TOKEN,
    IdTokenClaims, PROVIDER_UNAVAILABLE, ProviderEndpoints, TOKEN_EXCHANGE_FAILED, TokenResponse,
    authorization_url, discovery_url, pkce_challenge, random_token, sha256_hex,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::errors::{internal, unavailable};
use aphel_kernel::errors::AppError;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
pub const DISCOVERY_TTL: Duration = Duration::from_secs(3600);

pub fn http_client() -> Result<reqwest::Client, AppError> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(HTTP_TIMEOUT)
                .connect_timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(concat!("Aphel/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|error| error.to_string())
        })
        .clone()
        .map_err(internal)
}

#[derive(Deserialize)]
struct TokenError {
    #[serde(default)]
    error: Option<String>,
}

async fn token_request(
    token_endpoint: &str,
    form: &[(&str, &str)],
    failure_code: &'static str,
) -> Result<TokenResponse, AppError> {
    let response = http_client()?
        .post(token_endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(form)
        .send()
        .await
        .map_err(|_| unavailable("the identity provider could not be reached"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|_| unavailable("the identity provider answered with an unreadable body"))?;
    if status.is_success() {
        return serde_json::from_slice::<TokenResponse>(&body).map_err(|_| {
            AppError::validation_code(
                failure_code,
                "the identity provider answered without a token",
            )
        });
    }
    let error = serde_json::from_slice::<TokenError>(&body)
        .ok()
        .and_then(|error| error.error);
    tracing::warn!(
        status = status.as_u16(),
        provider_error = error.as_deref().unwrap_or("unknown"),
        "oauth token request refused"
    );
    if error.as_deref() == Some("invalid_grant") {
        return Err(AppError::validation_code(
            INVALID_GRANT,
            "the authorization was revoked or expired",
        ));
    }
    Err(AppError::validation_code(
        failure_code,
        "the identity provider refused the token request",
    ))
}

pub async fn exchange_code(
    token_endpoint: &str,
    credentials: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenResponse, AppError> {
    token_request(
        token_endpoint,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &credentials.client_id),
            ("client_secret", &credentials.client_secret),
            ("code_verifier", code_verifier),
        ],
        TOKEN_EXCHANGE_FAILED,
    )
    .await
}

pub async fn refresh_access_token(
    token_endpoint: &str,
    credentials: &ClientCredentials,
    refresh_token: &str,
    scopes: Option<&str>,
) -> Result<TokenResponse, AppError> {
    let mut form = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", credentials.client_id.as_str()),
        ("client_secret", credentials.client_secret.as_str()),
    ];
    if let Some(scopes) = scopes {
        form.push(("scope", scopes));
    }
    token_request(token_endpoint, &form, TOKEN_EXCHANGE_FAILED).await
}

#[derive(Debug, Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
}

pub async fn discover_cached(url: &str) -> Result<ProviderEndpoints, AppError> {
    type Cache = Mutex<HashMap<String, (Instant, ProviderEndpoints)>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(entries) = cache.lock()
        && let Some((fetched_at, endpoints)) = entries.get(url)
        && fetched_at.elapsed() < DISCOVERY_TTL
    {
        return Ok(endpoints.clone());
    }
    let endpoints = discover(url).await?;
    if let Ok(mut entries) = cache.lock() {
        if entries.len() > 256 {
            entries.clear();
        }
        entries.insert(url.to_owned(), (Instant::now(), endpoints.clone()));
    }
    Ok(endpoints)
}

pub async fn discover(url: &str) -> Result<ProviderEndpoints, AppError> {
    let unavailable = || unavailable("the identity provider discovery document is unavailable");
    let response = http_client()?
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| unavailable())?;
    if !response.status().is_success() {
        return Err(unavailable());
    }
    let document: DiscoveryDocument = response.json().await.map_err(|_| unavailable())?;
    let secure = |value: &str| value.starts_with("https://");
    if !secure(&document.authorization_endpoint) || !secure(&document.token_endpoint) {
        return Err(unavailable());
    }
    Ok(ProviderEndpoints {
        issuer: Some(document.issuer),
        authorization_endpoint: document.authorization_endpoint,
        token_endpoint: document.token_endpoint,
    })
}
