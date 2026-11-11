use crate::{AppError, Ctx};
pub use aphel_contract_approvals::*;
use aphel_kernel::{application::shared::RecordRef, database::transaction::MySqlTx};
use async_trait::async_trait;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalCheckpoint {
    pub record: RecordRef,
    pub moment: ApprovalMoment,
    pub target_id: Option<Uuid>,
}

impl ApprovalCheckpoint {
    pub fn new(record: RecordRef, moment: ApprovalMoment, target_id: Option<Uuid>) -> Self {
        Self {
            record,
            moment,
            target_id,
        }
    }

    pub fn stage_change(record: RecordRef, stage_id: Uuid) -> Self {
        Self {
            record,
            moment: ApprovalMoment::STAGE_CHANGE,
            target_id: Some(stage_id),
        }
    }

    pub fn quote_send(record: RecordRef) -> Self {
        Self {
            record,
            moment: ApprovalMoment::QUOTE_SEND,
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
    pub policy_id: Uuid,
    pub policy_name: String,
    pub record: RecordRef,
    pub moment: ApprovalMoment,
    pub target_id: Option<Uuid>,
    pub status: ApprovalStatus,
    pub current_step: u32,
    pub locked_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionOutcome {
    NotRequired,
    Pending(Vec<ApprovalRequestSummary>),
    Approved(Vec<ApprovalRequestSummary>),
}

#[async_trait]
pub trait ApprovalGate: Send + Sync {
    async fn checkpoint(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        checkpoint: &ApprovalCheckpoint,
    ) -> Result<(), AppError>;

    async fn submit(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        submission: &ApprovalSubmission,
    ) -> Result<SubmissionOutcome, AppError>;

    async fn cancel_open(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        record: &RecordRef,
        reason: &str,
    ) -> Result<u32, AppError>;

    async fn requests_for(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
    ) -> Result<Vec<ApprovalRequestSummary>, AppError>;

    async fn retained_targets(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        moment: &ApprovalMoment,
    ) -> Result<Vec<Uuid>, AppError>;
}

pub fn approval_gate(ctx: &Ctx<'_>) -> Option<std::sync::Arc<dyn ApprovalGate>> {
    ctx.services().get::<dyn ApprovalGate>()
}
