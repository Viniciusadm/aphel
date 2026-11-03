use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::errors::{internal, unavailable};
use aphel_kernel::errors::AppError;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
pub const CLOCK_SKEW_SECONDS: i64 = 300;
pub const DISCOVERY_TTL: Duration = Duration::from_secs(3600);

pub const PROVIDER_UNAVAILABLE: &str = "provider_unavailable";
pub const TOKEN_EXCHANGE_FAILED: &str = "token_exchange_failed";
pub const INVALID_GRANT: &str = "invalid_grant";
pub const INVALID_TOKEN: &str = "invalid_token";

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

pub fn random_token(bytes: usize) -> Result<String, AppError> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer)
        .map_err(|error| internal(format!("random generator failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

pub fn sha256_hex(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEndpoints {
    pub issuer: Option<String>,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationRequest<'a> {
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub scopes: &'a [&'a str],
    pub state: &'a str,
    pub nonce: &'a str,
    pub code_verifier: &'a str,
    pub extra: Vec<(&'static str, String)>,
}

pub fn authorization_url(endpoint: &str, request: &AuthorizationRequest<'_>) -> String {
    let mut query = form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("response_type", "code")
        .append_pair("client_id", request.client_id)
        .append_pair("redirect_uri", request.redirect_uri)
        .append_pair("scope", &request.scopes.join(" "))
        .append_pair("state", request.state)
        .append_pair("nonce", request.nonce)
        .append_pair("code_challenge", &pkce_challenge(request.code_verifier))
        .append_pair("code_challenge_method", "S256");
    for (key, value) in &request.extra {
        query.append_pair(key, value);
    }
    let separator = if endpoint.contains('?') { '&' } else { '?' };
    format!("{endpoint}{separator}{}", query.finish())
}

#[derive(Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
}

impl std::fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenResponse")
            .field("expires_in", &self.expires_in)
            .finish_non_exhaustive()
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct IdTokenClaims {
    pub raw: Map<String, Value>,
}

impl IdTokenClaims {
    pub fn decode(id_token: &str) -> Result<Self, AppError> {
        let invalid = || AppError::validation_code(INVALID_TOKEN, "the id token is malformed");
        let mut parts = id_token.split('.');
        let (Some(_header), Some(payload), Some(_signature), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        let bytes = URL_SAFE_NO_PAD
            .decode(payload.trim_end_matches('='))
            .map_err(|_| invalid())?;
        match serde_json::from_slice::<Value>(&bytes).map_err(|_| invalid())? {
            Value::Object(raw) => Ok(Self { raw }),
            _ => Err(invalid()),
        }
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        self.raw
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn flag(&self, key: &str) -> bool {
        match self.raw.get(key) {
            Some(Value::Bool(value)) => *value,
            Some(Value::String(value)) => value.eq_ignore_ascii_case("true"),
            _ => false,
        }
    }

    pub fn issuer(&self) -> Option<&str> {
        self.text("iss")
    }

    pub fn audience_contains(&self, client_id: &str) -> bool {
        match self.raw.get("aud") {
            Some(Value::String(audience)) => audience == client_id,
            Some(Value::Array(audiences)) => audiences
                .iter()
                .any(|audience| audience.as_str() == Some(client_id)),
            _ => false,
        }
    }

    pub fn validate(&self, client_id: &str, nonce: &str, now: i64) -> Result<(), AppError> {
        let invalid = |message: &str| AppError::validation_code(INVALID_TOKEN, message.to_owned());
        if !self.audience_contains(client_id) {
            return Err(invalid("the id token was issued to another client"));
        }
        if let Some(Value::Array(audiences)) = self.raw.get("aud")
            && audiences.len() > 1
            && self.text("azp") != Some(client_id)
        {
            return Err(invalid("the id token has an unexpected authorized party"));
        }
        let expires_at = self
            .raw
            .get("exp")
            .and_then(Value::as_i64)
            .ok_or_else(|| invalid("the id token has no expiration"))?;
        if expires_at + CLOCK_SKEW_SECONDS < now {
            return Err(invalid("the id token expired"));
        }
        if let Some(issued_at) = self.raw.get("iat").and_then(Value::as_i64)
            && issued_at - CLOCK_SKEW_SECONDS > now
        {
            return Err(invalid("the id token was issued in the future"));
        }
        if self.text("nonce") != Some(nonce) {
            return Err(invalid("the id token nonce does not match"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
}

pub fn discovery_url(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() || !trimmed.starts_with("https://") {
        return None;
    }
    if trimmed.ends_with("/.well-known/openid-configuration") {
        Some(trimmed.to_owned())
    } else {
        Some(format!("{trimmed}/.well-known/openid-configuration"))
    }
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
