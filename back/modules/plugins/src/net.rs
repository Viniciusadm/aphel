use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Client, Method};

use crate::registry::LoadedPlugin;
use crate::runtime::wit;
use aphel_kernel::application::webhooks::outgoing::UrlRejection;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::webhooks::outgoing::network::parse_webhook_url;
use aphel_kernel::infrastructure::webhooks::outgoing::{GuardedResolver, WebhookNetworkPolicy};
use aphel_kernel::state::runtime_state;

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
    let policy = WebhookNetworkPolicy::new(allow_private_networks, false);
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
    let state = runtime_state()?;
    let config = state.configuration().webhooks_config(&state.webhooks);
    let policy = WebhookNetworkPolicy::new(config.allow_private_networks, config.require_https);
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
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining < MIN_REMAINING {
        return Err(failed(
            "the plugin call has no time left for an HTTP request",
        ));
    }
    let method = Method::from_bytes(request.method.to_ascii_uppercase().as_bytes())
        .map_err(|_| AppError::Validation("the HTTP method is invalid".to_owned()))?;
    let mut builder = client(config.allow_private_networks)?
        .request(method, url)
        .timeout(remaining);
    for (name, value) in &request.headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| AppError::Validation(format!("the header name {name} is invalid")))?;
        if WITHHELD_HEADERS.contains(&name.as_str()) {
            continue;
        }
        let value = HeaderValue::from_str(value)
            .map_err(|_| AppError::Validation(format!("the header {name} has an invalid value")))?;
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
