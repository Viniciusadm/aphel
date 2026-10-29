use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::errors::AppError;

pub(crate) const PROVIDER_ERROR: &str = "ai_provider_error";
pub(crate) const PROVIDER_AUTH_FAILED: &str = "ai_provider_auth_failed";
pub(crate) const PROVIDER_RATE_LIMITED: &str = "ai_provider_rate_limited";
pub(crate) const PROVIDER_TIMEOUT: &str = "ai_provider_timeout";
pub(crate) const REFUSED: &str = "ai_refused";
pub(crate) const EMPTY_RESPONSE: &str = "ai_empty_response";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiRole {
    User,
    Assistant,
}

impl AiRole {
    fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AiMessage {
    pub(crate) role: AiRole,
    pub(crate) content: String,
}

impl AiMessage {
    pub(crate) fn user(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::User,
            content: content.into(),
        }
    }

    pub(crate) fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AiRequest {
    pub(crate) system: String,
    pub(crate) messages: Vec<AiMessage>,
    pub(crate) max_tokens: u32,
    pub(crate) temperature: Option<f32>,
}

impl AiRequest {
    pub(crate) fn new(
        system: impl Into<String>,
        messages: Vec<AiMessage>,
        max_tokens: u32,
    ) -> Self {
        Self {
            system: system.into(),
            messages,
            max_tokens,
            temperature: None,
        }
    }

    pub(crate) fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AiResponse {
    pub(crate) text: String,
    pub(crate) input_tokens: Option<u32>,
    pub(crate) output_tokens: Option<u32>,
}

#[async_trait]
pub(crate) trait AiProvider: Send + Sync {
    fn name(&self) -> &'static str;

    fn model(&self) -> &str;

    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AppError>;
}

fn client() -> Result<&'static reqwest::Client, AppError> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(|_| AppError::Internal("could not build the AI HTTP client".to_owned()))?;
    Ok(CLIENT.get_or_init(|| client))
}

fn unavailable(code: &'static str, message: impl Into<String>) -> AppError {
    AppError::unavailable_code(code, message)
}

fn transport_error(error: &reqwest::Error) -> AppError {
    if error.is_timeout() {
        unavailable(PROVIDER_TIMEOUT, "the AI provider did not answer in time")
    } else {
        unavailable(PROVIDER_ERROR, "could not reach the AI provider")
    }
}

fn status_error(status: reqwest::StatusCode) -> AppError {
    match status.as_u16() {
        401 | 403 => unavailable(
            PROVIDER_AUTH_FAILED,
            "the AI provider rejected the configured credentials",
        ),
        429 => AppError::too_many_requests_code(
            PROVIDER_RATE_LIMITED,
            "the AI provider is rate limiting this organization, try again later",
        ),
        code => unavailable(
            PROVIDER_ERROR,
            format!("the AI provider answered HTTP {code}"),
        ),
    }
}

async fn send(request: reqwest::RequestBuilder) -> Result<Value, AppError> {
    let response = request
        .send()
        .await
        .map_err(|error| transport_error(&error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status));
    }
    response
        .json::<Value>()
        .await
        .map_err(|_| unavailable(PROVIDER_ERROR, "the AI provider sent an unreadable answer"))
}

fn token_count(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|count| u32::try_from(count).ok())
}

fn non_empty(text: String) -> Result<String, AppError> {
    if text.trim().is_empty() {
        Err(unavailable(
            EMPTY_RESPONSE,
            "the AI provider returned an empty answer",
        ))
    } else {
        Ok(text)
    }
}

fn without_reasoning(text: &str) -> &str {
    let trimmed = text.trim_start();
    match (trimmed.starts_with("<think>"), trimmed.find("</think>")) {
        (true, Some(end)) => trimmed[end + "</think>".len()..].trim_start(),
        _ => text,
    }
}

pub(crate) struct AnthropicProvider {
    base_url: String,
    model: String,
    api_key: String,
}

impl AnthropicProvider {
    pub(crate) const NAME: &'static str = "anthropic";
    pub(crate) const DEFAULT_BASE_URL: &'static str = "https://api.anthropic.com";

    pub(crate) fn new(base_url: String, model: String, api_key: String) -> Self {
        Self {
            base_url,
            model,
            api_key,
        }
    }

    fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{base}/messages")
        } else {
            format!("{base}/v1/messages")
        }
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AppError> {
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|message| json!({ "role": message.role.as_str(), "content": message.content }))
            .collect();
        let body = json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "system": request.system,
            "messages": messages,
        });
        let answer = send(
            client()?
                .post(self.endpoint())
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("content-type", "application/json")
                .json(&body),
        )
        .await?;
        if answer.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
            return Err(AppError::validation_code(
                REFUSED,
                "the AI model declined this request",
            ));
        }
        let text = answer
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|block| block.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();
        let usage = answer.get("usage").cloned().unwrap_or(Value::Null);
        Ok(AiResponse {
            text: non_empty(text)?,
            input_tokens: token_count(&usage, "input_tokens"),
            output_tokens: token_count(&usage, "output_tokens"),
        })
    }
}

pub(crate) struct OpenAiCompatibleProvider {
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub(crate) const NAME: &'static str = "openai_compatible";
    pub(crate) const DEFAULT_BASE_URL: &'static str = "https://api.openai.com/v1";

    pub(crate) fn new(base_url: String, model: String, api_key: Option<String>) -> Self {
        Self {
            base_url,
            model,
            api_key,
        }
    }
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AppError> {
        let mut messages = vec![json!({ "role": "system", "content": request.system })];
        messages.extend(
            request.messages.iter().map(
                |message| json!({ "role": message.role.as_str(), "content": message.content }),
            ),
        );
        let mut body = json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": messages,
        });
        if let Some(temperature) = request.temperature {
            body["temperature"] = json!(temperature);
        }
        let mut http = client()?
            .post(format!(
                "{}/chat/completions",
                self.base_url.trim_end_matches('/')
            ))
            .header("content-type", "application/json")
            .json(&body);
        if let Some(key) = self.api_key.as_deref().filter(|key| !key.is_empty()) {
            http = http.bearer_auth(key);
        }
        let answer = send(http).await?;
        let choice = answer
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .cloned()
            .unwrap_or(Value::Null);
        if choice.get("finish_reason").and_then(Value::as_str) == Some("content_filter") {
            return Err(AppError::validation_code(
                REFUSED,
                "the AI model declined this request",
            ));
        }
        let text = choice
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let usage = answer.get("usage").cloned().unwrap_or(Value::Null);
        Ok(AiResponse {
            text: non_empty(without_reasoning(text).to_owned())?,
            input_tokens: token_count(&usage, "prompt_tokens"),
            output_tokens: token_count(&usage, "completion_tokens"),
        })
    }
}
