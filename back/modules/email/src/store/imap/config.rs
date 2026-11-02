use std::net::IpAddr;

use serde_json::{Map, Value, json};

use crate::domain::normalize_address;
use crate::{MailProviderError, MailboxCredentials};
use aphel_kernel::infrastructure::outbound_http::network::is_public_ip;

pub(crate) const PASSWORD_CREDENTIAL: &str = "password";
pub(crate) const SMTP_PASSWORD_CREDENTIAL: &str = "smtp_password";
pub(crate) const ALLOW_PRIVATE_NETWORKS_ENV: &str = "EMAIL_ALLOW_PRIVATE_NETWORKS";
pub(crate) const MAX_MESSAGE_MB_ENV: &str = "EMAIL_SYNC_MAX_MESSAGE_MB";
pub(crate) const DEFAULT_MAX_MESSAGE_MB: u32 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Security {
    Tls,
    StartTls,
    None,
}

impl Security {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Tls => "tls",
            Self::StartTls => "starttls",
            Self::None => "none",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "tls" | "ssl" => Some(Self::Tls),
            "starttls" => Some(Self::StartTls),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImapSettings {
    pub(crate) address: String,
    pub(crate) username: String,
    pub(crate) smtp_username: String,
    pub(crate) imap_host: String,
    pub(crate) imap_port: u16,
    pub(crate) imap_security: Security,
    pub(crate) smtp_host: String,
    pub(crate) smtp_port: u16,
    pub(crate) smtp_security: Security,
    pub(crate) sent_folder: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NetworkPolicy {
    pub(crate) allow_private_networks: bool,
    pub(crate) max_message_bytes: u32,
}

impl NetworkPolicy {
    pub(crate) fn from_env() -> Self {
        let allow_private_networks = std::env::var(ALLOW_PRIVATE_NETWORKS_ENV)
            .is_ok_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes"));
        let max_message_mb = std::env::var(MAX_MESSAGE_MB_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .filter(|value| (1..=100).contains(value))
            .unwrap_or(DEFAULT_MAX_MESSAGE_MB);
        Self {
            allow_private_networks,
            max_message_bytes: max_message_mb * 1024 * 1024,
        }
    }

    pub(crate) fn allows(&self, ip: IpAddr) -> bool {
        self.allow_private_networks || is_public_ip(ip)
    }
}

fn text(settings: &Map<String, Value>, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn port(settings: &Map<String, Value>, key: &str, default: u16) -> Result<u16, String> {
    match settings.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port > 0)
            .ok_or_else(|| format!("{key} must be a port between 1 and 65535")),
        Some(Value::String(value)) => value
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| format!("{key} must be a port between 1 and 65535")),
        _ => Err(format!("{key} must be a port between 1 and 65535")),
    }
}

fn security(settings: &Map<String, Value>, key: &str) -> Result<Security, String> {
    match text(settings, key) {
        None => Ok(Security::Tls),
        Some(value) => {
            Security::parse(&value).ok_or_else(|| format!("{key} must be tls, starttls or none"))
        }
    }
}

fn host(settings: &Map<String, Value>, key: &str) -> Result<String, String> {
    let host = text(settings, key)
        .map(|host| host.to_lowercase())
        .ok_or_else(|| format!("{key} is required"))?;
    let valid = host.len() <= 253
        && !host.starts_with(['.', '-'])
        && host.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | ':')
        });
    if valid {
        Ok(host)
    } else {
        Err(format!("{key} must be a host name"))
    }
}

fn default_port(security: Security, tls: u16, starttls: u16, plain: u16) -> u16 {
    match security {
        Security::Tls => tls,
        Security::StartTls => starttls,
        Security::None => plain,
    }
}

pub(crate) fn parse_settings(settings: &Value) -> Result<ImapSettings, MailProviderError> {
    let empty = Map::new();
    let values = settings.as_object().unwrap_or(&empty);
    let parse = || -> Result<ImapSettings, String> {
        let imap_security = security(values, "imap_security")?;
        let smtp_security = security(values, "smtp_security")?;
        let username = text(values, "username").ok_or("username is required")?;
        let address = text(values, "address")
            .or_else(|| username.contains('@').then(|| username.clone()))
            .ok_or("address is required when the username is not an e-mail address")?;
        let address =
            normalize_address(&address).map_err(|message| format!("address {message}"))?;
        Ok(ImapSettings {
            smtp_username: text(values, "smtp_username").unwrap_or_else(|| username.clone()),
            username,
            address,
            imap_host: host(values, "imap_host")?,
            imap_port: port(
                values,
                "imap_port",
                default_port(imap_security, 993, 143, 143),
            )?,
            imap_security,
            smtp_host: host(values, "smtp_host")?,
            smtp_port: port(
                values,
                "smtp_port",
                default_port(smtp_security, 465, 587, 25),
            )?,
            smtp_security,
            sent_folder: text(values, "sent_folder"),
        })
    };
    parse().map_err(MailProviderError::rejected)
}

impl ImapSettings {
    pub(crate) fn to_value(&self) -> Value {
        let mut value = json!({
            "address": self.address,
            "username": self.username,
            "imap_host": self.imap_host,
            "imap_port": self.imap_port,
            "imap_security": self.imap_security.as_str(),
            "smtp_host": self.smtp_host,
            "smtp_port": self.smtp_port,
            "smtp_security": self.smtp_security.as_str(),
        });
        if self.smtp_username != self.username {
            value["smtp_username"] = json!(self.smtp_username);
        }
        if let Some(folder) = &self.sent_folder {
            value["sent_folder"] = json!(folder);
        }
        value
    }
}

pub(crate) struct Passwords {
    pub(crate) imap: String,
    pub(crate) smtp: String,
}

pub(crate) fn passwords(credentials: &MailboxCredentials) -> Result<Passwords, MailProviderError> {
    let imap = credentials
        .get(PASSWORD_CREDENTIAL)
        .filter(|password| !password.is_empty())
        .ok_or_else(|| MailProviderError::authentication("the mailbox password is missing"))?
        .to_owned();
    let smtp = credentials
        .get(SMTP_PASSWORD_CREDENTIAL)
        .filter(|password| !password.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| imap.clone());
    Ok(Passwords { imap, smtp })
}

pub(crate) fn stored_credentials(credentials: &MailboxCredentials) -> MailboxCredentials {
    let mut values = Map::new();
    for key in [PASSWORD_CREDENTIAL, SMTP_PASSWORD_CREDENTIAL] {
        if let Some(value) = credentials.get(key).filter(|value| !value.is_empty()) {
            values.insert(key.to_owned(), Value::String(value.to_owned()));
        }
    }
    MailboxCredentials(values)
}
