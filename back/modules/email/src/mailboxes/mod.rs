use serde::Serialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::{EmailVisibility, validate_history_days};
use crate::{MAILBOX_CREDENTIALS_SECRET, MAILBOX_SECRET_OWNER, MailboxConnectionKind, MailboxView};
use aphel_kernel::application::secrets::SecretKey;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const MAILBOX_NOT_FOUND: &str = "mailbox_not_found";
pub(crate) const MAILBOX_ALREADY_CONNECTED: &str = "mailbox_already_connected";
pub(crate) const MAILBOX_ADDRESS_MISMATCH: &str = "mailbox_address_mismatch";
pub(crate) const MAILBOX_DISCONNECTED_CODE: &str = "mailbox_disconnected";
pub(crate) const MAILBOX_RECORD_TYPE: RecordType = RecordType::from_static("email.mailbox");
pub(crate) const MAX_DISPLAY_NAME_LENGTH: usize = 200;
pub(crate) const MAX_STATUS_REASON_LENGTH: usize = 500;

const SECRET_SETTING_MARKERS: [&str; 4] = ["password", "secret", "token", "credential"];

pub(crate) fn mailbox_not_found() -> AppError {
    AppError::not_found_code(MAILBOX_NOT_FOUND, "mailbox was not found")
}

pub(crate) fn mailbox_secret_key(mailbox_id: Uuid) -> SecretKey {
    SecretKey::new(
        MAILBOX_SECRET_OWNER,
        mailbox_id.to_string(),
        MAILBOX_CREDENTIALS_SECRET,
    )
}

pub(crate) fn public_settings(settings: &Value) -> Value {
    match settings {
        Value::Object(values) => Value::Object(
            values
                .iter()
                .filter(|(key, _)| {
                    let key = key.to_lowercase();
                    !SECRET_SETTING_MARKERS
                        .iter()
                        .any(|marker| key.contains(marker))
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
        _ => Value::Object(Map::new()),
    }
}

pub(crate) fn status_reason(message: &str) -> String {
    message.chars().take(MAX_STATUS_REASON_LENGTH).collect()
}

pub(crate) fn scope_user_uuid(scope: &AccessScope) -> Result<Uuid, AppError> {
    Uuid::parse_str(scope.user_id().as_str())
        .map_err(|_| AppError::Internal("user id is not a UUID".to_owned()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MailProviderInfo {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) connection: MailboxConnectionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MailboxDetails {
    pub(crate) mailbox: MailboxView,
    pub(crate) provider_label: String,
    pub(crate) credentials_configured: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConnectMailboxCommand {
    pub provider: String,
    pub display_name: Option<String>,
    pub history_days: Option<u32>,
    pub default_visibility: Option<String>,
    pub settings: Option<Value>,
    pub credentials: Map<String, Value>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UpdateMailboxCommand {
    pub(crate) display_name: Option<Option<String>>,
    pub(crate) default_visibility: Option<String>,
    pub(crate) history_days: Option<u32>,
    pub(crate) settings: Option<Value>,
    pub(crate) credentials: Option<Map<String, Value>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MailboxSnapshot {
    pub(crate) provider: String,
    pub(crate) address: String,
    pub(crate) display_name: Option<String>,
    pub(crate) settings: Value,
    pub(crate) history_days: u32,
    pub(crate) default_visibility: &'static str,
}

impl From<&MailboxView> for MailboxSnapshot {
    fn from(mailbox: &MailboxView) -> Self {
        Self {
            provider: mailbox.provider.clone(),
            address: mailbox.address.clone(),
            display_name: mailbox.display_name.clone(),
            settings: public_settings(&mailbox.settings),
            history_days: mailbox.history_days,
            default_visibility: mailbox.default_visibility.as_str(),
        }
    }
}

pub(crate) fn parse_display_name(
    value: Option<String>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let value = value
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())?;
    if value.chars().count() > MAX_DISPLAY_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "display_name",
            "must have at most 200 characters",
        ));
        return None;
    }
    Some(value)
}

pub(crate) fn parse_visibility(
    value: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<EmailVisibility> {
    let visibility = EmailVisibility::parse(value.trim());
    if visibility.is_none() {
        violations.push(FieldViolation::new(
            "default_visibility",
            "must be private, metadata or shared",
        ));
    }
    visibility
}

pub(crate) fn parse_history_days(days: u32, violations: &mut Vec<FieldViolation>) -> Option<u32> {
    validate_history_days(days)
        .map_err(|message| violations.push(FieldViolation::new("history_days", message)))
        .ok()
}

pub(crate) fn settings_object(value: Option<Value>, violations: &mut Vec<FieldViolation>) -> Value {
    match value {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(Value::Object(values)) => Value::Object(values),
        Some(_) => {
            violations.push(FieldViolation::new("settings", "must be an object"));
            Value::Object(Map::new())
        }
    }
}
