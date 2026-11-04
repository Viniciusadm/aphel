use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::workflow::WorkflowRecordTypes;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEAL_RENEWAL_CREATED, DEFAULT_RENEWAL_DAYS_BEFORE, DEFAULT_RENEWAL_TITLE_PREFIX,
    ExistingRenewal, MAX_RENEWAL_DAYS_BEFORE, MAX_RENEWAL_TITLE_PREFIX_LENGTH,
    PIPELINE_RENEWAL_UPDATED, RenewalOwnerMode, RenewalSettings, RenewalSettingsView,
    SaveRenewalSettingsCommand, renewal_title,
};
use crate::store::RenewalStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef, RecordType, ensure_version,
};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, changes_payload, publish_mutation};
use aphel_sdk::line_items::{LineItem, Recurrence};
#[derive(serde::Deserialize)]
struct RenewalSource {
    id: Uuid,
    title: String,
    amount: Option<rust_decimal::Decimal>,
    currency: String,
    contact_id: Option<Uuid>,
    company_id: Option<Uuid>,
    owner_id: Option<String>,
    source: Option<String>,
    contract_end: Option<chrono::NaiveDate>,
}
use aphel_sdk::deals::{MAX_DEAL_TITLE_LENGTH, RenewalDealCreation, RenewalDealWriter, date};

use aphel_sdk::workflow::{
    Pipeline, StageKind, WorkflowReader, first_active_of_kind, parse_id, pipeline_not_found,
};

const INVALID_RENEWAL_SETTINGS: &str = "invalid renewal settings";

pub(crate) async fn get(ctx: &Ctx<'_>, pipeline_id: &str) -> Result<RenewalSettingsView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let pipeline = pipeline(ctx, pipeline_id).await?;
    match RenewalStore::of(ctx).find(pipeline.id).await? {
        Some(settings) => Ok(RenewalSettingsView {
            settings,
            configured: true,
        }),
        None => Ok(RenewalSettingsView {
            settings: defaults(ctx, &pipeline).await?,
            configured: false,
        }),
    }
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<RenewalSettings>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    RenewalStore::of(ctx).list().await
}

pub(crate) async fn save(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    command: SaveRenewalSettingsCommand,
) -> Result<RenewalSettingsView, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let pipeline = pipeline(ctx, pipeline_id).await?;
    let store = RenewalStore::of(ctx);
    let current = store.find(pipeline.id).await?;
    if let Some(current) = &current {
        ensure_version(command.expected_updated_at, current.updated_at)?;
    }
    let mut settings = match &current {
        Some(current) => current.clone(),
        None => defaults(ctx, &pipeline).await?,
    };
    let mut violations = Vec::new();
    if let Some(enabled) = command.is_enabled {
        settings.is_enabled = enabled;
    }
    if let Some(days) = command.days_before {
        match u16::try_from(days) {
            Ok(days) if days <= MAX_RENEWAL_DAYS_BEFORE => settings.days_before = days,
            _ => violations.push(FieldViolation::new(
                "days_before",
                format!("must be between 0 and {MAX_RENEWAL_DAYS_BEFORE}"),
            )),
        }
    }
    if let Some(prefix) = &command.title_prefix {
        if prefix.chars().count() > MAX_RENEWAL_TITLE_PREFIX_LENGTH {
            violations.push(FieldViolation::new(
                "title_prefix",
                format!("must have at most {MAX_RENEWAL_TITLE_PREFIX_LENGTH} characters"),
            ));
        } else {
            settings.title_prefix = prefix.clone();
        }
    }
    if let Some(mode) = command.owner_mode {
        settings.owner_mode = mode;
    }
    apply_target(ctx, &command, &mut settings, &mut violations).await?;
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            INVALID_RENEWAL_SETTINGS,
            violations,
        ));
    }
    let old = current
        .as_ref()
        .map(RenewalSettings::snapshot)
        .unwrap_or(Value::Null);
    let changes = diff_values(&old, &settings.snapshot(), &DiffOptions::default())?;
    if current.is_some() && changes.is_empty() {
        return Ok(RenewalSettingsView {
            settings,
            configured: true,
        });
    }
    let now = ctx.now();
    settings.updated_by = Some(scope.user_id().clone());
    settings.updated_at = now;
    if current.is_none() {
        settings.created_at = now;
    }
    let mut tx = ctx.begin().await?;
    store.save(&mut tx, &settings).await?;
    let mut payload = changes_payload(&changes);
    payload["pipeline"] = json!({ "id": pipeline.id.to_string(), "name": pipeline.name });
    payload["settings"] = settings.snapshot();
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::PIPELINE, pipeline.id),
            event_type: PIPELINE_RENEWAL_UPDATED,
            payload,
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(RenewalSettingsView {
        settings,
        configured: true,
    })
}

async fn apply_target(
    ctx: &Ctx<'_>,
    command: &SaveRenewalSettingsCommand,
    settings: &mut RenewalSettings,
    violations: &mut Vec<FieldViolation>,
) -> Result<(), AppError> {
    let workflow = WorkflowReader::of(ctx)?;
    if let Some(value) = &command.target_pipeline_id {
        let target = match parse_id(value) {
            Some(id) => workflow.find_pipeline(id).await?,
            None => None,
        };
        match target {
            Some(target) if target.id != settings.target_pipeline_id => {
                settings.target_pipeline_id = target.id;
                if command.target_stage_id.is_none() {
                    let stages = workflow.list_stages(target.id).await?;
                    settings.target_stage_id = first_active_of_kind(&stages, StageKind::Open)
                        .map_or(Uuid::nil(), |stage| stage.id);
                }
            }
            Some(_) => {}
            None => violations.push(FieldViolation::new(
                "target_pipeline_id",
                "pipeline was not found",
            )),
        }
    }
    if let Some(value) = &command.target_stage_id {
        match parse_id(value) {
            Some(id) => settings.target_stage_id = id,
            None => {
                violations.push(FieldViolation::new("target_stage_id", "must be a valid id"));
                return Ok(());
            }
        }
    }
    match workflow.find_stage(settings.target_stage_id).await? {
        Some(stage) if stage.pipeline_id != settings.target_pipeline_id => {
            violations.push(FieldViolation::new(
                "target_stage_id",
                "must be a stage of the target pipeline",
            ));
        }
        Some(stage) if stage.kind != StageKind::Open || !stage.is_active => {
            violations.push(FieldViolation::new(
                "target_stage_id",
                "must be an active open stage",
            ));
        }
        Some(_) => {}
        None => violations.push(FieldViolation::new(
            "target_stage_id",
            "the target pipeline has no active open stage",
        )),
    }
    Ok(())
}

async fn defaults(ctx: &Ctx<'_>, pipeline: &Pipeline) -> Result<RenewalSettings, AppError> {
    let stages = WorkflowReader::of(ctx)?.list_stages(pipeline.id).await?;
    let now = ctx.now();
    Ok(RenewalSettings {
        pipeline_id: pipeline.id,
        is_enabled: false,
        days_before: DEFAULT_RENEWAL_DAYS_BEFORE,
        target_pipeline_id: pipeline.id,
        target_stage_id: first_active_of_kind(&stages, StageKind::Open)
            .map_or(Uuid::nil(), |stage| stage.id),
        title_prefix: DEFAULT_RENEWAL_TITLE_PREFIX.to_owned(),
        owner_mode: RenewalOwnerMode::Original,
        updated_by: None,
        created_at: now,
        updated_at: now,
    })
}

async fn pipeline(ctx: &Ctx<'_>, pipeline_id: &str) -> Result<Pipeline, AppError> {
    let id = parse_id(pipeline_id).ok_or_else(pipeline_not_found)?;
    WorkflowReader::of(ctx)?
        .find_pipeline(id)
        .await?
        .ok_or_else(pipeline_not_found)
}

fn items_enabled(ctx: &Ctx<'_>) -> bool {
    ctx.app().is_enabled(aphel_sdk::line_items::KEY)
}

pub(crate) async fn renew(
    ctx: &Ctx<'_>,
    settings: &RenewalSettings,
    original_id: Uuid,
) -> Result<bool, AppError> {
    let store = RenewalStore::of(ctx);
    let original: RenewalSource = serde_json::from_value(
        ctx.load_record(aphel_sdk::deals::RECORD_TYPE, original_id)
            .await?
            .ok_or_else(|| AppError::NotFound("the original deal is unavailable".into()))?,
    )
    .map_err(|_| AppError::Forbidden("the original deal fields are not readable".into()))?;
    let recurring = recurring_items(ctx, original_id).await?;
    let existing = match store.find_renewal(ctx, original_id).await? {
        Some(existing) => existing,
        None => match create(ctx, settings, &original, &recurring).await {
            Ok(deal_id) => ExistingRenewal {
                deal_id,
                deleted: false,
            },
            Err(error) => match store.find_renewal(ctx, original_id).await? {
                Some(existing) => existing,
                None => return Err(error),
            },
        },
    };
    if !existing.deleted {
        copy_items(ctx, existing.deal_id, &recurring).await?;
    }
    finish(ctx, &original, existing.deal_id).await
}

async fn recurring_items(ctx: &Ctx<'_>, deal_id: Uuid) -> Result<Vec<LineItem>, AppError> {
    if !items_enabled(ctx) {
        return Ok(Vec::new());
    }
    Ok(ctx
        .services()
        .require::<dyn aphel_sdk::line_items::LineItemsPort>()?
        .rows(
            ctx,
            &aphel_kernel::application::shared::RecordRef::parse(
                "deals.deal",
                &deal_id.to_string(),
            )?,
        )
        .await?
        .into_iter()
        .filter(|item| item.data.recurrence != Recurrence::None)
        .collect())
}

async fn create(
    ctx: &Ctx<'_>,
    settings: &RenewalSettings,
    original: &RenewalSource,
    recurring: &[LineItem],
) -> Result<Uuid, AppError> {
    let amount = if recurring.is_empty() {
        original.amount
    } else {
        Some(recurring.iter().map(|item| item.total).sum())
    };
    let owner = match settings.owner_mode {
        RenewalOwnerMode::Original => original
            .owner_id
            .as_ref()
            .map(|owner| owner.as_str().to_owned()),
        RenewalOwnerMode::Unassigned => None,
    };
    let input = |owner: Option<String>| RenewalDealCreation {
        title: renewal_title(
            &settings.title_prefix,
            &original.title,
            MAX_DEAL_TITLE_LENGTH,
        ),
        amount,
        currency: original.currency.clone(),
        pipeline_id: settings.target_pipeline_id,
        stage_id: settings.target_stage_id,
        expected_close_date: original.contract_end,
        contact_id: original.contact_id,
        company_id: original.company_id,
        owner_id: owner,
        source: original.source.clone(),
        contract_start: original.contract_end.and_then(|end| end.succ_opt()),
        original_id: original.id,
    };
    let writer = ctx.services().require::<dyn RenewalDealWriter>()?;
    match writer.create(ctx, input(owner.clone())).await {
        Err(AppError::InvalidFields { .. }) if owner.is_some() => {
            writer.create(ctx, input(None)).await
        }
        other => other,
    }
}

async fn copy_items(
    ctx: &Ctx<'_>,
    renewal_id: Uuid,
    recurring: &[LineItem],
) -> Result<(), AppError> {
    if recurring.is_empty() || !items_enabled(ctx) {
        return Ok(());
    }
    let record =
        aphel_kernel::application::shared::RecordRef::parse("deals.deal", &renewal_id.to_string())?;
    let port = ctx
        .services()
        .require::<dyn aphel_sdk::line_items::LineItemsPort>()?;
    if !port.rows(ctx, &record).await?.is_empty() {
        return Ok(());
    }
    port.replace(
        ctx,
        &record,
        recurring.iter().map(|item| item.data.clone()).collect(),
    )
    .await?;
    Ok(())
}

async fn finish(
    ctx: &Ctx<'_>,
    original: &RenewalSource,
    renewal_id: Uuid,
) -> Result<bool, AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let created = RenewalStore::of(ctx)
        .mark_renewed(&mut tx, original.id, renewal_id, now)
        .await?;
    if !created {
        drop(tx);
        return Ok(false);
    }
    let record = RecordRef::new(RecordType::DEAL, renewal_id);
    let payload = json!({
        "original_deal_id": original.id.to_string(),
        "original_title": original.title,
        "renewal_deal_id": renewal_id.to_string(),
        "contract_end": original.contract_end.map(date),
        "owner_id": original.owner_id.as_ref().map(|owner| owner.as_str()),
        "requested_by": ctx.scope.user_id().as_str(),
    });
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(Actor::System, DEAL_RENEWAL_CREATED, now)
                .with_record(record.clone())
                .with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::System, DEAL_RENEWAL_CREATED, now).with_record(record),
        )
        .await?;
    tx.commit().await?;
    Ok(true)
}
