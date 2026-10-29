mod email;
mod whatsapp;

use std::collections::BTreeMap;
use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::KEY;
use crate::store::StoredChannel;
use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::Secret;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::module_secret_key;

pub(crate) use whatsapp::{APP_SECRET, VERIFY_TOKEN, parse_webhook};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChannelKind {
    Whatsapp,
    Email,
}

impl ChannelKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Whatsapp => "whatsapp",
            Self::Email => "email",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "whatsapp" => Some(Self::Whatsapp),
            "email" => Some(Self::Email),
            _ => None,
        }
    }

    pub(crate) fn handler(&self) -> &'static dyn MessagingChannel {
        match self {
            Self::Whatsapp => &whatsapp::WhatsappChannel,
            Self::Email => &email::EmailChannel,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ChannelCredentials(BTreeMap<String, String>);

impl fmt::Debug for ChannelCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.0.keys()).finish()
    }
}

impl ChannelCredentials {
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.0
            .get(name)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn require(&self, name: &str) -> Result<&str, ChannelError> {
        self.get(name).ok_or_else(|| {
            ChannelError::permanent(
                "messaging_channel_credentials_missing",
                format!("the channel credential {name} is not configured"),
            )
        })
    }

    pub(crate) fn insert(&mut self, name: &str, value: String) {
        self.0.insert(name.to_owned(), value);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChannelError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) permanent: bool,
}

impl ChannelError {
    pub(crate) fn permanent(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            permanent: true,
        }
    }

    pub(crate) fn transient(code: &'static str, message: impl Into<String>) -> Self {
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

pub(crate) struct ChannelContext<'a> {
    pub(crate) config: &'a Value,
    pub(crate) credentials: &'a ChannelCredentials,
}

#[derive(Debug, Clone)]
pub(crate) struct OutgoingMessage {
    pub(crate) to: String,
    pub(crate) to_name: Option<String>,
    pub(crate) body: String,
    pub(crate) subject: Option<String>,
    pub(crate) message_id: String,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) references: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SentMessage {
    pub(crate) external_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InboundMessage {
    pub(crate) external_id: String,
    pub(crate) from: String,
    pub(crate) from_name: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) body: String,
    pub(crate) message_type: String,
    pub(crate) in_reply_to: Option<String>,
    #[serde(default)]
    pub(crate) references: Vec<String>,
    pub(crate) sent_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FetchState {
    pub(crate) uid_validity: Option<u64>,
    pub(crate) last_uid: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FetchBatch {
    pub(crate) state: FetchState,
    pub(crate) messages: Vec<(u64, Option<InboundMessage>)>,
}

pub(crate) struct ValidatedConfig {
    pub(crate) config: Value,
    pub(crate) violations: Vec<FieldViolation>,
}

#[async_trait]
pub(crate) trait MessagingChannel: Send + Sync {
    fn secret_names(&self) -> &'static [&'static str];
    fn required_secrets(&self) -> &'static [&'static str];
    fn validate_config(&self, config: &Value) -> ValidatedConfig;
    fn max_body_chars(&self) -> usize;
    fn address(&self, config: &Value) -> String;
    async fn test(&self, context: &ChannelContext<'_>) -> Result<String, ChannelError>;
    async fn send(
        &self,
        context: &ChannelContext<'_>,
        message: &OutgoingMessage,
    ) -> Result<SentMessage, ChannelError>;
    async fn fetch(
        &self,
        _context: &ChannelContext<'_>,
        state: FetchState,
        _limit: usize,
    ) -> Result<FetchBatch, ChannelError> {
        Ok(FetchBatch {
            state,
            messages: Vec::new(),
        })
    }
    fn polls(&self) -> bool {
        false
    }
}

pub(crate) fn secret_name(channel_id: uuid::Uuid, name: &str) -> String {
    format!("channel:{channel_id}:{name}")
}

pub(crate) async fn load_credentials(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
) -> Result<ChannelCredentials, AppError> {
    let secrets = ctx.secrets()?;
    let system = AccessScope::system();
    let mut credentials = ChannelCredentials::default();
    for name in channel.kind.handler().secret_names() {
        if let Some(value) = secrets
            .reveal(
                &system,
                &module_secret_key(KEY, &secret_name(channel.id, name)),
            )
            .await?
        {
            credentials.insert(name, value.expose().to_owned());
        }
    }
    Ok(credentials)
}

pub(crate) async fn configured_secrets(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
) -> Result<BTreeMap<String, bool>, AppError> {
    let credentials = load_credentials(ctx, channel).await?;
    Ok(channel
        .kind
        .handler()
        .secret_names()
        .iter()
        .map(|name| ((*name).to_owned(), credentials.get(name).is_some()))
        .collect())
}

pub(crate) async fn store_credential(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    channel_id: uuid::Uuid,
    name: &str,
    value: &str,
) -> Result<(), AppError> {
    ctx.secrets()?
        .store(
            tx,
            ctx.scope,
            &module_secret_key(KEY, &secret_name(channel_id, name)),
            &Secret::new(value.to_owned()),
        )
        .await
}

pub(crate) async fn remove_credential(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    channel_id: uuid::Uuid,
    name: &str,
) -> Result<(), AppError> {
    ctx.secrets()?
        .remove(
            tx,
            ctx.scope,
            &module_secret_key(KEY, &secret_name(channel_id, name)),
        )
        .await
}

pub(crate) async fn remove_credentials(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    channel: &StoredChannel,
) -> Result<(), AppError> {
    for name in channel.kind.handler().secret_names() {
        remove_credential(ctx, tx, channel.id, name).await?;
    }
    Ok(())
}

pub(crate) fn http_error(error: &reqwest::Error) -> ChannelError {
    if error.is_timeout() {
        ChannelError::transient(
            "messaging_provider_timeout",
            "the provider did not answer in time",
        )
    } else if error.is_connect() {
        ChannelError::transient(
            "messaging_provider_unreachable",
            "could not connect to the provider",
        )
    } else {
        ChannelError::transient(
            "messaging_provider_error",
            "communication with the provider failed",
        )
    }
}
