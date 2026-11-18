use aphel_contract_crm::CrmRecordTypes;
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{
    DisqualificationReason, LEAD_STAGE, LIFECYCLE_STAGE_FIELD, QUALIFIED_STAGE, StageChange,
    is_lifecycle_record_type,
};
use crate::store::{HistoryStore, LeadRecords};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{Hook, HookCtx, HookForwarding, PluginHookArgs};
use aphel_kernel::records::{ChangedRecord, RemovedRecord};
use aphel_sdk::platform::application::shared::{Actor, RecordType};
use aphel_sdk::platform::database::transaction::Tx;

pub use aphel_sdk::leads::{
    Conversion, ConvertLeadCommand, Lead, LeadConversion, LeadConverter, LeadConverting,
    LeadQualifying,
};
pub(crate) trait LeadState {
    fn name(&self) -> String;
    fn stage(&self) -> Option<&str>;
    fn is_open(&self) -> bool;
}
impl LeadState for Lead {
    fn name(&self) -> String {
        self.values
            .get("full_name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| self.id().to_string())
    }
    fn stage(&self) -> Option<&str> {
        self.values
            .get(LIFECYCLE_STAGE_FIELD)
            .and_then(Value::as_str)
    }
    fn is_open(&self) -> bool {
        matches!(self.stage(), Some(LEAD_STAGE) | Some(QUALIFIED_STAGE))
    }
}

pub(crate) const QUALIFYING: &str = "lead.qualifying";
pub(crate) const DISQUALIFYING: &str = "lead.disqualifying";
pub(crate) const CONVERTING: &str = "lead.converting";
pub(crate) const CONVERTED: &str = "lead.converted";

fn lead_json(lead: &Lead) -> Value {
    Value::Object(lead.values.clone())
}

pub(crate) fn qualifying_forwarding() -> HookForwarding<Lead> {
    HookForwarding {
        record_id: |lead| lead.id().to_string(),
        payload: |lead| json!({ "lead": lead_json(lead) }),
        record_type: |_| None,
        apply: None,
    }
}

fn merge_object(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (key, value) in patch {
        target.insert(key.clone(), value.clone());
    }
}

pub(crate) fn converting_forwarding() -> HookForwarding<LeadConversion> {
    HookForwarding {
        record_id: |conversion| conversion.lead.id().to_string(),
        payload: |conversion| {
            json!({
                "contact_id": conversion.lead.id(),
                "lead": lead_json(&conversion.lead),
                "deal": conversion.deal,
                "company": conversion.company,
            })
        },
        record_type: |_| None,
        apply: Some(|mut conversion, patches| {
            for patch in patches {
                if let Some(Value::Object(deal)) = patch.get("deal") {
                    merge_object(&mut conversion.deal, deal);
                }
                if let Some(Value::Object(company)) = patch.get("company") {
                    merge_object(conversion.company.get_or_insert_with(Map::new), company);
                }
            }
            Ok(conversion)
        }),
    }
}

impl PluginHookArgs for DisqualifyingLead {
    fn plugin_record_id(&self) -> String {
        self.lead().id().to_string()
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "lead": lead_json(self.lead()),
            "reason": {
                "id": self.reason_id(),
                "key": self.reason.key,
                "name": self.reason.name,
            },
            "note": self.note(),
        })
    }
}

impl PluginHookArgs for ConvertedLead {
    fn plugin_record_id(&self) -> String {
        self.lead().id().to_string()
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "lead": lead_json(self.lead()),
            "deal_id": self.deal_id(),
            "company_id": self.company_id(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisqualifyingLead {
    pub(crate) lead: Lead,
    pub(crate) reason: DisqualificationReason,
    pub(crate) note: Option<String>,
}

impl DisqualifyingLead {
    pub fn lead(&self) -> &Lead {
        &self.lead
    }

    pub fn reason_id(&self) -> Uuid {
        self.reason.id
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LeadDisqualifying;

impl Hook for LeadDisqualifying {
    type Args = DisqualifyingLead;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvertedLead {
    pub(crate) lead: Lead,
    pub(crate) deal_id: Uuid,
    pub(crate) company_id: Option<Uuid>,
}

impl ConvertedLead {
    pub fn lead(&self) -> &Lead {
        &self.lead
    }

    pub fn deal_id(&self) -> Uuid {
        self.deal_id
    }

    pub fn company_id(&self) -> Option<Uuid> {
        self.company_id
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LeadConverted;

impl Hook for LeadConverted {
    type Args = ConvertedLead;
}

pub struct DealLeadConverter;

pub(crate) fn deal_values(lead: &Lead, command: &ConvertLeadCommand) -> Map<String, Value> {
    let title = command
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map_or_else(|| lead.name(), str::to_owned);
    let mut deal = Map::new();
    deal.insert("title".to_owned(), json!(title));
    deal.insert("pipeline_id".to_owned(), json!(command.pipeline_id));
    deal.insert("contact_id".to_owned(), json!(lead.id().to_string()));
    if let Some(stage_id) = &command.stage_id {
        deal.insert("stage_id".to_owned(), json!(stage_id));
    }
    if let Some(amount) = &command.amount {
        deal.insert("amount".to_owned(), json!(amount));
    }
    if let Some(currency) = &command.currency {
        deal.insert("currency".to_owned(), json!(currency));
    }
    if let Some(company_id) = lead.company_id() {
        deal.insert("company_id".to_owned(), json!(company_id.to_string()));
    }
    if let Some(source) = lead.values.get("source").filter(|value| !value.is_null()) {
        deal.insert("source".to_owned(), json!(source));
    }
    deal
}

#[async_trait]
impl LeadConverter for DealLeadConverter {
    async fn convert(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut Tx,
        lead: &Lead,
        command: &ConvertLeadCommand,
    ) -> Result<Conversion, AppError> {
        let conversion = LeadConversion {
            lead: lead.clone(),
            deal: deal_values(lead, command),
            company: None,
        };
        let conversion = ctx
            .apply_hook(LeadConverting, tx, super::KEY, conversion)
            .await?;
        let mut deal = conversion.deal;
        let mut company_id = None;
        let mut pending = Vec::new();
        if let Some(company) = conversion.company
            && lead.company_id().is_none()
        {
            let created = ctx
                .create_record_in_tx(tx, RecordType::COMPANY.as_str(), Value::Object(company))
                .await?;
            let id = created.record.record_id;
            pending.push(created);
            deal.insert("company_id".to_owned(), json!(id.to_string()));
            company_id = Some(id);
        }
        let created = ctx
            .create_record_in_tx(tx, RecordType::DEAL.as_str(), Value::Object(deal))
            .await?;
        let deal_id = created.record.record_id;
        pending.push(created);
        Ok(Conversion {
            deal_id,
            company_id,
            pending,
        })
    }
}

fn stage_text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

pub(crate) async fn validate_stage(
    ctx: &mut HookCtx<'_>,
    args: &ChangedRecord,
) -> Result<(), AppError> {
    let record_type = &args.record_type;
    if !is_lifecycle_record_type(record_type) {
        return Ok(());
    }
    let write = args.write();
    let Some(change) = write.changed(LIFECYCLE_STAGE_FIELD) else {
        return Ok(());
    };
    let from_stage = stage_text(&change.old);
    let to_stage = stage_text(&change.new);
    if from_stage == to_stage {
        return Ok(());
    }
    if let Some(stage) = &to_stage {
        let options = LeadRecords::new(ctx.pool.clone(), args.registry.clone())
            .stage_options(record_type)
            .await?;
        if !options
            .iter()
            .any(|option| option.is_active && option.value == *stage)
        {
            return Err(AppError::invalid_fields(
                format!("invalid {}", record_type.as_str()),
                vec![FieldViolation::new(
                    LIFECYCLE_STAGE_FIELD,
                    "must be one of the active lifecycle stages",
                )],
            ));
        }
    }
    let actor = Actor::from_scope(ctx.scope);
    HistoryStore::new(ctx.pool.clone())
        .record(
            ctx.tx,
            &StageChange {
                record_type: record_type.clone(),
                record_id: args.record.record_id,
                from_stage,
                to_stage,
                actor_type: actor.kind().to_owned(),
                actor_id: actor.id().map(str::to_owned),
                at: chrono::Utc::now(),
            },
        )
        .await
}

pub(crate) async fn restart_stage(
    ctx: &mut HookCtx<'_>,
    args: &RemovedRecord,
) -> Result<(), AppError> {
    if !is_lifecycle_record_type(&args.record_type) {
        return Ok(());
    }
    let Some(Some(stage)) = LeadRecords::new(ctx.pool.clone(), args.registry.clone())
        .current_stage_in(ctx.tx, &args.record_type, args.record.record_id)
        .await?
    else {
        return Ok(());
    };
    let actor = Actor::from_scope(ctx.scope);
    HistoryStore::new(ctx.pool.clone())
        .record(
            ctx.tx,
            &StageChange {
                record_type: args.record_type.clone(),
                record_id: args.record.record_id,
                from_stage: None,
                to_stage: Some(stage),
                actor_type: actor.kind().to_owned(),
                actor_id: actor.id().map(str::to_owned),
                at: chrono::Utc::now(),
            },
        )
        .await
}

pub(crate) async fn close_stage(
    ctx: &mut HookCtx<'_>,
    args: &RemovedRecord,
) -> Result<(), AppError> {
    if !is_lifecycle_record_type(&args.record_type) {
        return Ok(());
    }
    let actor = Actor::from_scope(ctx.scope);
    HistoryStore::new(ctx.pool.clone())
        .record(
            ctx.tx,
            &StageChange {
                record_type: args.record_type.clone(),
                record_id: args.record.record_id,
                from_stage: None,
                to_stage: None,
                actor_type: actor.kind().to_owned(),
                actor_id: actor.id().map(str::to_owned),
                at: chrono::Utc::now(),
            },
        )
        .await
}
