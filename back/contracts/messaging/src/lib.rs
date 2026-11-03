use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, contract, extension_point,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};
use utoipa::ToSchema;
pub const CONTRACT: Contract = contract!("messaging", "1.0.0");
pub const CHANNEL: ExtensionPoint<ChannelDefinition> = extension_point!("messaging.channel");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[KeyEntry::new(
        "messaging.channel",
        KeyKind::ExtensionPoint,
        "Transport, validation, threading and delivery policy supplied by channel owners",
    )],
};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(transparent)]
pub struct ChannelKind(String);
impl ChannelKind {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn parse(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= 100
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character)))
        .then(|| Self(value.to_owned()))
    }
}
impl<'de> Deserialize<'de> for ChannelKind {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).ok_or_else(|| serde::de::Error::custom("invalid channel kind"))
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ReplyWindow {
    pub seconds: i64,
    pub code: &'static str,
    pub message: &'static str,
}
#[derive(Clone, Default)]
pub struct ChannelCredentials(BTreeMap<String, String>);

impl fmt::Debug for ChannelCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.0.keys()).finish()
    }
}

impl ChannelCredentials {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .get(name)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    pub fn require(&self, name: &str) -> Result<&str, ChannelError> {
        self.get(name).ok_or_else(|| {
            ChannelError::permanent(
                "messaging_channel_credentials_missing",
                format!("the channel credential {name} is not configured"),
            )
        })
    }

    pub fn insert(&mut self, name: &str, value: String) {
        self.0.insert(name.to_owned(), value);
    }
}

#[derive(Debug, Clone)]
pub struct ChannelError {
    pub code: &'static str,
    pub message: String,
    pub permanent: bool,
}

impl ChannelError {
    pub fn permanent(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            permanent: true,
        }
    }

    pub fn transient(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            permanent: false,
        }
    }
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

pub struct ChannelContext<'a> {
    pub config: &'a Value,
    pub credentials: &'a ChannelCredentials,
}

#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub to: String,
    pub to_name: Option<String>,
    pub body: String,
    pub subject: Option<String>,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SentMessage {
    pub external_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboundMessage {
    pub external_id: String,
    pub from: String,
    pub from_name: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    pub message_type: String,
    pub in_reply_to: Option<String>,
    #[serde(default)]
    pub references: Vec<String>,
    pub sent_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FetchState {
    pub uid_validity: Option<u64>,
    pub last_uid: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct FetchBatch {
    pub state: FetchState,
    pub messages: Vec<(u64, Option<InboundMessage>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelOption {
    pub value: String,
    pub label: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelField {
    pub key: String,
    pub input_type: String,
    pub required: bool,
    pub label: BTreeMap<String, String>,
    pub hint: BTreeMap<String, String>,
    pub options: Vec<ChannelOption>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelDefinition {
    pub key: String,
    pub label: BTreeMap<String, String>,
    pub fields: Vec<ChannelField>,
    pub secrets: Vec<ChannelField>,
    pub defaults: BTreeMap<String, String>,
}
