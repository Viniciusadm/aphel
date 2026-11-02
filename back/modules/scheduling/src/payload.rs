use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::domain::UserId;

pub const MEETING_BOOKED: &str = "meeting.booked";
pub(crate) const MEETING_RESCHEDULED: &str = "meeting.rescheduled";
pub(crate) const MEETING_CANCELED: &str = "meeting.canceled";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MeetingEventPayload {
    pub(crate) booking_id: Uuid,
    pub(crate) meeting_type_id: Uuid,
    pub(crate) activity_id: Uuid,
    pub(crate) contact_id: Uuid,
    pub(crate) host_user_id: UserId,
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) ends_at: DateTime<Utc>,
}

impl MeetingEventPayload {
    pub(crate) fn to_value(&self) -> Value {
        json!({
            "booking_id": self.booking_id,
            "meeting_type_id": self.meeting_type_id,
            "activity_id": self.activity_id,
            "contact_id": self.contact_id,
            "host_user_id": self.host_user_id.as_str(),
            "starts_at": self.starts_at.to_rfc3339_opts(SecondsFormat::Secs, true),
            "ends_at": self.ends_at.to_rfc3339_opts(SecondsFormat::Secs, true),
            "records": [{ "record_type": "contacts.contact", "record_id": self.contact_id }],
        })
    }
}
