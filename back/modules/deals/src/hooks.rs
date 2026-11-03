use aphel_contract_crm::CrmRecordTypes;

use crate::domain::{Deal, StageSummary};
use crate::service::{pipeline_condition, restrict_to_pipelines};
use aphel_kernel::application::query::QueryPlan;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{Hook, HookCtx};
use aphel_kernel::records::QueryConstraints;
use aphel_sdk::workflow::WorkflowReader;
use aphel_sdk::workflow::{LostReason, PipelineStage, pipeline_restriction, sees_every_pipeline};

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

pub(crate) async fn restrict_search(
    ctx: &mut HookCtx<'_>,
    mut plan: QueryPlan,
) -> Result<QueryPlan, AppError> {
    if plan.record_type.record_type != RecordType::DEAL || sees_every_pipeline(ctx.scope) {
        return Ok(plan);
    }
    let workflow = WorkflowReader::new(ctx.pool.clone(), ctx.services)?
        .list_pipelines()
        .await?;
    if let Some(pipeline_ids) = pipeline_restriction(ctx.scope, &workflow) {
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
    let workflow = WorkflowReader::new(ctx.pool.clone(), ctx.services)?
        .list_pipelines()
        .await?;
    if let Some(pipeline_ids) = pipeline_restriction(ctx.scope, &workflow) {
        constraints
            .conditions
            .push(pipeline_condition(&pipeline_ids));
    }
    Ok(constraints)
}
