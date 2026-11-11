use aphel_contract::{Contract, ContractDescriptor, KeyEntry, KeyKind, contract};
use serde::{Deserialize, Serialize};

pub const CONTRACT: Contract = contract!("publication", "1.0.0");

pub const MODULE: &str = "publication";
pub const PUBLISHABLE: &str = "publication.publishable";
pub const ACTIVATE_MOMENT: &str = "publication.activate";

pub const STATE_DRAFT: &str = "draft";
pub const STATE_PUBLISHED: &str = "published";
pub const STATE_WITHDRAWN: &str = "withdrawn";
pub const STATES: [&str; 3] = [STATE_DRAFT, STATE_PUBLISHED, STATE_WITHDRAWN];

pub const OP_CAPTURE: &str = "publication.capture";
pub const OP_REQUEST: &str = "publication.request";
pub const OP_WITHDRAW: &str = "publication.withdraw";
pub const OP_CANCEL: &str = "publication.cancel";
pub const OP_RESTORE: &str = "publication.restore";
pub const OP_RETRY: &str = "publication.retry";
pub const OP_STATUS: &str = "publication.status";
pub const OP_REVISIONS: &str = "publication.revisions";
pub const OP_REVISION: &str = "publication.revision";

pub const REVISION_CAPTURED: &str = "publication.revision_captured";
pub const REQUESTED: &str = "publication.requested";
pub const ACTIVATED: &str = "publication.activated";
pub const WITHDRAWN: &str = "publication.withdrawn";
pub const REQUEST_SUPERSEDED: &str = "publication.request_superseded";
pub const REQUEST_CANCELLED: &str = "publication.request_cancelled";
pub const REQUEST_BLOCKED: &str = "publication.request_blocked";
pub const REQUEST_FAILED: &str = "publication.request_failed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishableField {
    pub draft: String,
    pub published: String,
    #[serde(default)]
    pub file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishableDefinition {
    pub entity: String,
    pub format: u32,
    pub permission: String,
    pub fields: Vec<PublishableField>,
    pub state: String,
    pub published_at: String,
    pub revised_at: String,
}

impl PublishableDefinition {
    pub fn projected_fields(&self) -> impl Iterator<Item = &str> {
        self.fields
            .iter()
            .map(|field| field.published.as_str())
            .chain([
                self.state.as_str(),
                self.published_at.as_str(),
                self.revised_at.as_str(),
            ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestAction {
    Activate,
    Withdraw,
}

impl RequestAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Withdraw => "withdraw",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [Self::Activate, Self::Withdraw]
            .into_iter()
            .find(|action| action.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    AwaitingApproval,
    Scheduled,
    Ready,
    Completed,
    Superseded,
    Cancelled,
    Blocked,
    Failed,
}

impl RequestStatus {
    pub const ALL: [Self; 8] = [
        Self::AwaitingApproval,
        Self::Scheduled,
        Self::Ready,
        Self::Completed,
        Self::Superseded,
        Self::Cancelled,
        Self::Blocked,
        Self::Failed,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AwaitingApproval => "awaiting_approval",
            Self::Scheduled => "scheduled",
            Self::Ready => "ready",
            Self::Completed => "completed",
            Self::Superseded => "superseded",
            Self::Cancelled => "cancelled",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub fn is_open(&self) -> bool {
        matches!(self, Self::AwaitingApproval | Self::Scheduled | Self::Ready)
    }
}

pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            PUBLISHABLE,
            KeyKind::ExtensionPoint,
            "Entities whose draft fields are published as immutable revisions into protected fields",
        ),
        KeyEntry::new(
            OP_CAPTURE,
            KeyKind::Operation,
            "Capture the saved draft of a record as an immutable revision",
        ),
        KeyEntry::new(
            OP_REQUEST,
            KeyKind::Operation,
            "Publish a revision now or at a time, subject to approval policies",
        ),
        KeyEntry::new(
            OP_WITHDRAW,
            KeyKind::Operation,
            "Withdraw the published revision of a record",
        ),
        KeyEntry::new(
            OP_CANCEL,
            KeyKind::Operation,
            "Cancel an open publication request",
        ),
        KeyEntry::new(
            OP_RESTORE,
            KeyKind::Operation,
            "Copy a revision back into the draft fields",
        ),
        KeyEntry::new(
            OP_RETRY,
            KeyKind::Operation,
            "Retry a failed publication request",
        ),
        KeyEntry::new(
            OP_STATUS,
            KeyKind::Operation,
            "Publication state of a record",
        ),
        KeyEntry::new(
            OP_REVISIONS,
            KeyKind::Operation,
            "Revisions of a record without payloads",
        ),
        KeyEntry::new(
            OP_REVISION,
            KeyKind::Operation,
            "One revision with its payload",
        ),
        KeyEntry::new(
            ACTIVATED,
            KeyKind::Event,
            "A revision became the published one",
        ),
        KeyEntry::new(
            WITHDRAWN,
            KeyKind::Event,
            "The published revision of a record was withdrawn",
        ),
    ],
};
