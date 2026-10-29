use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;

pub(crate) const INCOMING_WEBHOOK_RECORD_TYPE: RecordType =
    RecordType::from_static("incoming_webhook");
pub(crate) const SECRET_OWNER_TYPE: &str = "incoming_webhook";
pub(crate) const SIGNING_SECRET_NAME: &str = "signing_secret";
pub(crate) const HOOKS_PATH_PREFIX: &str = "/api/v1/hooks/";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IncomingWebhook {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) token_prefix: String,
    pub(crate) is_active: bool,
    pub(crate) signature_required: bool,
    pub(crate) signature_tolerance_seconds: u32,
    pub(crate) actions: Vec<WebhookAction>,
    pub(crate) created_by: UserId,
    pub(crate) created_by_name: Option<String>,
    pub(crate) last_received_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContactMatch {
    Email,
    Phone,
    Field,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ContactAction {
    #[serde(default)]
    pub(crate) match_by: Option<ContactMatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) match_field: Option<String>,
    #[serde(default)]
    pub(crate) fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) distribution_rule_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DealAction {
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: String,
    #[serde(default)]
    pub(crate) fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) distribution_rule_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ActivityAction {
    pub(crate) activity_type_id: String,
    #[serde(default)]
    pub(crate) fields: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum WebhookAction {
    UpsertContact(ContactAction),
    CreateDeal(DealAction),
    CreateActivity(ActivityAction),
}

impl WebhookAction {
    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::UpsertContact(_) => ActionKind::UpsertContact.as_str(),
            Self::CreateDeal(_) => ActionKind::CreateDeal.as_str(),
            Self::CreateActivity(_) => ActionKind::CreateActivity.as_str(),
        }
    }

    pub(crate) fn kind(&self) -> ActionKind {
        match self {
            Self::UpsertContact(_) => ActionKind::UpsertContact,
            Self::CreateDeal(_) => ActionKind::CreateDeal,
            Self::CreateActivity(_) => ActionKind::CreateActivity,
        }
    }

    pub(crate) fn fields(&self) -> &Map<String, Value> {
        match self {
            Self::UpsertContact(action) => &action.fields,
            Self::CreateDeal(action) => &action.fields,
            Self::CreateActivity(action) => &action.fields,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionKind {
    UpsertContact,
    CreateDeal,
    CreateActivity,
}

impl ActionKind {
    pub(crate) const ALL: [Self; 3] = [Self::UpsertContact, Self::CreateDeal, Self::CreateActivity];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::UpsertContact => "upsert_contact",
            Self::CreateDeal => "create_deal",
            Self::CreateActivity => "create_activity",
        }
    }

    pub(crate) fn record_type(&self) -> RecordType {
        match self {
            Self::UpsertContact => RecordType::CONTACT,
            Self::CreateDeal => RecordType::DEAL,
            Self::CreateActivity => RecordType::ACTIVITY,
        }
    }

    pub(crate) fn system_fields(&self) -> &'static [(&'static str, bool)] {
        match self {
            Self::UpsertContact => &[
                ("first_name", true),
                ("last_name", false),
                ("email", false),
                ("phone", false),
                ("mobile", false),
                ("job_title", false),
                ("company_id", false),
                ("owner_id", false),
                ("source", false),
                ("notes", false),
                ("address_street", false),
                ("address_number", false),
                ("address_complement", false),
                ("address_district", false),
                ("address_city", false),
                ("address_state", false),
                ("address_postal_code", false),
                ("address_country", false),
            ],
            Self::CreateDeal => &[
                ("title", true),
                ("amount", false),
                ("currency", false),
                ("probability", false),
                ("expected_close_date", false),
                ("company_id", false),
                ("contact_id", false),
                ("owner_id", false),
                ("source", false),
            ],
            Self::CreateActivity => &[
                ("title", true),
                ("description", false),
                ("due_at", false),
                ("priority", false),
                ("duration_minutes", false),
                ("owner_id", false),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestStatus {
    Succeeded,
    Partial,
    Failed,
    Rejected,
}

impl RequestStatus {
    pub(crate) const ALL: [Self; 4] =
        [Self::Succeeded, Self::Partial, Self::Failed, Self::Rejected];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionFieldError {
    pub(crate) field: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionError {
    pub(crate) code: String,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) fields: Vec<ActionFieldError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionOutcome {
    pub(crate) index: usize,
    #[serde(rename = "type")]
    pub(crate) action_type: ActionKind,
    pub(crate) status: ActionStatus,
    #[serde(default)]
    pub(crate) operation: Option<String>,
    #[serde(default)]
    pub(crate) record_type: Option<String>,
    #[serde(default)]
    pub(crate) record_id: Option<String>,
    #[serde(default)]
    pub(crate) error: Option<ActionError>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IncomingRequestLog {
    pub(crate) id: Uuid,
    pub(crate) webhook_id: Uuid,
    pub(crate) received_at: DateTime<Utc>,
    pub(crate) ip: Option<String>,
    pub(crate) content_type: Option<String>,
    pub(crate) headers: Map<String, Value>,
    pub(crate) body: Option<String>,
    pub(crate) body_bytes: u64,
    pub(crate) body_truncated: bool,
    pub(crate) status: RequestStatus,
    pub(crate) http_status: u16,
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) results: Vec<ActionOutcome>,
    pub(crate) duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MappableField {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field_type: String,
    pub(crate) is_custom: bool,
    pub(crate) is_required: bool,
    pub(crate) is_unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionTarget {
    pub(crate) action_type: ActionKind,
    pub(crate) record_type: String,
    pub(crate) fields: Vec<MappableField>,
}
