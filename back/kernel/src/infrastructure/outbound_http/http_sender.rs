use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};

use crate::application::outbound_http::{
    DEFAULT_CONTENT_TYPE, HttpMethod, MAX_RESPONSE_BODY_BYTES, OutboundRequest, OutboundResponse,
    OutboundSender, OutboundUrlGuard, UrlRejection, is_reserved_transport_header,
};
use crate::errors::AppError;
use crate::infrastructure::outbound_http::network::{
    GuardedResolver, NetworkUrlGuard, OutboundNetworkPolicy,
};

const USER_AGENT_VALUE: &str = "Aphel/1.0";
const MAX_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct HttpOutboundSender {
    client: Client,
    guard: NetworkUrlGuard,
    timeout: Duration,
}

impl HttpOutboundSender {
    pub(crate) fn new(policy: OutboundNetworkPolicy, timeout: Duration) -> Result<Self, AppError> {
        let client = Client::builder()
            .dns_resolver(GuardedResolver::shared(policy))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(timeout)
            .connect_timeout(timeout.min(MAX_CONNECT_TIMEOUT))
            .build()
            .map_err(|error| {
                AppError::Configuration(format!("could not build the webhook HTTP client: {error}"))
            })?;
        Ok(Self {
            client,
            guard: NetworkUrlGuard::new(policy),
            timeout,
        })
    }

    fn describe(&self, error: &reqwest::Error) -> String {
        if error.is_timeout() {
            format!("timed out after {} s", self.timeout.as_secs())
        } else if error.is_connect() {
            "could not connect to the endpoint".to_owned()
        } else if error.is_redirect() {
            "redirects are not followed".to_owned()
        } else {
            let mut message = "request failed".to_owned();
            let mut source = std::error::Error::source(error);
            while let Some(inner) = source {
                message = format!("request failed: {inner}");
                source = inner.source();
            }
            message
        }
    }
}

fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}

async fn read_limited(mut response: reqwest::Response) -> String {
    let mut buffer: Vec<u8> = Vec::new();
    while buffer.len() < MAX_RESPONSE_BODY_BYTES {
        match response.chunk().await {
            Ok(Some(chunk)) => buffer.extend_from_slice(&chunk),
            Ok(None) | Err(_) => break,
        }
    }
    buffer.truncate(MAX_RESPONSE_BODY_BYTES);
    let text = String::from_utf8_lossy(&buffer).into_owned();
    text.trim_end_matches('\u{FFFD}').to_owned()
}

#[async_trait]
impl OutboundSender for HttpOutboundSender {
    async fn send(&self, request: &OutboundRequest) -> OutboundResponse {
        let started = Instant::now();
        if let Err(rejection) = self.guard.check(&request.url).await {
            let message = match rejection {
                UrlRejection::Invalid(message) | UrlRejection::NotAllowed(message) => message,
            };
            return OutboundResponse {
                status: None,
                body: None,
                duration_ms: elapsed_ms(started),
                error: Some(format!("URL not allowed: {message}")),
            };
        }
        let method = match request.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self.client.request(method, &request.url);
        for (name, value) in &request.headers {
            if !is_reserved_transport_header(name) {
                builder = builder.header(name.trim(), value);
            }
        }
        builder = builder.header(USER_AGENT, USER_AGENT_VALUE);
        if request.method.has_body() {
            builder = builder
                .header(
                    CONTENT_TYPE,
                    request
                        .content_type
                        .as_deref()
                        .unwrap_or(DEFAULT_CONTENT_TYPE),
                )
                .body(request.body.clone());
        }
        let result = builder.send().await;
        match result {
            Ok(response) => {
                let status = response.status().as_u16();
                let body = read_limited(response).await;
                OutboundResponse {
                    status: Some(status),
                    body: (!body.is_empty()).then_some(body),
                    duration_ms: elapsed_ms(started),
                    error: None,
                }
            }
            Err(error) => OutboundResponse {
                status: None,
                body: None,
                duration_ms: elapsed_ms(started),
                error: Some(self.describe(&error)),
            },
        }
    }
}
