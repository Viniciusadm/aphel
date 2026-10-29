use std::sync::Arc;

use async_trait::async_trait;

use crate::KEY;
use crate::domain::{
    ApprovalCheckpoint, ApprovalRequestSummary, ApprovalSubmission, SubmissionOutcome,
};
use crate::engine::{self, record_not_found};
use crate::store::{ApprovalDirectory, ApprovalStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::ChangedRecord;

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
}

pub struct EngineApprovalGate;

#[async_trait]
impl ApprovalGate for EngineApprovalGate {
    async fn checkpoint(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        checkpoint: &ApprovalCheckpoint,
    ) -> Result<(), AppError> {
        engine::checkpoint(ctx, tx, checkpoint).await
    }

    async fn submit(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        submission: &ApprovalSubmission,
    ) -> Result<SubmissionOutcome, AppError> {
        engine::submit(ctx, tx, submission, None).await
    }

    async fn cancel_open(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        record: &RecordRef,
        reason: &str,
    ) -> Result<u32, AppError> {
        engine::cancel_open(ctx, tx, record, reason).await
    }

    async fn requests_for(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
    ) -> Result<Vec<ApprovalRequestSummary>, AppError> {
        let definition = ctx.registry().require(record.record_type.as_str())?;
        let visibility = ctx.scope.record_filter(&definition.read_permission)?;
        if ApprovalDirectory::of(ctx)
            .record_state(ctx.scope, definition, record.record_id, Some(&visibility))
            .await?
            .is_none()
        {
            return Err(record_not_found());
        }
        Ok(ApprovalStore::of(ctx)
            .requests_for_record(record)
            .await?
            .iter()
            .map(|request| request.summary())
            .collect())
    }
}

pub fn approval_gate(ctx: &Ctx<'_>) -> Option<Arc<dyn ApprovalGate>> {
    if !ctx.app().is_enabled(KEY) {
        return None;
    }
    ctx.services().get::<dyn ApprovalGate>()
}

pub(crate) async fn guard_write(
    ctx: &mut HookCtx<'_>,
    args: &ChangedRecord,
) -> Result<(), AppError> {
    let module_ctx = Ctx::from_parts(
        ctx.scope,
        ctx.pool.clone(),
        args.registry.clone(),
        Arc::new(MySqlEventPublisher::new()),
        Arc::new(MySqlAuditLog::new()),
    );
    engine::guard(&module_ctx, ctx.tx, &args.write()).await
}
