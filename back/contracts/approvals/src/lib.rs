use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMoment {
    StageChange,
    QuoteSend,
    OnDemand,
}

impl ApprovalMoment {
    pub const ALL: [Self; 3] = [Self::StageChange, Self::QuoteSend, Self::OnDemand];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StageChange => "stage_change",
            Self::QuoteSend => "quote_send",
            Self::OnDemand => "on_demand",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|moment| moment.as_str() == value)
    }

    pub fn requires_target(&self) -> bool {
        matches!(self, Self::StageChange)
    }
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
    contract: aphel_contract::contract!("approvals", "1.0.0"),
    entries: &[
        aphel_contract::KeyEntry::new(
            "approvals.gate",
            aphel_contract::KeyKind::ExtensionPoint,
            "Scoped approval checkpoints and submissions",
        ),
        aphel_contract::KeyEntry::new(
            "approvals.defaults",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-contributed default approval policies",
        ),
    ],
};
