use std::collections::BTreeMap;

use crate::store::imap::{FolderState, INBOX, ImapCursor};
use crate::{
    EmailAddress, FetchedEmail, IMAP_PROVIDER, MailProviderError, MailProviderErrorKind,
    MailboxAccount, MailboxCredentials, ProviderEmail, SharedMailboxProvider, SyncRequest,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use aphel_kernel::errors::FieldViolation;
use aphel_sdk::messaging::text::{self, normalize_email, normalize_message_id};
use aphel_sdk::messaging::{
    ChannelContext, ChannelError, FetchBatch, FetchState, InboundMessage, MessagingChannel,
    OutgoingMessage, SentMessage, ValidatedConfig,
};

pub(crate) const IMAP_PASSWORD: &str = "imap_password";
pub(crate) const SMTP_PASSWORD: &str = "smtp_password";
const MAX_BODY_CHARS: usize = 100_000;

pub(crate) struct EmailChannel;

#[derive(Debug, Clone)]
struct EmailConfig {
    address: String,
    display_name: Option<String>,
    imap_host: String,
    imap_port: u16,
    imap_username: String,
    smtp_host: String,
    smtp_port: u16,
    smtp_starttls: bool,
    smtp_username: String,
}

fn config_text(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn config_port(config: &Value, key: &str, default: u16) -> Option<u16> {
    match config.get(key) {
        None | Some(Value::Null) => Some(default),
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port > 0),
        Some(Value::String(text)) => text.trim().parse::<u16>().ok().filter(|port| *port > 0),
        _ => None,
    }
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && !host.starts_with(['.', '-'])
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn parse_config(config: &Value) -> Result<EmailConfig, ChannelError> {
    let invalid = || {
        ChannelError::permanent(
            "messaging_channel_invalid",
            "the e-mail channel configuration is incomplete",
        )
    };
    let imap_username = config_text(config, "imap_username").ok_or_else(invalid)?;
    Ok(EmailConfig {
        address: config_text(config, "address")
            .and_then(|address| normalize_email(&address))
            .ok_or_else(invalid)?,
        display_name: config_text(config, "display_name"),
        imap_host: config_text(config, "imap_host").ok_or_else(invalid)?,
        imap_port: config_port(config, "imap_port", 993).ok_or_else(invalid)?,
        smtp_host: config_text(config, "smtp_host").ok_or_else(invalid)?,
        smtp_port: config_port(config, "smtp_port", 465).ok_or_else(invalid)?,
        smtp_starttls: config_text(config, "smtp_security").as_deref() == Some("starttls"),
        smtp_username: config_text(config, "smtp_username")
            .unwrap_or_else(|| imap_username.clone()),
        imap_username,
    })
}

fn demo_mode() -> bool {
    std::env::var("DEMO_MODE")
        .is_ok_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes"))
}

fn provider() -> Result<SharedMailboxProvider, ChannelError> {
    crate::store::mail_providers()
        .find(IMAP_PROVIDER)
        .ok_or_else(|| {
            ChannelError::permanent(
                "messaging_provider_error",
                "the IMAP/SMTP mail provider is not available",
            )
        })
}

fn account(context: &ChannelContext<'_>) -> Result<(EmailConfig, MailboxAccount), ChannelError> {
    let config = parse_config(context.config)?;
    let imap_password = context.credentials.require(IMAP_PASSWORD)?;
    let mut credentials = Map::new();
    credentials.insert("password".to_owned(), json!(imap_password));
    if let Some(smtp_password) = context.credentials.get(SMTP_PASSWORD) {
        credentials.insert("smtp_password".to_owned(), json!(smtp_password));
    }
    let settings = json!({
        "address": config.address,
        "username": config.imap_username,
        "smtp_username": config.smtp_username,
        "imap_host": config.imap_host,
        "imap_port": config.imap_port,
        "imap_security": "tls",
        "smtp_host": config.smtp_host,
        "smtp_port": config.smtp_port,
        "smtp_security": if config.smtp_starttls { "starttls" } else { "tls" },
    });
    let account = MailboxAccount {
        mailbox_id: Uuid::nil(),
        user_id: Uuid::nil(),
        address: config.address.clone(),
        settings,
        credentials: MailboxCredentials(credentials),
        persistence: Default::default(),
    };
    Ok((config, account))
}

fn channel_error(error: MailProviderError) -> ChannelError {
    let message = error.message.clone();
    let smtp = message.contains("SMTP");
    let code = match error.kind {
        MailProviderErrorKind::Authentication if smtp => "smtp_auth_failed",
        MailProviderErrorKind::Authentication => "imap_auth_failed",
        MailProviderErrorKind::Rejected if smtp => "smtp_rejected",
        MailProviderErrorKind::Rejected | MailProviderErrorKind::Unsupported => {
            "messaging_channel_invalid"
        }
        MailProviderErrorKind::Connection if message.contains("in time") => {
            "messaging_provider_timeout"
        }
        MailProviderErrorKind::Connection if message.contains("TLS") => "messaging_tls_error",
        MailProviderErrorKind::Connection if smtp => "smtp_error",
        MailProviderErrorKind::Connection if message.contains("IMAP") => "imap_error",
        MailProviderErrorKind::Connection | MailProviderErrorKind::Internal => {
            "messaging_provider_unreachable"
        }
    };
    if error.is_retryable() {
        ChannelError::transient(code, message)
    } else {
        ChannelError::permanent(code, message)
    }
}

fn provider_uid(email: &FetchedEmail) -> Option<u64> {
    email
        .provider_message_id
        .as_deref()?
        .rsplit(':')
        .next()?
        .parse()
        .ok()
}

fn inbound(
    email: FetchedEmail,
    uid_validity: u64,
    uid: u64,
    own_address: &str,
) -> Option<InboundMessage> {
    let sender = email.from.as_ref()?;
    let from = normalize_email(&sender.address)?;
    if from == own_address {
        return None;
    }
    let truncated = email.text.is_none() && email.html.is_none() && email.attachments.is_empty();
    let external_id = email
        .message_id
        .as_deref()
        .and_then(normalize_message_id)
        .unwrap_or_else(|| format!("imap-{uid_validity}-{uid}"));
    let body = email
        .text
        .as_deref()
        .map(|body| text::truncate(&text::clean_body(body), MAX_BODY_CHARS))
        .unwrap_or_default();
    Some(InboundMessage {
        external_id: text::truncate(&external_id, 255),
        from,
        from_name: sender
            .name
            .as_deref()
            .map(|name| text::single_line(name, 255))
            .filter(|name| !name.is_empty()),
        subject: Some(text::single_line(&email.subject, 500)).filter(|subject| !subject.is_empty()),
        body,
        message_type: if truncated { "too_large" } else { "email" }.to_owned(),
        in_reply_to: email.in_reply_to.as_deref().and_then(normalize_message_id),
        references: email
            .references
            .iter()
            .filter_map(|id| normalize_message_id(id))
            .take(30)
            .collect(),
        sent_at: email.date.map(|date| date.timestamp()),
    })
}

fn cursor_for(state: FetchState) -> Value {
    let mut folders = BTreeMap::new();
    if let (Some(validity), Some(last)) = (state.uid_validity, state.last_uid)
        && let (Ok(uid_validity), Ok(last_uid)) = (u32::try_from(validity), u32::try_from(last))
    {
        folders.insert(
            INBOX.to_owned(),
            FolderState {
                uid_validity,
                last_uid,
            },
        );
    }
    serde_json::to_value(ImapCursor {
        folders,
        sent_folder: Some(INBOX.to_owned()),
    })
    .unwrap_or(Value::Null)
}

fn inbox_state(cursor: &Value) -> Option<FolderState> {
    serde_json::from_value::<ImapCursor>(cursor.clone())
        .ok()?
        .folders
        .get(INBOX)
        .copied()
}

#[async_trait]
impl MessagingChannel for EmailChannel {
    fn definition(&self) -> aphel_sdk::messaging::ChannelDefinition {
        serde_json::from_str(include_str!("../locales/messaging-email.json"))
            .expect("valid channel definition")
    }
    fn key(&self) -> &'static str {
        "email"
    }
    fn contact_kind(&self) -> aphel_sdk::party::ContactPointKind {
        aphel_sdk::party::ContactPointKind::Email
    }
    fn uses_references(&self) -> bool {
        true
    }
    fn secret_names(&self) -> &'static [&'static str] {
        &[IMAP_PASSWORD, SMTP_PASSWORD]
    }

    fn required_secrets(&self) -> &'static [&'static str] {
        &[IMAP_PASSWORD]
    }

    fn validate_config(&self, config: &Value) -> ValidatedConfig {
        let mut violations = Vec::new();
        let address = config_text(config, "address").and_then(|value| normalize_email(&value));
        if address.is_none() {
            violations.push(FieldViolation::new(
                "config.address",
                "informe o endereço de e-mail da caixa",
            ));
        }
        let display_name =
            config_text(config, "display_name").map(|name| text::single_line(&name, 120));
        let mut hosts = Map::new();
        for key in ["imap_host", "smtp_host"] {
            let host = config_text(config, key).map(|host| host.to_lowercase());
            if !host.as_deref().is_some_and(valid_host) {
                violations.push(FieldViolation::new(
                    format!("config.{key}"),
                    "informe o nome do servidor, ex.: imap.exemplo.com",
                ));
            }
            hosts.insert(key.to_owned(), json!(host));
        }
        let imap_port = config_port(config, "imap_port", 993);
        if imap_port.is_none() {
            violations.push(FieldViolation::new("config.imap_port", "porta inválida"));
        }
        let smtp_port = config_port(config, "smtp_port", 465);
        if smtp_port.is_none() {
            violations.push(FieldViolation::new("config.smtp_port", "porta inválida"));
        }
        let security = config_text(config, "smtp_security").unwrap_or_else(|| "tls".to_owned());
        if !matches!(security.as_str(), "tls" | "starttls") {
            violations.push(FieldViolation::new(
                "config.smtp_security",
                "use tls (porta 465) ou starttls (porta 587)",
            ));
        }
        let imap_username = config_text(config, "imap_username")
            .or_else(|| address.clone())
            .map(|name| text::truncate(&name, 320));
        let smtp_username =
            config_text(config, "smtp_username").map(|name| text::truncate(&name, 320));
        let mut normalized = hosts;
        normalized.insert("address".to_owned(), json!(address));
        normalized.insert("display_name".to_owned(), json!(display_name));
        normalized.insert("imap_port".to_owned(), json!(imap_port));
        normalized.insert("imap_username".to_owned(), json!(imap_username));
        normalized.insert("smtp_port".to_owned(), json!(smtp_port));
        normalized.insert("smtp_security".to_owned(), json!(security));
        normalized.insert("smtp_username".to_owned(), json!(smtp_username));
        ValidatedConfig {
            config: Value::Object(normalized),
            violations,
        }
    }

    fn max_body_chars(&self) -> usize {
        MAX_BODY_CHARS
    }

    fn address(&self, config: &Value) -> String {
        config_text(config, "address").unwrap_or_default()
    }

    async fn test(&self, context: &ChannelContext<'_>) -> Result<String, ChannelError> {
        let (_, account) = account(context)?;
        provider()?.test(&account).await.map_err(channel_error)?;
        Ok("IMAP e SMTP conectados".to_owned())
    }

    async fn send(
        &self,
        context: &ChannelContext<'_>,
        message: &OutgoingMessage,
    ) -> Result<SentMessage, ChannelError> {
        if demo_mode() {
            return Err(ChannelError::permanent(
                "demo_restricted",
                "sending e-mail is disabled in the demo",
            ));
        }
        let (config, account) = account(context)?;
        let outgoing = ProviderEmail {
            message_id: message.message_id.clone(),
            from: EmailAddress {
                address: config.address.clone(),
                name: config.display_name.clone(),
            },
            to: vec![EmailAddress {
                address: message.to.clone(),
                name: message.to_name.clone(),
            }],
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            subject: message.subject.clone().unwrap_or_default(),
            text: message.body.clone(),
            html: None,
            in_reply_to: message.in_reply_to.clone(),
            references: message.references.clone(),
            attachments: Vec::new(),
            headers: Vec::new(),
        };
        provider()?
            .send(&account, &outgoing)
            .await
            .map_err(|error| match error.kind {
                MailProviderErrorKind::Rejected if error.message.starts_with("invalid address") => {
                    ChannelError::permanent("messaging_invalid_recipient", "invalid e-mail address")
                }
                _ => channel_error(error),
            })?;
        Ok(SentMessage {
            external_id: message.message_id.clone(),
        })
    }

    async fn fetch(
        &self,
        context: &ChannelContext<'_>,
        state: FetchState,
        limit: usize,
    ) -> Result<FetchBatch, ChannelError> {
        let (config, account) = account(context)?;
        let batch = provider()?
            .sync_since(
                &account,
                SyncRequest {
                    cursor: Some(cursor_for(state)),
                    since: Utc::now() + Duration::days(1),
                    limit: limit.max(1),
                },
            )
            .await
            .map_err(channel_error)?;
        let Some(inbox) = inbox_state(&batch.cursor) else {
            return Ok(FetchBatch {
                state,
                messages: Vec::new(),
            });
        };
        let validity = u64::from(inbox.uid_validity);
        if state.last_uid.is_none() || state.uid_validity != Some(validity) {
            return Ok(FetchBatch {
                state: FetchState {
                    uid_validity: Some(validity),
                    last_uid: Some(u64::from(inbox.last_uid)),
                },
                messages: Vec::new(),
            });
        }
        let mut messages: Vec<(u64, Option<InboundMessage>)> = batch
            .messages
            .into_iter()
            .filter_map(|email| {
                let uid = provider_uid(&email)?;
                Some((uid, inbound(email, validity, uid, &config.address)))
            })
            .collect();
        messages.sort_by_key(|(uid, _)| *uid);
        let last_listed = messages.last().map(|(uid, _)| *uid);
        let last_seen = u64::from(inbox.last_uid);
        if state.last_uid.is_some_and(|last| last_seen > last)
            && last_listed.is_none_or(|uid| last_seen > uid)
        {
            messages.push((last_seen, None));
        }
        Ok(FetchBatch { state, messages })
    }

    fn polls(&self) -> bool {
        true
    }
}
