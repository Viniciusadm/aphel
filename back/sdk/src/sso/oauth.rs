use crate::AppError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
pub const CLOCK_SKEW_SECONDS: i64 = 300;

pub const PROVIDER_UNAVAILABLE: &str = "provider_unavailable";
pub const TOKEN_EXCHANGE_FAILED: &str = "token_exchange_failed";
pub const INVALID_GRANT: &str = "invalid_grant";
pub const INVALID_TOKEN: &str = "invalid_token";

pub fn random_token(bytes: usize) -> Result<String, AppError> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer)
        .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
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

#[async_trait::async_trait]
pub trait OAuthProtocol: Send + Sync {
    fn http_client(&self) -> Result<reqwest::Client, AppError>;
    async fn exchange_code(
        &self,
        endpoint: &str,
        credentials: &ClientCredentials,
        code: &str,
        redirect_uri: &str,
        verifier: &str,
    ) -> Result<TokenResponse, AppError>;
    async fn refresh_access_token(
        &self,
        endpoint: &str,
        credentials: &ClientCredentials,
        refresh_token: &str,
        scope: Option<&str>,
    ) -> Result<TokenResponse, AppError>;
    async fn discover(&self, url: &str) -> Result<ProviderEndpoints, AppError>;
    async fn discover_cached(&self, url: &str) -> Result<ProviderEndpoints, AppError>;
}
fn protocol() -> Result<std::sync::Arc<dyn OAuthProtocol>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn OAuthProtocol>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "oauth_protocol_unavailable",
                "OAuth protocol support is not enabled",
            )
        })
}
pub fn http_client() -> Result<reqwest::Client, AppError> {
    protocol()?.http_client()
}
pub async fn exchange_code(
    endpoint: &str,
    credentials: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenResponse, AppError> {
    protocol()?
        .exchange_code(endpoint, credentials, code, redirect_uri, verifier)
        .await
}
pub async fn refresh_access_token(
    endpoint: &str,
    credentials: &ClientCredentials,
    refresh_token: &str,
    scope: Option<&str>,
) -> Result<TokenResponse, AppError> {
    protocol()?
        .refresh_access_token(endpoint, credentials, refresh_token, scope)
        .await
}
pub async fn discover(url: &str) -> Result<ProviderEndpoints, AppError> {
    protocol()?.discover(url).await
}
pub async fn discover_cached(url: &str) -> Result<ProviderEndpoints, AppError> {
    protocol()?.discover_cached(url).await
}
