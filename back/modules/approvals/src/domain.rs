use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::application::query::Filter;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;

pub(crate) const APPROVAL_REQUESTED: &str = "approval.requested";
pub const APPROVAL_APPROVED: &str = "approval.approved";
pub const APPROVAL_REJECTED: &str = "approval.rejected";
pub const APPROVAL_CANCELLED: &str = "approval.cancelled";

pub const APPROVAL_PENDING: &str = "approval_pending";
pub(crate) const APPROVAL_REQUIRED: &str = "approval_required";
pub(crate) const APPROVAL_NO_APPROVER: &str = "approval_no_approver";

pub(crate) const APPROVAL_STEP_ADVANCED: &str = "approval.step_advanced";
pub(crate) const APPROVAL_DECISION_RECORDED: &str = "approval.decision_recorded";
pub(crate) const APPROVAL_OVERDUE: &str = "approval.overdue";
pub(crate) const APPROVAL_POLICY_CREATED: &str = "approval_policy.created";
pub(crate) const APPROVAL_POLICY_UPDATED: &str = "approval_policy.updated";
pub(crate) const APPROVAL_POLICY_DELETED: &str = "approval_policy.deleted";
pub(crate) const APPROVAL_POLICIES_REORDERED: &str = "approval_policy.reordered";

pub(crate) const APPROVAL_POLICY_NOT_FOUND: &str = "approval_policy_not_found";
pub(crate) const APPROVAL_POLICY_KEY_TAKEN: &str = "approval_policy_key_taken";
pub(crate) const APPROVAL_REQUEST_NOT_FOUND: &str = "approval_request_not_found";
pub(crate) const APPROVAL_NOT_APPROVER: &str = "approval_not_approver";
pub(crate) const APPROVAL_NOT_PENDING: &str = "approval_not_pending";
pub(crate) const APPROVAL_RECORD_NOT_FOUND: &str = "record_not_found";

pub(crate) const APPROVAL_POLICY_RECORD: &str = "approval_policy";

pub(crate) const APPROVALS_OVERDUE_JOB: &str = "approvals.overdue";
pub(crate) const APPROVALS_OVERDUE_CRON: &str = "0 */10 * * * *";
pub(crate) const APPROVALS_CLEANUP_CONSUMER: &str = "approvals.cleanup";

pub(crate) const MAX_POLICY_DESCRIPTION_LENGTH: usize = 2_000;
pub(crate) const MAX_STEP_NAME_LENGTH: usize = 140;
pub(crate) const MAX_LOCKED_FIELDS: usize = 100;
pub(crate) const MAX_CONDITION_FORMULA_LENGTH: usize = 2_000;
pub(crate) const DELETED_RECORD_REASON: &str = "record_deleted";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PolicyCondition {
    Always,
    Filter(Filter),
    Formula(String),
}

impl PolicyCondition {
    pub(crate) fn kind(&self) -> ConditionKind {
        match self {
            Self::Always => ConditionKind::Always,
            Self::Filter(_) => ConditionKind::Filter,
            Self::Formula(_) => ConditionKind::Formula,
        }
    }

    pub(crate) fn filter(&self) -> Option<&Filter> {
        match self {
            Self::Filter(filter) => Some(filter),
            _ => None,
        }
    }

    pub(crate) fn formula(&self) -> Option<&str> {
        match self {
            Self::Formula(formula) => Some(formula),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyStep {
    pub(crate) id: Uuid,
    pub(crate) position: u32,
    pub(crate) name: Option<String>,
    pub(crate) approver_kind: ApproverKind,
    pub(crate) approver_user_id: Option<UserId>,
    pub(crate) approver_role_id: Option<Uuid>,
    pub(crate) rule: ApprovalRule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalPolicy {
    pub id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) moment: ApprovalMoment,
    pub(crate) target_id: Option<Uuid>,
    pub(crate) condition: PolicyCondition,
    pub(crate) locked_fields: Vec<String>,
    pub(crate) due_hours: Option<u32>,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) steps: Vec<PolicyStep>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ApprovalPolicy {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "key": self.key,
            "name": self.name,
            "description": self.description,
            "record_type": self.record_type,
            "moment": self.moment.as_str(),
            "target_id": self.target_id.map(|id| id.to_string()),
            "condition_kind": self.condition.kind().as_str(),
            "condition_filter": self.condition.filter().map(|filter| serde_json::to_value(filter).unwrap_or(Value::Null)),
            "condition_formula": self.condition.formula(),
            "locked_fields": self.locked_fields,
            "due_hours": self.due_hours,
            "is_active": self.is_active,
            "steps": self.steps.iter().map(StepSnapshot::from_step).collect::<Vec<_>>(),
        })
    }

    pub(crate) fn step_snapshots(&self) -> Vec<StepSnapshot> {
        self.steps.iter().map(StepSnapshot::from_step).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StepSnapshot {
    pub(crate) position: u32,
    #[serde(default)]
    pub(crate) name: Option<String>,
    pub(crate) approver_kind: String,
    #[serde(default)]
    pub(crate) approver_user_id: Option<String>,
    #[serde(default)]
    pub(crate) approver_role_id: Option<String>,
    pub(crate) rule: String,
}

impl StepSnapshot {
    pub(crate) fn from_step(step: &PolicyStep) -> Self {
        Self {
            position: step.position,
            name: step.name.clone(),
            approver_kind: step.approver_kind.as_str().to_owned(),
            approver_user_id: step
                .approver_user_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            approver_role_id: step.approver_role_id.map(|id| id.to_string()),
            rule: step.rule.as_str().to_owned(),
        }
    }

    pub(crate) fn kind(&self) -> Option<ApproverKind> {
        ApproverKind::parse(&self.approver_kind)
    }

    pub(crate) fn rule(&self) -> ApprovalRule {
        ApprovalRule::parse(&self.rule).unwrap_or(ApprovalRule::Any)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApprovalRequest {
    pub(crate) id: Uuid,
    pub(crate) policy_id: Option<Uuid>,
    pub(crate) policy_key: Option<String>,
    pub(crate) policy_name: String,
    pub(crate) record: RecordRef,
    pub(crate) moment: ApprovalMoment,
    pub(crate) target_id: Option<Uuid>,
    pub(crate) status: ApprovalStatus,
    pub(crate) current_step: u32,
    pub(crate) steps: Vec<StepSnapshot>,
    pub(crate) locked_fields: Vec<String>,
    pub(crate) comment: Option<String>,
    pub(crate) requested_by: Option<UserId>,
    pub(crate) requested_at: DateTime<Utc>,
    pub(crate) due_at: Option<DateTime<Utc>>,
    pub(crate) resolved_at: Option<DateTime<Utc>>,
    pub(crate) resolved_by: Option<UserId>,
    pub(crate) resolution_comment: Option<String>,
    pub(crate) consumed_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ApprovalRequest {
    pub(crate) fn summary(&self) -> ApprovalRequestSummary {
        ApprovalRequestSummary {
            id: self.id,
            policy_id: self.policy_id.unwrap_or_default(),
            policy_name: self.policy_name.clone(),
            record: self.record.clone(),
            moment: self.moment,
            target_id: self.target_id,
            status: self.status,
            current_step: self.current_step,
            locked_fields: self.locked_fields.clone(),
        }
    }

    pub(crate) fn is_unconsumed_approval(&self) -> bool {
        self.status == ApprovalStatus::Approved && self.consumed_at.is_none()
    }

    pub(crate) fn current(&self) -> Option<&StepSnapshot> {
        self.steps.get(self.current_step as usize)
    }

    pub(crate) fn event_payload(&self, step: Option<u32>) -> Value {
        json!({
            "request_id": self.id.to_string(),
            "policy_id": self.policy_id.map(|id| id.to_string()),
            "policy_key": self.policy_key,
            "policy_name": self.policy_name,
            "moment": self.moment.as_str(),
            "target_id": self.target_id.map(|id| id.to_string()),
            "record_type": self.record.record_type.as_str(),
            "record_id": self.record.record_id.to_string(),
            "requested_by": self.requested_by.as_ref().map(UserId::as_str),
            "step": step,
            "comment": self.comment,
            "resolution_comment": self.resolution_comment,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestApprover {
    pub(crate) step_position: u32,
    pub(crate) user_id: UserId,
    pub(crate) decision: Option<ApprovalDecision>,
    pub(crate) comment: Option<String>,
    pub(crate) decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApprovalRequestRecord {
    pub(crate) request: ApprovalRequest,
    pub(crate) approvers: Vec<RequestApprover>,
}

impl ApprovalRequestRecord {
    pub(crate) fn step_approvers(&self, position: u32) -> impl Iterator<Item = &RequestApprover> {
        self.approvers
            .iter()
            .filter(move |approver| approver.step_position == position)
    }

    pub(crate) fn awaits(&self, user_id: &UserId) -> bool {
        self.request.status == ApprovalStatus::Pending
            && self
                .step_approvers(self.request.current_step)
                .any(|approver| &approver.user_id == user_id && approver.decision.is_none())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RequestBox {
    #[default]
    PendingForMe,
    RequestedByMe,
    All,
}

impl RequestBox {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending_for_me" => Some(Self::PendingForMe),
            "requested_by_me" => Some(Self::RequestedByMe),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RequestListFilter {
    pub(crate) record: Option<RecordRef>,
    pub(crate) awaiting: Option<UserId>,
    pub(crate) requested_by: Option<UserId>,
    pub(crate) status: Option<ApprovalStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RequestListQuery {
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) box_name: Option<String>,
    pub(crate) status: Option<ApprovalStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PolicyListFilter {
    pub(crate) record_type: Option<String>,
    pub(crate) active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepInput {
    pub name: Option<String>,
    pub approver_kind: String,
    pub approver_user_id: Option<String>,
    pub approver_role_id: Option<String>,
    pub rule: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolicyCommand {
    pub key: Option<String>,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub record_type: Option<String>,
    pub moment: Option<String>,
    pub target_id: Option<Option<String>>,
    pub condition_kind: Option<String>,
    pub condition_filter: Option<Value>,
    pub condition_formula: Option<Option<String>>,
    pub locked_fields: Option<Vec<String>>,
    pub due_hours: Option<Option<i64>>,
    pub is_active: Option<bool>,
    pub steps: Option<Vec<StepInput>>,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SubmitApprovalCommand {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) moment: String,
    pub(crate) target_id: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DecideCommand {
    pub(crate) comment: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RequestView {
    pub(crate) record: ApprovalRequestRecord,
    pub(crate) record_title: Option<String>,
    pub(crate) target_name: Option<String>,
    pub(crate) names: std::collections::HashMap<String, String>,
    pub(crate) can_decide: bool,
    pub(crate) can_cancel: bool,
    pub(crate) is_overdue: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SubmissionView {
    pub(crate) outcome: &'static str,
    pub(crate) requests: Vec<RequestView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordState {
    pub(crate) owner_id: Option<UserId>,
}

pub(crate) const MAX_POLICY_STEPS: usize = 10;
pub(crate) const MAX_POLICY_NAME_LENGTH: usize = 140;
pub(crate) const MAX_DECISION_COMMENT_LENGTH: usize = 2_000;
pub(crate) const MAX_DUE_HOURS: u32 = 24 * 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalMoment {
    StageChange,
    QuoteSend,
    OnDemand,
}

impl ApprovalMoment {
    pub(crate) const ALL: [Self; 3] = [Self::StageChange, Self::QuoteSend, Self::OnDemand];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StageChange => "stage_change",
            Self::QuoteSend => "quote_send",
            Self::OnDemand => "on_demand",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|moment| moment.as_str() == value)
    }

    pub(crate) fn requires_target(&self) -> bool {
        matches!(self, Self::StageChange)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl ApprovalStatus {
    pub(crate) const ALL: [Self; 4] = [
        Self::Pending,
        Self::Approved,
        Self::Rejected,
        Self::Cancelled,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ApproverKind {
    User,
    Role,
    OwnerTeamManager,
}

impl ApproverKind {
    pub(crate) const ALL: [Self; 3] = [Self::User, Self::Role, Self::OwnerTeamManager];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Role => "role",
            Self::OwnerTeamManager => "owner_team_manager",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ApprovalRule {
    Any,
    All,
}

impl ApprovalRule {
    pub(crate) const ALL: [Self; 2] = [Self::Any, Self::All];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::All => "all",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.as_str() == value)
    }

    pub(crate) fn is_satisfied(&self, approvals: usize, approvers: usize) -> bool {
        match self {
            Self::Any => approvals >= 1,
            Self::All => approvers > 0 && approvals >= approvers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ApprovalDecision {
    Approved,
    Rejected,
}

impl ApprovalDecision {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ConditionKind {
    Always,
    Filter,
    Formula,
}

impl ConditionKind {
    pub(crate) const ALL: [Self; 3] = [Self::Always, Self::Filter, Self::Formula];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Filter => "filter",
            Self::Formula => "formula",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

pub(crate) fn locks_field(locked_fields: &[String], field: &str) -> bool {
    locked_fields.is_empty()
        || locked_fields.iter().any(|locked| {
            locked == field
                || field
                    .strip_prefix(locked.as_str())
                    .is_some_and(|rest| rest.starts_with('.'))
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalCheckpoint {
    pub(crate) record: RecordRef,
    pub(crate) moment: ApprovalMoment,
    pub(crate) target_id: Option<Uuid>,
}

impl ApprovalCheckpoint {
    pub(crate) fn stage_change(record: RecordRef, stage_id: Uuid) -> Self {
        Self {
            record,
            moment: ApprovalMoment::StageChange,
            target_id: Some(stage_id),
        }
    }

    pub fn quote_send(record: RecordRef) -> Self {
        Self {
            record,
            moment: ApprovalMoment::QuoteSend,
            target_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSubmission {
    pub checkpoint: ApprovalCheckpoint,
    pub comment: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequestSummary {
    pub id: Uuid,
    pub(crate) policy_id: Uuid,
    pub(crate) policy_name: String,
    pub(crate) record: RecordRef,
    pub moment: ApprovalMoment,
    pub(crate) target_id: Option<Uuid>,
    pub status: ApprovalStatus,
    pub(crate) current_step: u32,
    pub(crate) locked_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionOutcome {
    NotRequired,
    Pending(Vec<ApprovalRequestSummary>),
    Approved(Vec<ApprovalRequestSummary>),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DecisionEntry<'a> {
    pub(crate) request_id: Uuid,
    pub(crate) step: u32,
    pub(crate) user_id: &'a UserId,
    pub(crate) decision: ApprovalDecision,
    pub(crate) comment: Option<&'a str>,
    pub(crate) at: DateTime<Utc>,
}
