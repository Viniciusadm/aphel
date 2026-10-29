use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

pub const MAX_RESPONSE_BODY_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebhookMethod {
    Get,
    #[default]
    Post,
    Put,
    Patch,
    Delete,
}

impl WebhookMethod {
    pub const ALL: [Self; 5] = [Self::Get, Self::Post, Self::Put, Self::Patch, Self::Delete];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|method| method.as_str().eq_ignore_ascii_case(value.trim()))
    }

    pub fn has_body(&self) -> bool {
        !matches!(self, Self::Get | Self::Delete)
    }
}

pub const DEFAULT_WEBHOOK_CONTENT_TYPE: &str = "application/json";
pub const RESERVED_WEBHOOK_HEADERS: [&str; 6] = [
    "content-type",
    "user-agent",
    "content-length",
    "host",
    "x-aphel-signature",
    "x-aphel-event",
];

pub fn is_reserved_webhook_header(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    RESERVED_WEBHOOK_HEADERS.contains(&name.as_str()) || name.starts_with("x-aphel-")
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WebhookRequest {
    pub url: String,
    pub delivery_id: Uuid,
    pub event_type: String,
    pub timestamp: i64,
    pub signature: String,
    pub body: String,
    pub method: WebhookMethod,
    pub headers: Vec<(String, String)>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookResponse {
    pub status: Option<u16>,
    pub body: Option<String>,
    pub duration_ms: u32,
    pub error: Option<String>,
}

impl WebhookResponse {
    pub fn succeeded(&self) -> bool {
        self.error.is_none()
            && self
                .status
                .is_some_and(|status| (200..300).contains(&status))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlRejection {
    Invalid(String),
    NotAllowed(String),
}

#[async_trait]
pub trait WebhookUrlGuard: Send + Sync {
    async fn check(&self, url: &str) -> Result<(), UrlRejection>;
}

#[async_trait]
pub trait WebhookSender: Send + Sync {
    async fn send(&self, request: &WebhookRequest) -> WebhookResponse;
}

#[async_trait]
impl<S> WebhookSender for Arc<S>
where
    S: WebhookSender + ?Sized,
{
    async fn send(&self, request: &WebhookRequest) -> WebhookResponse {
        (**self).send(request).await
    }
}
