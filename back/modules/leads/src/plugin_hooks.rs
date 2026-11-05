use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::hooks::{ConvertedLead, DisqualifyingLead, Lead, LeadConversion};
use crate::http::{ConvertLeadRequest, DisqualifyLeadRequest, LeadActionResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, OperationFuture, PluginHook, PluginHookKind, forward_plugin_hook, operation_input,
    operation_output,
};

pub(crate) const QUALIFYING: &str = "lead.qualifying";
pub(crate) const DISQUALIFYING: &str = "lead.disqualifying";
pub(crate) const CONVERTING: &str = "lead.converting";
pub(crate) const CONVERTED: &str = "lead.converted";

const CONTACT: &str = aphel_contract_crm::keys::CONTACT;

fn lead_json(lead: &Lead) -> Value {
    Value::Object(lead.values.clone())
}

pub(crate) async fn qualifying(ctx: &mut HookCtx<'_>, lead: &Lead) -> Result<(), AppError> {
    forward_plugin_hook(
        ctx,
        QUALIFYING,
        CONTACT,
        lead.id(),
        || json!({ "lead": lead_json(lead) }),
    )
    .await
    .map(drop)
}

pub(crate) async fn disqualifying(
    ctx: &mut HookCtx<'_>,
    args: &DisqualifyingLead,
) -> Result<(), AppError> {
    forward_plugin_hook(ctx, DISQUALIFYING, CONTACT, args.lead().id(), || {
        json!({
            "lead": lead_json(args.lead()),
            "reason": {
                "id": args.reason_id(),
                "key": args.reason.key,
                "name": args.reason.name,
            },
            "note": args.note(),
        })
    })
    .await
    .map(drop)
}

fn merge_object(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (key, value) in patch {
        target.insert(key.clone(), value.clone());
    }
}

pub(crate) async fn converting(
    ctx: &mut HookCtx<'_>,
    mut conversion: LeadConversion,
) -> Result<LeadConversion, AppError> {
    let patches = forward_plugin_hook(ctx, CONVERTING, CONTACT, conversion.lead.id(), || {
        json!({
            "contact_id": conversion.lead.id(),
            "lead": lead_json(&conversion.lead),
            "deal": conversion.deal,
            "company": conversion.company,
        })
    })
    .await?;
    for patch in patches {
        if let Some(Value::Object(deal)) = patch.get("deal") {
            merge_object(&mut conversion.deal, deal);
        }
        if let Some(Value::Object(company)) = patch.get("company") {
            merge_object(conversion.company.get_or_insert_with(Map::new), company);
        }
    }
    Ok(conversion)
}

pub(crate) async fn converted(
    ctx: &mut HookCtx<'_>,
    converted: &ConvertedLead,
) -> Result<(), AppError> {
    forward_plugin_hook(ctx, CONVERTED, CONTACT, converted.lead().id(), || {
        json!({
            "lead": lead_json(converted.lead()),
            "deal_id": converted.deal_id(),
            "company_id": converted.company_id(),
        })
    })
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![
        Arc::new(PluginHook::new(
            QUALIFYING,
            CONTACT,
            PluginHookKind::Veto,
            "Before a lead is qualified in the lead inbox; values: { lead }. An error blocks the qualification",
        )),
        Arc::new(PluginHook::new(
            DISQUALIFYING,
            CONTACT,
            PluginHookKind::Veto,
            "Before a lead is disqualified; values: { lead, reason: { id, key, name }, note }. An error blocks it",
        )),
        Arc::new(PluginHook::new(
            CONVERTING,
            CONTACT,
            PluginHookKind::Filter,
            "Values of the deal (and optional company) a lead conversion creates; values: { contact_id, lead, deal, company }. Return { deal?: {..}, company?: {..} } to merge field values into them (a company object makes the conversion create a company when the contact has none)",
        )),
        Arc::new(PluginHook::new(
            CONVERTED,
            CONTACT,
            PluginHookKind::Action,
            "After a lead was converted, inside the transaction; values: { lead, deal_id, company_id }",
        )),
    ]
}

#[derive(Deserialize)]
struct ContactId {
    contact_id: String,
}

#[derive(Deserialize)]
struct DisqualifyInput {
    contact_id: String,
    #[serde(flatten)]
    request: DisqualifyLeadRequest,
}

#[derive(Deserialize)]
struct ConvertInput {
    contact_id: String,
    #[serde(flatten)]
    request: ConvertLeadRequest,
}

fn qualify<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ContactId = operation_input(input)?;
        let result = service::qualify_lead(ctx, &input.contact_id, None).await?;
        operation_output(LeadActionResponse::from(result))
    })
}

fn disqualify<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: DisqualifyInput = operation_input(input)?;
        let result =
            service::disqualify_lead(ctx, &input.contact_id, input.request.into_command(None))
                .await?;
        operation_output(LeadActionResponse::from(result))
    })
}

fn convert<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ConvertInput = operation_input(input)?;
        let result =
            service::convert_lead(ctx, &input.contact_id, input.request.into_command(None)).await?;
        operation_output(LeadActionResponse::from(result))
    })
}

const OP_LEADS_QUALIFY: OperationKey<Value, Value> = operation!("leads.qualify");
const OP_LEADS_DISQUALIFY: OperationKey<Value, Value> = operation!("leads.disqualify");
const OP_LEADS_CONVERT: OperationKey<Value, Value> = operation!("leads.convert");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_LEADS_QUALIFY, OperationSpec::write("Qualify a contact in the lead stage: { contact_id } -> { contact_id, lifecycle_stage, deal_id }").durable(), qualify);
    app.operation(OP_LEADS_DISQUALIFY, OperationSpec::write("Disqualify an open lead with a reason: { contact_id, reason_id, note? } -> { contact_id, lifecycle_stage, deal_id }").durable(), disqualify);
    app.operation(OP_LEADS_CONVERT, OperationSpec::write("Convert an open lead into a deal: { contact_id, pipeline_id, stage_id?, title?, amount?, currency? } -> { contact_id, lifecycle_stage, deal_id }").durable(), convert);
}
