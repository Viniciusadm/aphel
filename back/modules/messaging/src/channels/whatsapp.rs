use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Map, Value, json};

use super::{
    ChannelContext, ChannelError, InboundMessage, MessagingChannel, OutgoingMessage, SentMessage,
    ValidatedConfig, http_error,
};
use crate::errors::WHATSAPP_WINDOW_CLOSED;
use crate::text::{self, normalize_phone};
use aphel_kernel::errors::FieldViolation;

pub(crate) const GRAPH_BASE_URL: &str = "https://graph.facebook.com/v21.0";
pub(crate) const ACCESS_TOKEN: &str = "access_token";
pub(crate) const APP_SECRET: &str = "app_secret";
pub(crate) const VERIFY_TOKEN: &str = "verify_token";
const MAX_BODY_CHARS: usize = 4096;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const WINDOW_CLOSED_CODES: &[i64] = &[131047];
const RETRYABLE_CODES: &[i64] = &[4, 80007, 130429, 131000, 131016, 131048, 131056, 133004];

pub(crate) struct WhatsappChannel;

fn client() -> Result<&'static reqwest::Client, ChannelError> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| {
            ChannelError::transient(
                "messaging_provider_error",
                "could not build the HTTP client",
            )
        })?;
    Ok(CLIENT.get_or_init(|| client))
}

fn config_text(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn phone_number_id(config: &Value) -> Result<String, ChannelError> {
    config_text(config, "phone_number_id").ok_or_else(|| {
        ChannelError::permanent(
            "messaging_channel_invalid",
            "the channel has no phone number id",
        )
    })
}

fn is_digits(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && value.chars().all(|c| c.is_ascii_digit())
}

fn provider_error(status: reqwest::StatusCode, body: &Value) -> ChannelError {
    let error = body.get("error");
    let code = error
        .and_then(|error| error.get("code"))
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let message = error
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(|message| text::single_line(message, 300))
        .unwrap_or_else(|| format!("the provider answered {}", status.as_u16()));
    if WINDOW_CLOSED_CODES.contains(&code) {
        return ChannelError::permanent(
            WHATSAPP_WINDOW_CLOSED,
            "more than 24 hours since the last customer message; WhatsApp only accepts template messages now",
        );
    }
    if code == 190 || status == reqwest::StatusCode::UNAUTHORIZED {
        return ChannelError::permanent(
            "whatsapp_auth_failed",
            "the access token was rejected by Meta",
        );
    }
    if RETRYABLE_CODES.contains(&code)
        || status.is_server_error()
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return ChannelError::transient("whatsapp_provider_error", message);
    }
    ChannelError::permanent("whatsapp_provider_error", message)
}

async fn read_json(
    response: reqwest::Response,
) -> Result<(reqwest::StatusCode, Value), ChannelError> {
    let status = response.status();
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    Ok((status, body))
}

#[async_trait]
impl MessagingChannel for WhatsappChannel {
    fn definition(&self) -> aphel_sdk::messaging::ChannelDefinition {
        serde_json::from_str(include_str!("../../locales/messaging-whatsapp.json"))
            .expect("valid channel definition")
    }
    fn key(&self) -> &'static str {
        "whatsapp"
    }
    fn contact_kind(&self) -> aphel_sdk::party::ContactPointKind {
        aphel_sdk::party::ContactPointKind::Phone
    }
    fn uses_references(&self) -> bool {
        false
    }
    fn reply_window(&self) -> Option<aphel_sdk::messaging::ReplyWindow> {
        Some(aphel_sdk::messaging::ReplyWindow {
            seconds: 86400,
            code: WHATSAPP_WINDOW_CLOSED,
            message: "the channel reply window has closed; an approved template is required",
        })
    }
    fn secret_names(&self) -> &'static [&'static str] {
        &[ACCESS_TOKEN, APP_SECRET, VERIFY_TOKEN]
    }

    fn required_secrets(&self) -> &'static [&'static str] {
        &[ACCESS_TOKEN, APP_SECRET, VERIFY_TOKEN]
    }

    fn validate_config(&self, config: &Value) -> ValidatedConfig {
        let mut violations = Vec::new();
        let phone_number_id = config_text(config, "phone_number_id").unwrap_or_default();
        if !is_digits(&phone_number_id, 32) {
            violations.push(FieldViolation::new(
                "config.phone_number_id",
                "informe o identificador numérico do número (Phone number ID) da Meta",
            ));
        }
        let business_account_id = config_text(config, "business_account_id");
        if business_account_id
            .as_deref()
            .is_some_and(|value| !is_digits(value, 32))
        {
            violations.push(FieldViolation::new(
                "config.business_account_id",
                "o identificador da conta do WhatsApp Business deve ser numérico",
            ));
        }
        let display_phone_number = config_text(config, "display_phone_number");
        let normalized_display = display_phone_number.as_deref().map(normalize_phone);
        if matches!(normalized_display, Some(None)) {
            violations.push(FieldViolation::new(
                "config.display_phone_number",
                "telefone inválido; use o formato internacional, ex.: +55 11 99999-0000",
            ));
        }
        let mut normalized = Map::new();
        normalized.insert("phone_number_id".to_owned(), json!(phone_number_id));
        normalized.insert("business_account_id".to_owned(), json!(business_account_id));
        normalized.insert(
            "display_phone_number".to_owned(),
            json!(normalized_display.flatten()),
        );
        ValidatedConfig {
            config: Value::Object(normalized),
            violations,
        }
    }

    fn max_body_chars(&self) -> usize {
        MAX_BODY_CHARS
    }

    fn address(&self, config: &Value) -> String {
        config_text(config, "display_phone_number")
            .or_else(|| config_text(config, "phone_number_id"))
            .unwrap_or_default()
    }

    async fn test(&self, context: &ChannelContext<'_>) -> Result<String, ChannelError> {
        let token = context.credentials.require(ACCESS_TOKEN)?;
        let id = phone_number_id(context.config)?;
        let response = client()?
            .get(format!(
                "{GRAPH_BASE_URL}/{id}?fields=display_phone_number,verified_name"
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|error| http_error(&error))?;
        let (status, body) = read_json(response).await?;
        if !status.is_success() {
            return Err(provider_error(status, &body));
        }
        let name = body
            .get("verified_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let number = body
            .get("display_phone_number")
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(text::single_line(&format!("{name} {number}"), 200))
    }

    async fn send(
        &self,
        context: &ChannelContext<'_>,
        message: &OutgoingMessage,
    ) -> Result<SentMessage, ChannelError> {
        let token = context.credentials.require(ACCESS_TOKEN)?;
        let id = phone_number_id(context.config)?;
        let to: String = message.to.chars().filter(char::is_ascii_digit).collect();
        let response = client()?
            .post(format!("{GRAPH_BASE_URL}/{id}/messages"))
            .bearer_auth(token)
            .json(&json!({
                "messaging_product": "whatsapp",
                "recipient_type": "individual",
                "to": to,
                "type": "text",
                "text": { "preview_url": false, "body": message.body },
            }))
            .send()
            .await
            .map_err(|error| http_error(&error))?;
        let (status, body) = read_json(response).await?;
        if !status.is_success() {
            return Err(provider_error(status, &body));
        }
        let external_id = body
            .get("messages")
            .and_then(Value::as_array)
            .and_then(|messages| messages.first())
            .and_then(|message| message.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ChannelError::transient(
                    "whatsapp_provider_error",
                    "the provider did not return a message id",
                )
            })?;
        Ok(SentMessage {
            external_id: external_id.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatusUpdate {
    pub(crate) external_id: String,
    pub(crate) status: String,
    pub(crate) at: Option<NaiveDateTime>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct WhatsappWebhook {
    pub(crate) messages: Vec<InboundMessage>,
    pub(crate) statuses: Vec<StatusUpdate>,
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_str)
}

fn message_body(message: &Value, kind: &str) -> String {
    let candidates: &[&[&str]] = match kind {
        "text" => &[&["text", "body"]],
        "button" => &[&["button", "text"]],
        "interactive" => &[
            &["interactive", "button_reply", "title"],
            &["interactive", "list_reply", "title"],
        ],
        "image" => &[&["image", "caption"]],
        "video" => &[&["video", "caption"]],
        "document" => &[&["document", "caption"], &["document", "filename"]],
        "reaction" => &[&["reaction", "emoji"]],
        "location" => &[&["location", "name"], &["location", "address"]],
        _ => &[],
    };
    let body = candidates
        .iter()
        .find_map(|path| str_at(message, path))
        .unwrap_or_default();
    let body = if kind == "location" && body.is_empty() {
        match (
            message.get("location").and_then(|l| l.get("latitude")),
            message.get("location").and_then(|l| l.get("longitude")),
        ) {
            (Some(latitude), Some(longitude)) => format!("{latitude}, {longitude}"),
            _ => String::new(),
        }
    } else {
        body.to_owned()
    };
    text::clean_body(&body)
}

pub(crate) fn parse_webhook(payload: &Value, phone_number_id: &str) -> WhatsappWebhook {
    let mut webhook = WhatsappWebhook::default();
    let changes = payload
        .get("entry")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("changes").and_then(Value::as_array))
        .flatten()
        .filter(|change| change.get("field").and_then(Value::as_str) == Some("messages"))
        .filter_map(|change| change.get("value"));
    for value in changes {
        if str_at(value, &["metadata", "phone_number_id"]) != Some(phone_number_id) {
            continue;
        }
        let profiles: Vec<(&str, &str)> = value
            .get("people")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|contact| {
                Some((
                    contact.get("wa_id").and_then(Value::as_str)?,
                    str_at(contact, &["profile", "name"])?,
                ))
            })
            .collect();
        for message in value
            .get("messages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(from)) = (
                message.get("id").and_then(Value::as_str),
                message.get("from").and_then(Value::as_str),
            ) else {
                continue;
            };
            let Some(peer) = normalize_phone(from) else {
                continue;
            };
            let kind = message
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unsupported");
            let kind =
                if kind.len() <= 32 && kind.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    kind
                } else {
                    "unsupported"
                };
            webhook.messages.push(InboundMessage {
                external_id: text::truncate(id, 255),
                from: peer,
                from_name: profiles
                    .iter()
                    .find(|(wa_id, _)| *wa_id == from)
                    .map(|(_, name)| text::single_line(name, 255))
                    .filter(|name| !name.is_empty()),
                subject: None,
                body: message_body(message, kind),
                message_type: kind.to_owned(),
                in_reply_to: str_at(message, &["context", "id"]).map(str::to_owned),
                references: Vec::new(),
                sent_at: message
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .and_then(|value| value.trim().parse::<i64>().ok()),
            });
        }
        for status in value
            .get("statuses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(state)) = (
                status.get("id").and_then(Value::as_str),
                status.get("status").and_then(Value::as_str),
            ) else {
                continue;
            };
            if !matches!(state, "sent" | "delivered" | "read" | "failed") {
                continue;
            }
            let error = status
                .get("errors")
                .and_then(Value::as_array)
                .and_then(|errors| errors.first())
                .map(|error| {
                    let code = error
                        .get("code")
                        .and_then(Value::as_i64)
                        .unwrap_or_default();
                    let title = error
                        .get("title")
                        .or_else(|| error.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    text::single_line(&format!("{code} {title}"), 300)
                });
            webhook.statuses.push(StatusUpdate {
                external_id: id.to_owned(),
                status: state.to_owned(),
                at: status
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .and_then(text::from_unix),
                error,
            });
        }
    }
    webhook
}
