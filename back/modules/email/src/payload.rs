use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{EmailDirection, EmailKind, EmailVisibility};
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;

pub use aphel_sdk::email::EMAIL_RECEIVED;
pub(crate) const EMAIL_SENT: &str = "email.sent";
pub use aphel_sdk::email::EMAIL_BOUNCED;
pub use aphel_sdk::email::EMAIL_CLICKED;
pub use aphel_sdk::email::EMAIL_OPENED;
pub(crate) const MAILBOX_CONNECTED: &str = "mailbox.connected";
pub(crate) const MAILBOX_UPDATED: &str = "mailbox.updated";
pub(crate) const MAILBOX_DISCONNECTED: &str = "mailbox.disconnected";
pub(crate) const EMAIL_THREAD_LINKED: &str = "email_thread.linked";
pub(crate) const EMAIL_THREAD_UNLINKED: &str = "email_thread.unlinked";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmailEventPayload {
    pub(crate) mailbox_id: Uuid,
    pub(crate) thread_id: Uuid,
    pub(crate) email_id: Uuid,
    pub(crate) owner_id: UserId,
    pub(crate) direction: EmailDirection,
    pub(crate) kind: EmailKind,
    pub(crate) visibility: EmailVisibility,
    pub(crate) records: Vec<RecordRef>,
    pub(crate) is_reply: bool,
    pub(crate) extra: Value,
}

impl EmailEventPayload {
    pub(crate) fn to_value(&self) -> Value {
        let mut payload = json!({
            "mailbox_id": self.mailbox_id,
            "thread_id": self.thread_id,
            "email_id": self.email_id,
            "owner_id": self.owner_id.as_str(),
            "direction": self.direction.as_str(),
            "kind": self.kind.as_str(),
            "visibility": self.visibility.as_str(),
            "is_reply": self.is_reply,
            "records": self
                .records
                .iter()
                .map(|record| json!({
                    "record_type": record.record_type.as_str(),
                    "record_id": record.record_id,
                }))
                .collect::<Vec<_>>(),
        });
        if let (Value::Object(target), Value::Object(extra)) = (&mut payload, &self.extra) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
        }
        payload
    }

    pub(crate) fn targets(&self) -> Vec<Option<RecordRef>> {
        if self.records.is_empty() {
            return vec![None];
        }
        self.records.iter().cloned().map(Some).collect()
    }
}
