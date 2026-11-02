use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::Deal;
use crate::hooks::{ClosingDeal, StageChange};
use crate::http::{DealResponse, MoveDealPipelineRequest, MoveDealRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, OperationFuture, PluginHook, PluginHookKind, PluginOperation, forward_plugin_hook,
    operation_input, operation_output,
};
use pipelines::domain::PipelineStage;

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

fn stage_change_json(change: &StageChange) -> Value {
    json!({
        "deal": deal_json(change.deal()),
        "previous": change.previous().map(deal_json),
        "from": change.from().map(|from| json!({
            "id": from.id,
            "name": from.name,
            "kind": from.kind.as_str(),
        })),
        "to": stage_json(change.to()),
    })
}

fn closing_json(closing: &ClosingDeal) -> Value {
    json!({
        "deal": deal_json(closing.deal()),
        "previous": closing.previous().map(deal_json),
        "stage": stage_json(closing.stage()),
        "lost_reason": closing.lost_reason().map(|reason| json!({
            "id": reason.id,
            "name": reason.name,
        })),
    })
}

pub(crate) async fn stage_changing(
    ctx: &mut HookCtx<'_>,
    change: &StageChange,
) -> Result<(), AppError> {
    forward_plugin_hook(
        ctx,
        STAGE_CHANGING,
        RecordType::DEAL.as_str(),
        change.deal().id,
        || stage_change_json(change),
    )
    .await
    .map(drop)
}

pub(crate) async fn closing(
    ctx: &mut HookCtx<'_>,
    hook: &'static str,
    closing: &ClosingDeal,
) -> Result<(), AppError> {
    forward_plugin_hook(
        ctx,
        hook,
        RecordType::DEAL.as_str(),
        closing.deal().id,
        || closing_json(closing),
    )
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    let deal = "deal";
    vec![
        Arc::new(PluginHook::new(
            STAGE_CHANGING,
            deal,
            PluginHookKind::Veto,
            "Before a deal moves to another stage; values: { deal, previous, from, to }. An error blocks the move",
        )),
        Arc::new(PluginHook::new(
            WINNING,
            deal,
            PluginHookKind::Veto,
            "Before a deal is won; values: { deal, previous, stage, lost_reason }. An error blocks it",
        )),
        Arc::new(PluginHook::new(
            LOSING,
            deal,
            PluginHookKind::Veto,
            "Before a deal is lost; values: { deal, previous, stage, lost_reason }. An error blocks it",
        )),
        Arc::new(PluginHook::new(
            WON,
            deal,
            PluginHookKind::Action,
            "After a deal is won, inside the transaction; values: { deal, previous, stage }",
        )),
        Arc::new(PluginHook::new(
            LOST,
            deal,
            PluginHookKind::Action,
            "After a deal is lost, inside the transaction; values: { deal, previous, stage, lost_reason }",
        )),
    ]
}

#[derive(Deserialize)]
struct MoveStageInput {
    id: String,
    #[serde(flatten)]
    request: MoveDealRequest,
}

#[derive(Deserialize)]
struct MovePipelineInput {
    id: String,
    #[serde(flatten)]
    request: MoveDealPipelineRequest,
}

fn move_stage<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: MoveStageInput = operation_input(input)?;
        let deal = service::move_to_stage(ctx, &input.id, input.request.into_command(None)).await?;
        operation_output(DealResponse::from(deal))
    })
}

fn move_pipeline<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: MovePipelineInput = operation_input(input)?;
        let deal =
            service::move_to_pipeline(ctx, &input.id, input.request.into_command(None)).await?;
        operation_output(DealResponse::from(deal))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::write(
            "deals.move_stage",
            "Move a deal to a stage (a won/lost stage wins/loses it): { id, stage_id, lost_reason_id?, lost_reason_note?, fields? }",
            move_stage,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "deals.move_pipeline",
            "Move a deal to another pipeline: { id, pipeline_id, stage_id?, lost_reason_id?, lost_reason_note?, fields? }",
            move_pipeline,
        )
        .with_durable_mutations()),
    ]
}
