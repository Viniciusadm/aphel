use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAddress {
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl EmailAddress {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecordInput {
    pub record_type: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SendEmailCommand {
    pub mailbox_id: Option<String>,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub template_id: Option<String>,
    pub record: Option<RecordInput>,
    pub reply_to_message_id: Option<String>,
    pub attachment_file_ids: Vec<String>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub tracking: Option<bool>,
    pub ip: Option<String>,
}

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("email", "1.0.0"),
    entries: &[
        aphel_contract::KeyEntry::new(
            "email.mailbox_oauth",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-managed mailbox OAuth connection, token acquisition and recovery",
        ),
        aphel_contract::KeyEntry::new(
            "email.mailbox_provider",
            aphel_contract::KeyKind::ExtensionPoint,
            "Native mailbox provider operations and credential persistence callbacks",
        ),
        aphel_contract::KeyEntry::new(
            "email.sender",
            aphel_contract::KeyKind::ExtensionPoint,
            "Transactional scoped email enqueue",
        ),
        aphel_contract::KeyEntry::new(
            "email.engagement_projection",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-controlled transactional email engagement maintenance",
        ),
    ],
};

pub const EMAIL_RECEIVED: &str = "email.received";

pub const EMAIL_OPENED: &str = "email.opened";

pub const EMAIL_CLICKED: &str = "email.clicked";

pub const EMAIL_BOUNCED: &str = "email.bounced";

pub const MAIL_API_FAILED: &str = "mail_api_failed";
pub const MAIL_TOKEN_REJECTED: &str = "mail_token_rejected";
pub const MAIL_ACCESS_DENIED: &str = "mail_access_denied";
pub const MAIL_NOT_FOUND: &str = "mail_not_found";
pub const MAIL_CURSOR_EXPIRED: &str = "mail_cursor_expired";
pub const MAIL_NOT_AUTHORIZED: &str = "mail_not_authorized";
pub const MAIL_AUTHORIZATION_REVOKED: &str = "mail_authorization_revoked";
pub const MAIL_NO_REFRESH_TOKEN: &str = "mail_no_refresh_token";
pub const MAIL_INVALID_MESSAGE: &str = "mail_invalid_message";
pub const MAIL_CONNECT_FAILED: &str = "mail_connect_failed";
pub const ALREADY_CONNECTED: &str = "already_connected";

pub const PURPOSE_MAIL: &str = "mail";
