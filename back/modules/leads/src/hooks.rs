use async_trait::async_trait;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{
    DisqualificationReason, LEAD_STAGE, LIFECYCLE_STAGE_FIELD, QUALIFIED_STAGE, StageChange,
    is_lifecycle_record_type,
};
use crate::store::{HistoryStore, LeadRecords};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{Actor, RecordType};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{Hook, HookCtx};
use aphel_kernel::records::{ChangedRecord, RecordCtx, RemovedRecord};
use contacts::domain::ContactView;

#[derive(Debug, Clone, PartialEq)]
pub struct Lead {
    pub contact: ContactView,
}

impl Lead {
    pub fn new(contact: ContactView) -> Self {
        Self { contact }
    }

    pub fn id(&self) -> Uuid {
        self.contact.contact.id
    }

    pub(crate) fn name(&self) -> String {
        self.contact.contact.full_name()
    }

    pub(crate) fn stage(&self) -> Option<&str> {
        self.contact
            .contributed
            .get(LIFECYCLE_STAGE_FIELD)
            .and_then(Value::as_str)
    }

    pub(crate) fn is_open(&self) -> bool {
        matches!(self.stage(), Some(LEAD_STAGE) | Some(QUALIFIED_STAGE))
    }
}

pub struct LeadQualifying;

impl Hook for LeadQualifying {
    type Args = Lead;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisqualifyingLead {
    pub(crate) lead: Lead,
    pub(crate) reason: DisqualificationReason,
    pub(crate) note: Option<String>,
}

pub struct LeadDisqualifying;

impl Hook for LeadDisqualifying {
    type Args = DisqualifyingLead;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeadConversion {
    pub lead: Lead,
    pub deal: Map<String, Value>,
    pub company: Option<Map<String, Value>>,
}

pub struct LeadConverting;

impl Hook for LeadConverting {
    type Args = LeadConversion;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvertedLead {
    pub(crate) lead: Lead,
    pub(crate) deal_id: Uuid,
    pub(crate) company_id: Option<Uuid>,
}

pub struct LeadConverted;

impl Hook for LeadConverted {
    type Args = ConvertedLead;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertLeadCommand {
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) amount: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conversion {
    pub(crate) deal_id: Uuid,
    pub(crate) company_id: Option<Uuid>,
}

#[async_trait]
pub trait LeadConverter: Send + Sync {
    async fn convert(
        &self,
        ctx: &Ctx<'_>,
        lead: &Lead,
        command: &ConvertLeadCommand,
    ) -> Result<Conversion, AppError>;
}

pub struct DealLeadConverter;

fn created_id(record: &Value, record_type: &str) -> Result<Uuid, AppError> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| AppError::Internal(format!("the created {record_type} has no id")))
}

pub(crate) fn deal_values(lead: &Lead, command: &ConvertLeadCommand) -> Map<String, Value> {
    let data = &lead.contact.contact.data;
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
    if let Some(company_id) = data.company_id {
        deal.insert("company_id".to_owned(), json!(company_id.to_string()));
    }
    if let Some(source) = &data.source {
        deal.insert("source".to_owned(), json!(source));
    }
    deal
}

#[async_trait]
impl LeadConverter for DealLeadConverter {
    async fn convert(
        &self,
        ctx: &Ctx<'_>,
        lead: &Lead,
        command: &ConvertLeadCommand,
    ) -> Result<Conversion, AppError> {
        let conversion = LeadConversion {
            lead: lead.clone(),
            deal: deal_values(lead, command),
            company: None,
        };
        let mut tx = ctx.begin().await?;
        let conversion = ctx
            .apply_hook::<LeadConverting>(&mut tx, super::KEY, conversion)
            .await?;
        tx.commit().await?;
        let state = ctx.state();
        let record_ctx = RecordCtx::new(ctx.scope, &state);
        let registry = ctx.registry();
        let mut deal = conversion.deal;
        let mut company_id = None;
        if let Some(company) = conversion.company
            && lead.contact.contact.data.company_id.is_none()
        {
            let created = registry
                .handler(RecordType::COMPANY.as_str())?
                .create(&record_ctx, Value::Object(company))
                .await?;
            let id = created_id(&created, RecordType::COMPANY.as_str())?;
            deal.insert("company_id".to_owned(), json!(id.to_string()));
            company_id = Some(id);
        }
        let created = registry
            .handler(RecordType::DEAL.as_str())?
            .create(&record_ctx, Value::Object(deal))
            .await?;
        Ok(Conversion {
            deal_id: created_id(&created, RecordType::DEAL.as_str())?,
            company_id,
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
                actor_type: actor.kind(),
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
                actor_type: actor.kind(),
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
                actor_type: actor.kind(),
                actor_id: actor.id().map(str::to_owned),
                at: chrono::Utc::now(),
            },
        )
        .await
}
