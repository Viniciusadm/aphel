use aphel_contract_crm::CrmRecordTypes;
use serde_json::{Value, json};

use crate::domain::{Deal, StageSummary};
use crate::service::{pipeline_condition, restrict_to_pipelines};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{Hook, HookCtx, PluginHookArgs};
use aphel_kernel::records::QueryConstraints;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::workflow::WorkflowReader;
use aphel_sdk::workflow::{LostReason, PipelineStage, pipeline_restriction, sees_every_pipeline};

pub(crate) const STAGE_CHANGING: &str = "deal.stage_changing";
pub(crate) const WINNING: &str = "deal.winning";
pub(crate) const LOSING: &str = "deal.losing";
pub(crate) const WON: &str = "deal.won";
pub(crate) const LOST: &str = "deal.lost";

fn deal_json(deal: &Deal) -> Value {
    json!({
        "id": deal.id,
        "title": deal.title,
        "owner_id": deal.owner_id.as_ref().map(|owner| owner.as_str()),
        "pipeline_id": deal.pipeline_id,
        "stage_id": deal.stage_id,
        "amount": deal.amount,
        "currency": deal.currency,
        "amount_base": deal.revenue.amount_base,
        "probability": deal.probability,
        "expected_close_date": deal.expected_close_date,
        "status": deal.status.as_str(),
        "closed_at": deal.closed_at,
        "lost_reason_id": deal.lost_reason_id,
        "lost_reason_note": deal.lost_reason_note,
        "source": deal.source,
        "contact_id": deal.contact_id,
        "company_id": deal.company_id,
        "stage_entered_at": deal.stage_entered_at,
        "created_at": deal.created_at,
        "updated_at": deal.updated_at,
    })
}

fn stage_json(stage: &PipelineStage) -> Value {
    json!({
        "id": stage.id,
        "pipeline_id": stage.pipeline_id,
        "name": stage.name,
        "kind": stage.kind.as_str(),
        "probability": stage.probability,
    })
}

impl PluginHookArgs for StageChange {
    fn plugin_record_id(&self) -> String {
        self.deal().id.to_string()
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "deal": deal_json(self.deal()),
            "previous": self.previous().map(deal_json),
            "from": self.from().map(|from| json!({
                "id": from.id,
                "name": from.name,
                "kind": from.kind.as_str(),
            })),
            "to": stage_json(self.to()),
        })
    }
}

impl PluginHookArgs for ClosingDeal {
    fn plugin_record_id(&self) -> String {
        self.deal().id.to_string()
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "deal": deal_json(self.deal()),
            "previous": self.previous().map(deal_json),
            "stage": stage_json(self.stage()),
            "lost_reason": self.lost_reason().map(|reason| json!({
                "id": reason.id,
                "name": reason.name,
            })),
        })
    }
}

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
