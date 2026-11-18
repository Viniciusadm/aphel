use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CallDirection {
    Inbound,
    #[default]
    Outbound,
}

impl CallDirection {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "inbound" => Some(Self::Inbound),
            "outbound" => Some(Self::Outbound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallOutcomeSummary {
    pub(crate) key: String,
    pub(crate) name: Option<String>,
    pub(crate) is_connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CallDetails {
    pub(crate) direction: CallDirection,
    pub(crate) outcome_key: String,
    pub(crate) duration_seconds: Option<u32>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) session_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityCallView {
    pub(crate) direction: CallDirection,
    pub(crate) outcome: CallOutcomeSummary,
    pub(crate) duration_seconds: Option<u32>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) session_id: Option<Uuid>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ActivityCallView {
    pub(crate) fn details(&self) -> CallDetails {
        CallDetails {
            direction: self.direction,
            outcome_key: self.outcome.key.clone(),
            duration_seconds: self.duration_seconds,
            phone_number: self.phone_number.clone(),
            recording_url: self.recording_url.clone(),
            provider: self.provider.clone(),
            provider_call_id: self.provider_call_id.clone(),
            session_id: self.session_id,
        }
    }
}
