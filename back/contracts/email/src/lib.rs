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
    entries: &[aphel_contract::KeyEntry::new(
        "email.sender",
        aphel_contract::KeyKind::ExtensionPoint,
        "Transactional scoped email enqueue",
    )],
};

pub const EMAIL_RECEIVED: &str = "email.received";

pub const EMAIL_OPENED: &str = "email.opened";

pub const EMAIL_CLICKED: &str = "email.clicked";

pub const EMAIL_BOUNCED: &str = "email.bounced";
