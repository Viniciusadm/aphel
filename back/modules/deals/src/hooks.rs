use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::{Deal, StageSummary};
use crate::service::{pipeline_condition, restrict_to_pipelines};
use aphel_kernel::Ctx;
use aphel_kernel::application::query::QueryPlan;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{Hook, HookCtx};
use aphel_kernel::records::QueryConstraints;
use pipelines::domain::{LostReason, PipelineStage, pipeline_restriction, sees_every_pipeline};
use pipelines::store::PipelineStore;

#[derive(Debug, Clone, PartialEq)]
pub struct StageChange {
    pub(crate) deal: Deal,
    pub(crate) previous: Option<Deal>,
    pub(crate) from: Option<StageSummary>,
    pub(crate) to: PipelineStage,
}

impl StageChange {
    pub fn deal(&self) -> &Deal {
        &self.deal
    }

    pub fn previous(&self) -> Option<&Deal> {
        self.previous.as_ref()
    }

    pub fn from(&self) -> Option<&StageSummary> {
        self.from.as_ref()
    }

    pub fn to(&self) -> &PipelineStage {
        &self.to
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DealStageChanging;

impl Hook for DealStageChanging {
    type Args = StageChange;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosingDeal {
    pub(crate) deal: Deal,
    pub(crate) previous: Option<Deal>,
    pub(crate) stage: PipelineStage,
    pub(crate) lost_reason: Option<LostReason>,
}

impl ClosingDeal {
    pub fn deal(&self) -> &Deal {
        &self.deal
    }

    pub fn previous(&self) -> Option<&Deal> {
        self.previous.as_ref()
    }

    pub fn stage(&self) -> &PipelineStage {
        &self.stage
    }

    pub fn lost_reason(&self) -> Option<&LostReason> {
        self.lost_reason.as_ref()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DealWinning;

impl Hook for DealWinning {
    type Args = ClosingDeal;
}

#[derive(Debug, Clone, Copy)]
pub struct DealLosing;

impl Hook for DealLosing {
    type Args = ClosingDeal;
}

#[derive(Debug, Clone, Copy)]
pub struct DealWon;

impl Hook for DealWon {
    type Args = ClosingDeal;
}

#[derive(Debug, Clone, Copy)]
pub struct DealLost;

impl Hook for DealLost {
    type Args = ClosingDeal;
}

#[async_trait]
pub trait AmountCalculator: Send + Sync {
    async fn amount_locked(&self, ctx: &Ctx<'_>, deal_id: Uuid) -> Result<bool, AppError>;

    async fn refresh_revenue(&self, ctx: &Ctx<'_>, deal: &mut Deal) -> Result<(), AppError>;

    async fn sync_rate(
        &self,
        tx: &mut MySqlTx,
        deal_id: Uuid,
        rate: Option<Decimal>,
    ) -> Result<(), AppError>;

    async fn sync_currency_rate(
        &self,
        tx: &mut MySqlTx,
        currency: &str,
        rate: Option<Decimal>,
    ) -> Result<(), AppError>;
}

pub struct ManualAmountCalculator;

#[async_trait]
impl AmountCalculator for ManualAmountCalculator {
    async fn amount_locked(&self, _ctx: &Ctx<'_>, _deal_id: Uuid) -> Result<bool, AppError> {
        Ok(false)
    }

    async fn refresh_revenue(&self, _ctx: &Ctx<'_>, _deal: &mut Deal) -> Result<(), AppError> {
        Ok(())
    }

    async fn sync_rate(
        &self,
        _tx: &mut MySqlTx,
        _deal_id: Uuid,
        _rate: Option<Decimal>,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn sync_currency_rate(
        &self,
        _tx: &mut MySqlTx,
        _currency: &str,
        _rate: Option<Decimal>,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

pub(crate) async fn restrict_search(
    ctx: &mut HookCtx<'_>,
    mut plan: QueryPlan,
) -> Result<QueryPlan, AppError> {
    if plan.record_type.record_type != RecordType::DEAL || sees_every_pipeline(ctx.scope) {
        return Ok(plan);
    }
    let pipelines = PipelineStore::new(ctx.pool.clone())
        .list_pipelines()
        .await?;
    if let Some(pipeline_ids) = pipeline_restriction(ctx.scope, &pipelines) {
        restrict_to_pipelines(&mut plan, &pipeline_ids);
    }
    Ok(plan)
}

pub(crate) async fn restrict_query(
    ctx: &mut HookCtx<'_>,
    mut constraints: QueryConstraints,
) -> Result<QueryConstraints, AppError> {
    if constraints.record_type != RecordType::DEAL || sees_every_pipeline(ctx.scope) {
        return Ok(constraints);
    }
    let pipelines = PipelineStore::new(ctx.pool.clone())
        .list_pipelines()
        .await?;
    if let Some(pipeline_ids) = pipeline_restriction(ctx.scope, &pipelines) {
        constraints
            .conditions
            .push(pipeline_condition(&pipeline_ids));
    }
    Ok(constraints)
}
