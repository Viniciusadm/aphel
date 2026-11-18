use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Client, Method};

use crate::registry::LoadedPlugin;
use crate::runtime::wit;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::outbound_http::UrlRejection;
use aphel_sdk::platform::infrastructure::outbound_http::network::parse_webhook_url;
use aphel_sdk::platform::infrastructure::outbound_http::{GuardedResolver, OutboundNetworkPolicy};

pub const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 5 * 1024 * 1024;
const MIN_REMAINING: Duration = Duration::from_millis(100);
const MAX_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HOST_NOT_ALLOWED: &str = "plugin_host_not_allowed";
const HTTP_FAILED: &str = "plugin_http_failed";
const WITHHELD_HEADERS: &[&str] = &[
    "host",
    "content-length",
    "connection",
    "transfer-encoding",
    "proxy-authorization",
    "upgrade",
];

fn client(allow_private_networks: bool) -> Result<&'static Client, AppError> {
    static PUBLIC: OnceLock<Client> = OnceLock::new();
    static PRIVATE: OnceLock<Client> = OnceLock::new();
    let cell = if allow_private_networks {
        &PRIVATE
    } else {
        &PUBLIC
    };
    if let Some(client) = cell.get() {
        return Ok(client);
    }
    let policy = OutboundNetworkPolicy::new(allow_private_networks, false);
    let built = Client::builder()
        .dns_resolver(GuardedResolver::shared(policy))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(MAX_CONNECT_TIMEOUT)
        .build()
        .map_err(|error| {
            AppError::Configuration(format!("could not build the plugin HTTP client: {error}"))
        })?;
    Ok(cell.get_or_init(|| built))
}

fn failed(message: impl Into<String>) -> AppError {
    AppError::unavailable_code(HTTP_FAILED, message)
}

pub async fn fetch(
    plugin: &LoadedPlugin,
    deadline: Instant,
    request: wit::HttpRequest,
) -> Result<wit::HttpResponse, AppError> {
    let state = aphel_sdk::runtime()?;
    let config = state
        .configuration()
        .outbound_http_config(state.outbound_http_settings());
    let policy = OutboundNetworkPolicy::new(config.allow_private_networks, config.require_https);
    let url = parse_webhook_url(&request.url, &policy).map_err(|rejection| match rejection {
        UrlRejection::Invalid(message) | UrlRejection::NotAllowed(message) => {
            AppError::validation_code(
                HOST_NOT_ALLOWED,
                format!("the URL is not allowed: {message}"),
            )
        }
    })?;
    let host = url.host_str().unwrap_or_default();
    if !plugin.manifest.host_allowed(host) {
        return Err(AppError::forbidden_code(
            HOST_NOT_ALLOWED,
            format!(
                "the plugin {} did not declare {host} in http.allowed_hosts",
                plugin.key
            ),
        ));
    }
    let identity_client = if let Some(name) = request.client_identity.as_deref() {
        let setting = identity_setting(&plugin.manifest, name, &url)?;
        let pem = state
            .secrets()
            .reveal(
                &aphel_sdk::platform::domain::access::AccessScope::system(),
                &crate::host::setting_secret_key(&plugin.key, setting),
            )
            .await?
            .ok_or_else(|| {
                AppError::Configuration("client identity is not configured".to_owned())
            })?;
        // Build per request: renewal/revocation takes effect without a shared identity cache.
        Some(
            Client::builder()
                .dns_resolver(GuardedResolver::shared(OutboundNetworkPolicy::new(
                    config.allow_private_networks,
                    false,
                )))
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .connect_timeout(MAX_CONNECT_TIMEOUT)
                .identity(parse_identity(pem.expose())?)
                .build()
                .map_err(|_| {
                    AppError::Configuration("could not build the mTLS client".to_owned())
                })?,
        )
    } else {
        None
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining < MIN_REMAINING {
        return Err(failed(
            "the plugin call has no time left for an HTTP request",
        ));
    }
    let method = Method::from_bytes(request.method.to_ascii_uppercase().as_bytes())
        .map_err(|_| AppError::Validation("the HTTP method is invalid".to_owned()))?;
    let selected_client = match identity_client.as_ref() {
        Some(client) => client,
        None => client(config.allow_private_networks)?,
    };
    let authentication = if let Some(name) = request.authentication.as_deref() {
        let setting = authentication_setting(&plugin.manifest, name, &url)?;
        let credential = state
            .secrets()
            .reveal(
                &aphel_sdk::platform::domain::access::AccessScope::system(),
                &crate::host::setting_secret_key(&plugin.key, setting),
            )
            .await?
            .ok_or_else(|| {
                AppError::Configuration("HTTP credential is not configured".to_owned())
            })?;
        Some(credential_header(credential.expose())?)
    } else {
        None
    };
    let mut builder = selected_client.request(method, url).timeout(remaining);
    for (name, value) in &request.headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| AppError::Validation(format!("the header name {name} is invalid")))?;
        if authentication
            .as_ref()
            .is_some_and(|(protected, _)| protected == name)
        {
            return Err(AppError::Validation(
                "request headers cannot override a credential header".to_owned(),
            ));
        }
        if WITHHELD_HEADERS.contains(&name.as_str()) {
            continue;
        }
        let value = HeaderValue::from_str(value)
            .map_err(|_| AppError::Validation(format!("the header {name} has an invalid value")))?;
        builder = builder.header(name, value);
    }
    if let Some((name, value)) = authentication {
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        if body.len() > MAX_REQUEST_BYTES {
            return Err(AppError::Validation(format!(
                "the request body exceeds {MAX_REQUEST_BYTES} bytes"
            )));
        }
        builder = builder.body(body);
    }
    let mut response = builder
        .send()
        .await
        .map_err(|error| failed(format!("the HTTP request failed: {error}")))?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| failed(format!("the HTTP response could not be read: {error}")))?
    {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(failed(format!(
                "the HTTP response exceeds {MAX_RESPONSE_BYTES} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(wit::HttpResponse {
        status,
        headers,
        body,
    })
}

/// Input is a PEM certificate chain followed by an unencrypted private key.
pub fn parse_identity(pem: &str) -> Result<reqwest::Identity, AppError> {
    let invalid = || {
        AppError::Validation(
            "invalid TLS identity: expected a PEM certificate chain and matching private key"
                .to_owned(),
        )
    };
    let identity = reqwest::Identity::from_pem(pem.as_bytes()).map_err(|_| invalid())?;
    Client::builder()
        .identity(identity.clone())
        .build()
        .map_err(|_| invalid())?;
    Ok(identity)
}

pub fn valid_tls_origin(origin: &str) -> bool {
    reqwest::Url::parse(origin).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some_and(|host| !host.contains('*'))
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

pub(crate) fn identity_setting<'a>(
    manifest: &'a crate::manifest::PluginManifest,
    name: &str,
    url: &reqwest::Url,
) -> Result<&'a crate::manifest::SettingSpec, AppError> {
    let setting = manifest
        .settings
        .iter()
        .find(|setting| {
            setting.key == name && setting.kind == crate::manifest::SETTING_TLS_IDENTITY
        })
        .ok_or_else(|| {
            AppError::forbidden_code(
                "plugin_tls_identity_not_declared",
                "client identity is not declared",
            )
        })?;
    authorize_identity(url, &setting.tls_origins)?;
    Ok(setting)
}

fn authorize_identity(url: &reqwest::Url, origins: &[String]) -> Result<(), AppError> {
    if url.scheme() == "https"
        && origins.iter().any(|origin| {
            valid_tls_origin(origin)
                && reqwest::Url::parse(origin).is_ok_and(|allowed| allowed.origin() == url.origin())
        })
    {
        return Ok(());
    }
    Err(AppError::forbidden_code(
        "plugin_tls_origin_not_allowed",
        "client identity is not allowed for this HTTPS origin",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_is_bound_to_exact_https_origin() {
        let origins = vec!["https://api.example.com".to_owned()];
        for allowed in ["https://api.example.com/a", "https://api.example.com:443/b"] {
            assert!(authorize_identity(&reqwest::Url::parse(allowed).unwrap(), &origins).is_ok());
        }
        for denied in [
            "http://api.example.com",
            "https://api.example.com:444",
            "https://other.example.com",
            "https://api.example.com.evil.test",
        ] {
            assert!(authorize_identity(&reqwest::Url::parse(denied).unwrap(), &origins).is_err());
        }
    }
    #[test]
    fn rejects_invalid_credentials_and_origin_declarations() {
        assert!(parse_identity("not a certificate").is_err());
        for origin in [
            "http://example.com",
            "https://user@example.com",
            "https://example.com/path",
            "https://example.com?x=1",
            "https://example.com#fragment",
            "https://*.example.com",
        ] {
            assert!(!valid_tls_origin(origin), "{origin}");
        }
    }
}

#[cfg(test)]
mod mtls_integration_tests {
    use super::*;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn server_requires_a_valid_client_certificate() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(CertificateDer::from(
                include_bytes!("../tests/fixtures/mtls/ca.der").to_vec(),
            ))
            .unwrap();
        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![CertificateDer::from(
                    include_bytes!("../tests/fixtures/mtls/server.der").to_vec(),
                )],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
                    include_bytes!("../tests/fixtures/mtls/server-key.der").to_vec(),
                )),
            )
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
            let mut authenticated = 0;
            for _ in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                if let Ok(mut stream) = acceptor.accept(stream).await {
                    let mut buffer = [0u8; 2048];
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0);
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                        )
                        .await
                        .unwrap();
                    authenticated += 1;
                }
            }
            authenticated
        });
        let root = reqwest::Certificate::from_pem(include_bytes!("../tests/fixtures/mtls/ca.pem"))
            .unwrap();
        let url = format!("https://{address}/");
        let anonymous = Client::builder()
            .no_proxy()
            .add_root_certificate(root.clone())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        assert!(anonymous.get(&url).send().await.is_err());
        let identity =
            parse_identity(include_str!("../tests/fixtures/mtls/client-identity.pem")).unwrap();
        let authenticated = Client::builder()
            .no_proxy()
            .add_root_certificate(root)
            .identity(identity)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        assert_eq!(
            authenticated
                .get(&url)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "ok"
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), server)
                .await
                .unwrap()
                .unwrap(),
            1
        );
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "scheme", rename_all = "snake_case", deny_unknown_fields)]
enum HttpCredential {
    Bearer { token: String },
    Basic { username: String, password: String },
    Header { name: String, value: String },
}

pub fn credential_header(json: &str) -> Result<(HeaderName, HeaderValue), AppError> {
    use base64::Engine;
    let invalid = || AppError::Validation("invalid HTTP credential".to_owned());
    let credential: HttpCredential = serde_json::from_str(json).map_err(|_| invalid())?;
    let (name, value) = match credential {
        HttpCredential::Bearer { token }
            if !token.is_empty() && !token.chars().any(char::is_whitespace) =>
        {
            ("authorization".to_owned(), format!("Bearer {token}"))
        }
        HttpCredential::Basic { username, password }
            if !username.is_empty() && !username.contains(':') =>
        {
            (
                "authorization".to_owned(),
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD
                        .encode(format!("{username}:{password}"))
                ),
            )
        }
        HttpCredential::Header { name, value } if !value.is_empty() => (name, value),
        _ => return Err(invalid()),
    };
    let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| invalid())?;
    if WITHHELD_HEADERS.contains(&name.as_str()) || name == "idempotency-key" {
        return Err(invalid());
    }
    let mut value = HeaderValue::from_str(&value).map_err(|_| invalid())?;
    value.set_sensitive(true);
    Ok((name, value))
}

pub(crate) fn authentication_setting<'a>(
    manifest: &'a crate::manifest::PluginManifest,
    name: &str,
    url: &reqwest::Url,
) -> Result<&'a crate::manifest::SettingSpec, AppError> {
    let setting = manifest
        .settings
        .iter()
        .find(|setting| {
            setting.key == name && setting.kind == crate::manifest::SETTING_HTTP_CREDENTIAL
        })
        .ok_or_else(|| {
            AppError::forbidden_code(
                "plugin_http_credential_not_declared",
                "HTTP credential is not declared",
            )
        })?;
    authorize_identity(url, &setting.http_origins)?;
    Ok(setting)
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    #[test]
    fn authentication_formats_and_header_injection() {
        let (_, bearer) = credential_header(r#"{"scheme":"bearer","token":"token"}"#).unwrap();
        assert_eq!(bearer, "Bearer token");
        assert!(bearer.is_sensitive());
        let (_, basic) =
            credential_header(r#"{"scheme":"basic","username":"key","password":""}"#).unwrap();
        assert_eq!(basic, "Basic a2V5Og==");
        for input in [
            r#"{"scheme":"header","name":"host","value":"evil.test"}"#,
            r#"{"scheme":"header","name":"idempotency-key","value":"replace"}"#,
            r#"{"scheme":"bearer","token":"bad\r\nheader"}"#,
            r#"{"scheme":"basic","username":"bad:name","password":""}"#,
        ] {
            assert!(credential_header(input).is_err());
        }
    }
}
