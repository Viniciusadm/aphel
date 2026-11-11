use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApprovalMoment(Cow<'static, str>);

pub const MAX_MOMENT_LENGTH: usize = 64;

impl ApprovalMoment {
    pub const STAGE_CHANGE: Self = Self(Cow::Borrowed("stage_change"));
    pub const QUOTE_SEND: Self = Self(Cow::Borrowed("quote_send"));
    pub const ON_DEMAND: Self = Self(Cow::Borrowed("on_demand"));

    pub const fn from_static(key: &'static str) -> Self {
        Self(Cow::Borrowed(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let valid = !value.is_empty()
            && value.len() <= MAX_MOMENT_LENGTH
            && value.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '_' | '.')
            });
        valid.then(|| Self(Cow::Owned(value.to_owned())))
    }
}

impl std::fmt::Display for ApprovalMoment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MomentTarget {
    None,
    Policy,
    Subject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalMomentDefinition {
    pub moment: ApprovalMoment,
    pub label: String,
    pub target: MomentTarget,
    pub record_types: Option<Vec<String>>,
}

impl ApprovalMomentDefinition {
    pub fn applies_to(&self, record_type: &str) -> bool {
        self.record_types
            .as_ref()
            .is_none_or(|types| types.iter().any(|candidate| candidate == record_type))
    }
}

pub trait ApprovalMomentSource: Send + Sync {
    fn moments(&self) -> Vec<ApprovalMomentDefinition>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl ApprovalStatus {
    pub const ALL: [Self; 4] = [
        Self::Pending,
        Self::Approved,
        Self::Rejected,
        Self::Cancelled,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

pub const APPROVAL_PENDING: &str = "approval_pending";
pub const APPROVAL_REQUIRED: &str = "approval_required";
pub const APPROVAL_APPROVED: &str = "approval.approved";
pub const APPROVAL_REJECTED: &str = "approval.rejected";
pub const APPROVAL_CANCELLED: &str = "approval.cancelled";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicyTemplate {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub record_type: String,
    pub moment: ApprovalMoment,
    pub filter: serde_json::Value,
    pub locked_fields: Vec<String>,
    pub steps: Vec<ApprovalStepTemplate>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalStepTemplate {
    pub name: Option<String>,
    pub approver_kind: String,
    pub approver_user_id: Option<uuid::Uuid>,
    pub approver_role_id: Option<uuid::Uuid>,
    pub rule: String,
}

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("approvals", "1.1.0"),
    entries: &[
        aphel_contract::KeyEntry::new(
            "approvals.gate",
            aphel_contract::KeyKind::ExtensionPoint,
            "Scoped approval checkpoints and submissions",
        ),
        aphel_contract::KeyEntry::new(
            "approvals.moments",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-contributed approval moments and how they target records",
        ),
        aphel_contract::KeyEntry::new(
            "approvals.defaults",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-contributed default approval policies",
        ),
    ],
};
