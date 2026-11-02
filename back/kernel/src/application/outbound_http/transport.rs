use std::sync::Arc;

use async_trait::async_trait;

pub const MAX_RESPONSE_BODY_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    Get,
    #[default]
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
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

pub const DEFAULT_CONTENT_TYPE: &str = "application/json";
pub fn is_reserved_transport_header(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "content-type"
            | "user-agent"
            | "content-length"
            | "host"
            | "connection"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutboundRequest {
    pub url: String,
    pub body: String,
    pub method: HttpMethod,
    pub headers: Vec<(String, String)>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundResponse {
    pub status: Option<u16>,
    pub body: Option<String>,
    pub duration_ms: u32,
    pub error: Option<String>,
}

impl OutboundResponse {
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
pub trait OutboundUrlGuard: Send + Sync {
    async fn check(&self, url: &str) -> Result<(), UrlRejection>;
}

#[async_trait]
pub trait OutboundSender: Send + Sync {
    async fn send(&self, request: &OutboundRequest) -> OutboundResponse;
}

#[async_trait]
impl<S> OutboundSender for Arc<S>
where
    S: OutboundSender + ?Sized,
{
    async fn send(&self, request: &OutboundRequest) -> OutboundResponse {
        (**self).send(request).await
    }
}
