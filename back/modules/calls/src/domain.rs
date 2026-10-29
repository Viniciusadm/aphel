use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::application::files::UserReference;
use aphel_kernel::domain::UserId;

pub(crate) use crate::call::{ActivityCallView, CallDetails, CallDirection};

pub const CALL_LOGGED: &str = "call.logged";
pub const CALL_UPDATED: &str = "call.updated";
pub(crate) const CALL_OUTCOME_CREATED: &str = "call_outcome.created";
pub(crate) const CALL_OUTCOME_UPDATED: &str = "call_outcome.updated";
pub(crate) const CALL_OUTCOME_REORDERED: &str = "call_outcome.reordered";
pub(crate) const CALL_SESSION_STARTED: &str = "call_session.started";
pub(crate) const CALL_SESSION_COMPLETED: &str = "call_session.completed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallActivityType {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

pub(crate) const MAX_CALL_DURATION_SECONDS: i64 = 86_400;
pub(crate) const MAX_RECORDING_URL_LENGTH: usize = 2048;
pub(crate) const MAX_PHONE_NUMBER_LENGTH: usize = 64;
pub(crate) const MAX_PROVIDER_LENGTH: usize = 128;
pub(crate) const MAX_PROVIDER_CALL_ID_LENGTH: usize = 255;
pub(crate) const MAX_CALL_OUTCOMES: usize = 50;
pub(crate) const MAX_OUTCOME_NAME_LENGTH: usize = 140;
pub(crate) const MAX_SESSION_CONTACTS: u64 = 500;
pub(crate) const MAX_SESSION_NAME_LENGTH: usize = 200;
pub(crate) const MAX_SESSION_SCRIPT_LENGTH: usize = 20_000;
pub(crate) const VIEW_SCRIPT_SETTING: &str = "call_script";
pub(crate) const CALL_ACTIVITY_TYPE_KEY: &str = "call";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultCallOutcome {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
    pub(crate) is_connected: bool,
}

pub(crate) const DEFAULT_CALL_OUTCOMES: [DefaultCallOutcome; 4] = [
    DefaultCallOutcome {
        key: "answered",
        name: "Atendeu",
        is_connected: true,
    },
    DefaultCallOutcome {
        key: "voicemail",
        name: "Caixa postal",
        is_connected: false,
    },
    DefaultCallOutcome {
        key: "no_answer",
        name: "Sem resposta",
        is_connected: false,
    },
    DefaultCallOutcome {
        key: "wrong_number",
        name: "Número errado",
        is_connected: false,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallOutcome {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) is_connected: bool,
    pub(crate) is_system: bool,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl CallOutcome {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallSessionStatus {
    Active,
    Completed,
}

impl CallSessionStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionItemStatus {
    Pending,
    Logged,
    Skipped,
}

impl SessionItemStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Logged => "logged",
            Self::Skipped => "skipped",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "logged" => Some(Self::Logged),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallSession {
    pub(crate) id: Uuid,
    pub(crate) user_id: UserId,
    pub(crate) view_id: Option<Uuid>,
    pub(crate) name: String,
    pub(crate) script: Option<String>,
    pub(crate) status: CallSessionStatus,
    pub(crate) position: u32,
    pub(crate) total: u32,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallSessionItem {
    pub(crate) position: u32,
    pub(crate) contact_id: Uuid,
    pub(crate) contact_name: Option<String>,
    pub(crate) status: SessionItemStatus,
    pub(crate) activity_id: Option<Uuid>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) handled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SessionCounts {
    pub(crate) pending: u32,
    pub(crate) logged: u32,
    pub(crate) skipped: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallSessionView {
    pub(crate) session: CallSession,
    pub(crate) counts: SessionCounts,
    pub(crate) current: Option<CallSessionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallReportFilter {
    pub(crate) from: DateTime<Utc>,
    pub(crate) to: DateTime<Utc>,
    pub(crate) owner_id: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallReportRow {
    pub(crate) user: Option<UserReference>,
    pub(crate) outcome_key: String,
    pub(crate) calls: u64,
    pub(crate) duration_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallReportOutcomeCount {
    pub(crate) outcome_key: String,
    pub(crate) calls: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallReportUser {
    pub(crate) user: Option<UserReference>,
    pub(crate) calls: u64,
    pub(crate) connected: u64,
    pub(crate) duration_seconds: u64,
    pub(crate) outcomes: Vec<CallReportOutcomeCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallReport {
    pub(crate) from: DateTime<Utc>,
    pub(crate) to: DateTime<Utc>,
    pub(crate) outcomes: Vec<CallOutcome>,
    pub(crate) users: Vec<CallReportUser>,
    pub(crate) calls: u64,
    pub(crate) connected: u64,
    pub(crate) duration_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedCallView {
    pub(crate) provider: String,
    pub(crate) provider_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallRecordingView {
    pub(crate) url: String,
    pub(crate) expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LogCallCommand {
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) direction: Option<CallDirection>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) occurred_at: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateCallCommand {
    pub(crate) direction: Option<CallDirection>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) duration_seconds: Option<Option<i64>>,
    pub(crate) phone_number: Option<Option<String>>,
    pub(crate) recording_url: Option<Option<String>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CallEventCommand {
    pub(crate) provider: String,
    pub(crate) provider_call_id: String,
    pub(crate) direction: Option<CallDirection>,
    pub(crate) phone_number: Option<String>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) recording_url: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) occurred_at: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct StartCallCommand {
    pub(crate) provider: String,
    pub(crate) phone_number: String,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateCallOutcomeCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) is_connected: bool,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateCallOutcomeCommand {
    pub(crate) name: Option<String>,
    pub(crate) is_connected: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct StartCallSessionCommand {
    pub(crate) view_id: String,
    pub(crate) name: Option<String>,
    pub(crate) script: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SessionCallCommand {
    pub(crate) position: Option<u32>,
    pub(crate) direction: Option<CallDirection>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CallReportQuery {
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) owner_id: Option<String>,
}
