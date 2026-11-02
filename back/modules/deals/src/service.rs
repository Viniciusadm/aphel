use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::KEY;
use crate::domain::{
    AMOUNT_DECIMALS, Board, BoardColumn, BoardQuery, DEAL_AMOUNT_CHANGED,
    DEAL_AMOUNT_MANAGED_BY_ITEMS, DEAL_ARCHIVED, DEAL_CREATED, DEAL_DELETED,
    DEAL_EXPECTED_CLOSE_DATE_CHANGED, DEAL_LOST, DEAL_OWNER_CHANGED, DEAL_PIPELINE_CHANGED,
    DEAL_REOPENED, DEAL_STAGE_CHANGED, DEAL_UNARCHIVED, DEAL_UPDATED, DEAL_WON, Deal, DealInput,
    DealRecord, DealRevenue, DealStatus, DealTransitionError, DealView, LoseDealCommand,
    LostDetails, MAX_BOARD_PER_COLUMN, MAX_DEAL_SOURCE_LENGTH, MAX_DEAL_TITLE_LENGTH,
    MAX_LOST_REASON_NOTE_LENGTH, MoveDealCommand, MoveDealPipelineCommand, NO_DEFAULT_PIPELINE,
    ReopenDealCommand, StageTransition, amount_json, apply_contract_input, date, deal_not_found,
    deal_snapshot, instant, needs_contract_refresh, normalize_amount, normalize_currency,
    normalize_optional_text, normalize_title, revenue_changes, transition_violation,
};
use crate::hooks::{
    AmountCalculator, ClosingDeal, DealLosing, DealLost, DealStageChanging, DealWinning, DealWon,
    StageChange,
};
use crate::store::DealStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::EntityDefinition;
use aphel_kernel::application::fields::permissions::check_record_writes;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, FieldSet, RecordDraft, RecordPipeline,
    RecordWrite, WriteMode,
};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    FieldTarget, Filter, FilterOperator, PlanCondition, PlanNode, PlanValue, Predicate, QueryPlan,
    RecordQuery, RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts, ResolveContext,
    SegmentFilters, SegmentResolution, resolve_query_with,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{
    InputValidator, PendingCommit, changes_payload, contributed_input, is_writable_contributed,
    load_contributed, merge_contributed, with_contributed, write_contributed,
};
use currencies::{
    CurrencyConverter,
    domain::{base_amount, currency_not_enabled},
};
use pipelines::domain::{
    LostReason, PIPELINE_NOT_FOUND, Pipeline, PipelineStage, StageKind, first_active_of_kind,
    parse_id, pipeline_ref, pipeline_visible, sees_every_pipeline, stage_ref, valid_probability,
};
use pipelines::store::PipelineStore;

const STAGE_FIELDS: [&str; 7] = [
    "pipeline_id",
    "stage_id",
    "status",
    "closed_at",
    "lost_reason_id",
    "lost_reason_note",
    "archived_at",
];

fn owner_json(owner: Option<(&UserId, &str)>) -> Value {
    match owner {
        Some((id, name)) => json!({ "id": id.as_str(), "display_name": name }),
        None => Value::Null,
    }
}

fn parse_reference(value: &str, field: &str, violations: &mut Vec<FieldViolation>) -> Option<Uuid> {
    let id = parse_id(value);
    if id.is_none() {
        violations.push(FieldViolation::new(field, "must be a valid id"));
    }
    id
}

fn check(message: &str, violations: Vec<FieldViolation>) -> Result<(), AppError> {
    if violations.is_empty() {
        Ok(())
    } else {
        Err(AppError::invalid_fields(message, violations))
    }
}

fn calculator(ctx: &Ctx<'_>) -> Result<std::sync::Arc<dyn AmountCalculator>, AppError> {
    ctx.services().require::<dyn AmountCalculator>()
}

struct StageTarget {
    stage: PipelineStage,
    lost: Option<LostDetails>,
    lost_reason: Option<LostReason>,
}

struct Planner {
    fields: FieldSet,
    context: ResolveContext,
    visibility: RecordScopeFilter,
    segments: SegmentFilters,
}

impl Planner {
    fn plan(&self, query: RecordQuery) -> Result<QueryPlan, AppError> {
        resolve_query_with(
            &self.fields,
            &self.context,
            self.visibility.clone(),
            query,
            SegmentResolution::Expand(&self.segments),
        )
    }
}

pub(crate) fn pipeline_condition(pipeline_ids: &[Uuid]) -> PlanNode {
    let values = if pipeline_ids.is_empty() {
        vec![PlanValue::Text(Uuid::nil().to_string())]
    } else {
        pipeline_ids
            .iter()
            .map(|id| PlanValue::Text(id.to_string()))
            .collect()
    };
    PlanNode::Condition(PlanCondition {
        target: FieldTarget::System {
            column: "pipeline_id".to_owned(),
        },
        field_type: FieldType::Record,
        predicate: Predicate::In(values),
        negated: false,
    })
}

pub(crate) fn restrict_to_pipelines(plan: &mut QueryPlan, pipeline_ids: &[Uuid]) {
    let condition = pipeline_condition(pipeline_ids);
    plan.filter = Some(match plan.filter.take() {
        None => condition,
        Some(PlanNode::And(mut nodes)) => {
            nodes.push(condition);
            PlanNode::And(nodes)
        }
        Some(other) => PlanNode::And(vec![other, condition]),
    });
}

struct Outcome {
    action: &'static str,
    events: Vec<(&'static str, Value)>,
    changes: Vec<FieldChange>,
    ip: Option<String>,
}

pub async fn list(ctx: &Ctx<'_>, query: RecordQuery) -> Result<Listing<DealView>, AppError> {
    let scope = ctx.scope;
    let planner = planner(ctx, query.filter.as_ref()).await?;
    let plan = planner.plan(query)?;
    let listing = ctx.query().executor().execute(scope, &plan).await?;
    Ok(Listing {
        items: views(ctx, &listing.items).await?,
        total: listing.total,
    })
}

pub async fn get(ctx: &Ctx<'_>, deal_id: &str) -> Result<DealView, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::DEALS_READ)?;
    let record = require(ctx, &visibility, deal_id).await?;
    view(ctx, record).await
}

pub async fn create(ctx: &Ctx<'_>, input: DealInput) -> Result<DealView, AppError> {
    create_with_revenue(ctx, input, DealRevenue::default()).await
}

pub async fn create_with_revenue(
    ctx: &Ctx<'_>,
    input: DealInput,
    revenue: DealRevenue,
) -> Result<DealView, AppError> {
    let mut tx = ctx.begin().await?;
    let pending = create_in_tx(ctx, &mut tx, input, revenue).await?;
    tx.commit().await?;
    pending.committed(ctx).await;
    get(ctx, &pending.record.record_id.to_string()).await
}

pub(crate) async fn create_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    input: DealInput,
    revenue: DealRevenue,
) -> Result<PendingCommit, AppError> {
    create_record(ctx, tx, input, revenue, Uuid::now_v7(), false).await
}

async fn create_record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    input: DealInput,
    revenue: DealRevenue,
    record_id: Uuid,
    filtered: bool,
) -> Result<PendingCommit, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::DEALS_CREATE)?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::DEAL.as_str())?;
    let mut input = input;
    split_contributed(definition, &mut input);
    let now = ctx.now();
    let mut violations = Vec::new();
    let mut validator = InputValidator::new();
    let contributed =
        merge_contributed(definition, &Map::new(), &input.contributed, &mut validator);
    violations.extend(validator.into_violations());
    let title = match input.title.as_deref() {
        Some(title) => normalize_title(title),
        None => None,
    };
    if title.is_none() {
        violations.push(FieldViolation::new(
            "title",
            format!("must have between 1 and {MAX_DEAL_TITLE_LENGTH} characters"),
        ));
    }
    let currency = match input.currency.as_deref() {
        Some(currency) => currency.to_owned(),
        None => DealStore::of(ctx).organization_currency().await?,
    };
    let mut deal = Deal {
        id: record_id,
        owner_id: Some(scope.user_id().clone()),
        pipeline_id: Uuid::nil(),
        stage_id: Uuid::nil(),
        title: title.unwrap_or_default(),
        amount: None,
        currency,
        revenue: revenue.clone(),
        probability: None,
        expected_close_date: None,
        status: DealStatus::Open,
        closed_at: None,
        lost_reason_id: None,
        lost_reason_note: None,
        source: None,
        contact_id: None,
        company_id: None,
        stage_entered_at: now,
        last_activity_at: None,
        created_by: Some(scope.user_id().clone()),
        archived_at: None,
        created_at: now,
        updated_at: now,
    };
    if let Some(currency) = input.currency.as_deref() {
        apply_currency(&mut deal, currency, &mut violations);
    }
    apply_fields(ctx, tx, &mut deal, &input, &mut violations).await?;
    let target = resolve_target(ctx, None, &deal, &input, &mut violations).await?;
    check("invalid deal", violations)?;
    check_field_access(ctx, &input, &Value::Null, &deal_values(&deal, &contributed)).await?;
    let Some(target) = target else {
        return Err(AppError::Internal(
            "a new deal always resolves a stage".to_owned(),
        ));
    };
    let status = DealStatus::for_stage(target.stage.kind);
    if status == DealStatus::Lost && target.lost.is_none() {
        return Err(AppError::invalid_fields(
            "invalid deal",
            vec![transition_violation(
                DealTransitionError::LostReasonRequired,
            )],
        ));
    }
    deal.pipeline_id = target.stage.pipeline_id;
    deal.stage_id = target.stage.id;
    deal.status = status;
    if status != DealStatus::Open {
        deal.closed_at = Some(now);
    }
    if let Some(lost) = target.lost.clone() {
        deal.lost_reason_id = Some(lost.reason_id);
        deal.lost_reason_note = lost.note;
    }
    convert_deal(ctx, None, &mut deal).await?;

    let record = RecordRef::new(RecordType::DEAL, deal.id);
    if !filtered
        && let Some(input) = saving(
            ctx,
            tx,
            &record,
            WriteMode::Create,
            (&deal, &contributed),
            &input,
        )
        .await?
    {
        return Box::pin(create_record(ctx, tx, input, revenue, record_id, true)).await;
    }
    let moving = Moving {
        previous: None,
        deal: &deal,
        target: &target,
        stage_changed: true,
        status_changed: deal.status != DealStatus::Open,
    };
    before_moving(ctx, tx, &moving).await?;
    DealStore::of(ctx).insert(tx, &deal).await?;
    write_contributed(tx, definition, deal.id, &contributed).await?;
    after_moving(ctx, tx, &moving).await?;
    let values = deal_values(&deal, &contributed);
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(tx, scope, &record, &input.custom_fields, WriteMode::Create)
        .await?;
    let mut changes = diff_values(&Value::Null, &values, &DiffOptions::default())?;
    changes.extend(custom);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Create,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(tx, scope, &write).await?;
    pipeline.saved(tx, scope, &write).await?;
    let mut events = vec![(
        DEAL_CREATED,
        json!({
            "title": deal.title,
            "amount": amount_json(deal.amount),
            "currency": deal.currency,
            "stage": stage_ref(&target.stage),
        }),
    )];
    events.extend(closing_events(&deal, &target, now));
    finish(
        ctx,
        tx,
        &record,
        Outcome {
            action: DEAL_CREATED,
            events,
            changes: changes.clone(),
            ip: input.ip,
        },
    )
    .await?;
    Ok(PendingCommit {
        record,
        mode: WriteMode::Create,
        changes,
        values,
    })
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    deal_id: &str,
    input: DealInput,
) -> Result<DealView, AppError> {
    commit_update(ctx, deal_id, input, false).await
}

pub(crate) async fn update_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    deal_id: &str,
    input: DealInput,
) -> Result<Option<PendingCommit>, AppError> {
    update_record(ctx, tx, deal_id, input, false).await
}

async fn commit_update(
    ctx: &Ctx<'_>,
    deal_id: &str,
    input: DealInput,
    filtered: bool,
) -> Result<DealView, AppError> {
    let mut tx = ctx.begin().await?;
    match update_record(ctx, &mut tx, deal_id, input, filtered).await? {
        Some(pending) => {
            tx.commit().await?;
            pending.committed(ctx).await;
        }
        None => drop(tx),
    }
    get(ctx, deal_id).await
}

async fn update_record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    deal_id: &str,
    input: DealInput,
    filtered: bool,
) -> Result<Option<PendingCommit>, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::DEALS_UPDATE)?;
    let current = require(ctx, &visibility, deal_id).await?;
    ensure_version(input.expected_updated_at, current.deal.updated_at)?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::DEAL.as_str())?;
    let mut input = input;
    split_contributed(definition, &mut input);
    let old_contributed = contributed_input(
        definition,
        &load_contributed(ctx.pool(), definition, &[current.deal.id])
            .await?
            .remove(&current.deal.id)
            .unwrap_or_default(),
    );
    let now = ctx.now();
    let mut deal = current.deal.clone();
    let mut violations = Vec::new();
    let mut validator = InputValidator::new();
    let contributed = merge_contributed(
        definition,
        &old_contributed,
        &input.contributed,
        &mut validator,
    );
    violations.extend(validator.into_violations());
    if let Some(title) = input.title.as_deref() {
        match normalize_title(title) {
            Some(title) => deal.title = title,
            None => violations.push(FieldViolation::new(
                "title",
                format!("must have between 1 and {MAX_DEAL_TITLE_LENGTH} characters"),
            )),
        }
    }
    if let Some(currency) = input.currency.as_deref() {
        apply_currency(&mut deal, currency, &mut violations);
    }
    apply_fields(ctx, tx, &mut deal, &input, &mut violations).await?;
    if deal.amount != current.deal.amount && calculator(ctx)?.amount_locked(ctx, deal.id).await? {
        return Err(AppError::invalid_fields_code(
            DEAL_AMOUNT_MANAGED_BY_ITEMS,
            "the amount of this deal is the sum of its items",
            vec![FieldViolation::new(
                "amount",
                "is calculated from the items of the deal",
            )],
        ));
    }
    let target = resolve_target(ctx, Some(&current), &deal, &input, &mut violations).await?;
    let transition = match &target {
        Some(target) => match deal.enter_stage(&target.stage, target.lost.clone(), now) {
            Ok(transition) => Some(transition),
            Err(error) => {
                violations.push(transition_violation(error));
                None
            }
        },
        None => None,
    };
    check("invalid deal", violations)?;
    check_field_access(
        ctx,
        &input,
        &deal_values(&current.deal, &old_contributed),
        &deal_values(&deal, &contributed),
    )
    .await?;
    convert_deal(ctx, Some(&current.deal), &mut deal).await?;
    refresh_contract(ctx, &current.deal, &mut deal, input.contract_end.is_some()).await?;
    deal.updated_at = now;

    let record = RecordRef::new(RecordType::DEAL, deal.id);
    let values = deal_values(&deal, &contributed);
    let mut changes = diff_values(
        &deal_values(&current.deal, &old_contributed),
        &values,
        &DiffOptions::default(),
    )?;
    aphel_kernel::records::lock_version(tx, "deals", current.deal.id, input.expected_updated_at)
        .await?;
    if !filtered
        && let Some(input) = saving(
            ctx,
            tx,
            &record,
            WriteMode::Update,
            (&deal, &contributed),
            &input,
        )
        .await?
    {
        return Box::pin(update_record(ctx, tx, deal_id, input, true)).await;
    }
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(tx, scope, &record, &input.custom_fields, WriteMode::Update)
        .await?;
    changes.extend(custom);
    if changes.is_empty() {
        return Ok(None);
    }
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(tx, scope, &write).await?;
    let moving = match (&target, &transition) {
        (Some(target), Some(transition)) => Some(Moving {
            previous: Some(&current),
            deal: &deal,
            target,
            stage_changed: transition.stage_changed,
            status_changed: transition.status_changed(),
        }),
        _ => None,
    };
    if let Some(moving) = &moving {
        before_moving(ctx, tx, moving).await?;
    }
    DealStore::of(ctx).update(tx, &deal).await?;
    if contributed != old_contributed {
        write_contributed(tx, definition, deal.id, &contributed).await?;
    }
    sync_converted_items(ctx, tx, &current.deal, &deal).await?;
    if let Some(moving) = &moving {
        after_moving(ctx, tx, moving).await?;
    }
    pipeline.saved(tx, scope, &write).await?;

    let mut events = Vec::new();
    let specific = field_events(ctx, &current, &deal, &changes).await?;
    let general: Vec<&FieldChange> = changes
        .iter()
        .filter(|change| {
            !STAGE_FIELDS.contains(&change.field.as_str())
                && !specific.covers(change.field.as_str())
        })
        .collect();
    if !general.is_empty() {
        events.push((
            DEAL_UPDATED,
            json!({
                "changes": general
                    .iter()
                    .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
                    .collect::<Vec<_>>(),
            }),
        ));
    }
    events.extend(specific.events);
    if let (Some(target), Some(transition)) = (&target, &transition) {
        events.extend(transition_events(&current, &deal, target, transition, now));
    }
    events.extend(pipeline_event(ctx, &current, &deal).await?);
    finish(
        ctx,
        tx,
        &record,
        Outcome {
            action: DEAL_UPDATED,
            events,
            changes: changes.clone(),
            ip: input.ip,
        },
    )
    .await?;
    Ok(Some(PendingCommit {
        record,
        mode: WriteMode::Update,
        changes,
        values,
    }))
}

pub async fn move_to_stage(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: MoveDealCommand,
) -> Result<DealView, AppError> {
    let input = DealInput {
        stage_id: Some(command.stage_id),
        lost_reason_id: command.lost_reason_id.map(Some),
        lost_reason_note: command.lost_reason_note.map(Some),
        ip: command.ip,
        ..DealInput::default()
    };
    if command.fields.is_empty() {
        return transition(ctx, deal_id, input).await;
    }
    update(ctx, deal_id, input.with_field_values(&command.fields)).await
}

pub(crate) async fn move_to_pipeline(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: MoveDealPipelineCommand,
) -> Result<DealView, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::DEALS_UPDATE)?;
    let current = require(ctx, &visibility, deal_id).await?;
    let pipeline = match parse_id(&command.pipeline_id) {
        Some(id) => visible_pipeline(ctx, id).await?,
        None => None,
    };
    let Some(pipeline) = pipeline else {
        return Err(AppError::invalid_fields(
            "invalid deal",
            vec![FieldViolation::new("pipeline_id", "pipeline was not found")],
        ));
    };
    let stage_id = match command
        .stage_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(stage_id) => stage_id.to_owned(),
        None => {
            let stages = PipelineStore::of(ctx).list_stages(pipeline.id).await?;
            match first_active_of_kind(&stages, current.stage.kind) {
                Some(stage) => stage.id.to_string(),
                None => {
                    return Err(AppError::invalid_fields(
                        "invalid deal",
                        vec![FieldViolation::new(
                            "stage_id",
                            "the pipeline has no active stage of this kind",
                        )],
                    ));
                }
            }
        }
    };
    let input = DealInput {
        pipeline_id: Some(pipeline.id.to_string()),
        stage_id: Some(stage_id),
        lost_reason_id: command.lost_reason_id.map(Some),
        lost_reason_note: command.lost_reason_note.map(Some),
        ip: command.ip,
        ..DealInput::default()
    };
    if command.fields.is_empty() {
        return transition(ctx, deal_id, input).await;
    }
    update(ctx, deal_id, input.with_field_values(&command.fields)).await
}

pub async fn win(ctx: &Ctx<'_>, deal_id: &str, ip: Option<String>) -> Result<DealView, AppError> {
    let input = DealInput {
        status: Some(DealStatus::Won.as_str().to_owned()),
        ip,
        ..DealInput::default()
    };
    transition(ctx, deal_id, input).await
}

pub async fn lose(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: LoseDealCommand,
) -> Result<DealView, AppError> {
    let input = DealInput {
        status: Some(DealStatus::Lost.as_str().to_owned()),
        lost_reason_id: Some(command.lost_reason_id),
        lost_reason_note: command.lost_reason_note.map(Some),
        ip: command.ip,
        ..DealInput::default()
    };
    transition(ctx, deal_id, input).await
}

pub(crate) async fn reopen(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: ReopenDealCommand,
) -> Result<DealView, AppError> {
    let input = DealInput {
        status: Some(DealStatus::Open.as_str().to_owned()),
        stage_id: command.stage_id,
        ip: command.ip,
        ..DealInput::default()
    };
    transition(ctx, deal_id, input).await
}

pub(crate) async fn archive(
    ctx: &Ctx<'_>,
    deal_id: &str,
    ip: Option<String>,
) -> Result<DealView, AppError> {
    set_archived(ctx, deal_id, true, ip).await
}

pub(crate) async fn unarchive(
    ctx: &Ctx<'_>,
    deal_id: &str,
    ip: Option<String>,
) -> Result<DealView, AppError> {
    set_archived(ctx, deal_id, false, ip).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    deal_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::DEALS_DELETE)?;
    let current = require(ctx, &visibility, deal_id).await?;
    let now = ctx.now();
    let record = RecordRef::new(RecordType::DEAL, current.deal.id);
    let pipeline = ctx.pipeline();
    let mut tx = ctx.begin().await?;
    pipeline.deleting(&mut tx, scope, &record).await?;
    DealStore::of(ctx)
        .soft_delete(&mut tx, current.deal.id, now)
        .await?;
    pipeline.deleted(&mut tx, scope, &record).await?;
    finish(
        ctx,
        &mut tx,
        &record,
        Outcome {
            action: DEAL_DELETED,
            events: vec![(DEAL_DELETED, json!({ "title": current.deal.title }))],
            changes: diff_values(
                &deal_snapshot(&current.deal),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn board(ctx: &Ctx<'_>, query: BoardQuery) -> Result<Board, AppError> {
    let scope = ctx.scope;
    let planner = planner(ctx, query.query.filter.as_ref()).await?;
    let pipeline = board_pipeline(ctx, query.pipeline_id.as_deref()).await?;
    let mut stages: Vec<PipelineStage> = PipelineStore::of(ctx)
        .list_stages(pipeline.id)
        .await?
        .into_iter()
        .filter(|stage| stage.is_active)
        .collect();
    if let Some(stage_id) = query.stage_id.as_deref() {
        let id = parse_id(stage_id);
        stages.retain(|stage| Some(stage.id) == id);
        if stages.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid board query",
                vec![FieldViolation::new(
                    "stage_id",
                    "must be an active stage of the pipeline",
                )],
            ));
        }
    }
    let per_column = query.per_column.clamp(1, MAX_BOARD_PER_COLUMN);
    let page = query.page.max(1);
    let base = |extra: Option<Filter>| {
        let mut conditions: Vec<Filter> = query.query.filter.iter().cloned().collect();
        conditions.push(Filter::condition(
            "pipeline_id",
            FilterOperator::Eq,
            json!(pipeline.id.to_string()),
        ));
        conditions.extend(extra);
        Filter::and(conditions)
    };
    let totals_plan = planner.plan(RecordQuery {
        filter: Some(base(None)),
        sort: Vec::new(),
        search: query.query.search.clone(),
        archived: query.query.archived,
        window: ListWindow {
            offset: 0,
            limit: 1,
        },
    })?;
    let totals: HashMap<Uuid, Decimal> = DealStore::of(ctx)
        .stage_totals(scope, &totals_plan)
        .await?
        .into_iter()
        .map(|total| (total.stage_id, total.total_amount))
        .collect();

    let mut listings = Vec::with_capacity(stages.len());
    for stage in &stages {
        let plan = planner.plan(RecordQuery {
            filter: Some(base(Some(Filter::condition(
                "stage_id",
                FilterOperator::Eq,
                json!(stage.id.to_string()),
            )))),
            sort: query.query.sort.clone(),
            search: query.query.search.clone(),
            archived: query.query.archived,
            window: ListWindow {
                offset: (page - 1) * per_column,
                limit: per_column,
            },
        })?;
        listings.push(ctx.query().executor().execute(scope, &plan).await?);
    }
    let ids: Vec<Uuid> = listings
        .iter()
        .flat_map(|listing| listing.items.iter().copied())
        .collect();
    let mut views: HashMap<Uuid, DealView> = views(ctx, &ids)
        .await?
        .into_iter()
        .map(|view| (view.record.deal.id, view))
        .collect();
    let columns = stages
        .into_iter()
        .zip(listings)
        .map(|(stage, listing)| BoardColumn {
            total_amount: totals.get(&stage.id).copied().unwrap_or_default(),
            count: listing.total,
            items: listing
                .items
                .iter()
                .filter_map(|id| views.remove(id))
                .collect(),
            stage,
        })
        .collect();
    Ok(Board {
        pipeline,
        columns,
        page,
        per_column,
    })
}

async fn check_field_access(
    ctx: &Ctx<'_>,
    input: &DealInput,
    old: &Value,
    new: &Value,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let mut provided = input.provided_keys();
    provided.extend(input.contributed.keys().map(String::as_str));
    check_record_writes(
        ctx.fields().definitions(),
        scope,
        &RecordType::DEAL,
        &provided,
        old,
        new,
    )
    .await
}

async fn transition(ctx: &Ctx<'_>, deal_id: &str, input: DealInput) -> Result<DealView, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::DEALS_UPDATE)?;
    let current = require(ctx, &visibility, deal_id).await?;
    let now = ctx.now();
    let mut deal = current.deal.clone();
    let mut violations = Vec::new();
    let target = resolve_target(ctx, Some(&current), &deal, &input, &mut violations).await?;
    check("invalid deal", violations)?;
    let Some(target) = target else {
        return view(ctx, current).await;
    };
    let transition = deal
        .enter_stage(&target.stage, target.lost.clone(), now)
        .map_err(|error| {
            AppError::invalid_fields("invalid deal", vec![transition_violation(error)])
        })?;
    check_field_access(
        ctx,
        &input,
        &deal_snapshot(&current.deal),
        &deal_snapshot(&deal),
    )
    .await?;
    convert_deal(ctx, Some(&current.deal), &mut deal).await?;
    refresh_contract(ctx, &current.deal, &mut deal, false).await?;
    let changes = diff_values(
        &deal_snapshot(&current.deal),
        &deal_snapshot(&deal),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return view(ctx, current).await;
    }
    let mut events = transition_events(&current, &deal, &target, &transition, now);
    events.extend(pipeline_event(ctx, &current, &deal).await?);
    let revenue = revenue_changes(&current.deal.revenue, &deal.revenue);
    if !revenue.is_empty() {
        events.push((DEAL_UPDATED, changes_payload(&revenue)));
    }
    let action = match (transition.status_changed(), deal.status) {
        (true, DealStatus::Won) => DEAL_WON,
        (true, DealStatus::Lost) => DEAL_LOST,
        (true, DealStatus::Open) => DEAL_REOPENED,
        (false, _) if transition.stage_changed => DEAL_STAGE_CHANGED,
        _ => DEAL_UPDATED,
    };
    if events.is_empty() {
        events.push((
            DEAL_UPDATED,
            json!({
                "changes": changes
                    .iter()
                    .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
                    .collect::<Vec<_>>(),
            }),
        ));
    }
    let record = RecordRef::new(RecordType::DEAL, deal.id);
    let mut tx = ctx.begin().await?;
    if let Some(input) = saving(
        ctx,
        &mut tx,
        &record,
        WriteMode::Update,
        (&deal, &Map::new()),
        &input,
    )
    .await?
    {
        drop(tx);
        return Box::pin(commit_update(ctx, deal_id, input, true)).await;
    }
    let values = deal_snapshot(&deal);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(&mut tx, scope, &write).await?;
    let moving = Moving {
        previous: Some(&current),
        deal: &deal,
        target: &target,
        stage_changed: transition.stage_changed,
        status_changed: transition.status_changed(),
    };
    before_moving(ctx, &mut tx, &moving).await?;
    DealStore::of(ctx).update(&mut tx, &deal).await?;
    sync_converted_items(ctx, &mut tx, &current.deal, &deal).await?;
    after_moving(ctx, &mut tx, &moving).await?;
    pipeline.saved(&mut tx, scope, &write).await?;
    finish(
        ctx,
        &mut tx,
        &record,
        Outcome {
            action,
            events,
            changes: changes.clone(),
            ip: input.ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(scope, &write).await;
    get(ctx, deal_id).await
}

async fn saving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    mode: WriteMode,
    (deal, contributed): (&Deal, &Map<String, Value>),
    input: &DealInput,
) -> Result<Option<DealInput>, AppError> {
    let scope = ctx.scope;
    let values = deal_values(deal, contributed);
    let draft = ctx
        .pipeline()
        .saving(
            tx,
            scope,
            RecordDraft::new(record.clone(), mode, &values, input.custom_fields.clone()),
        )
        .await?;
    let changed = draft.changed_values(&values);
    if changed.is_empty() && draft.custom_fields == input.custom_fields {
        return Ok(None);
    }
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::DEAL.as_str())?;
    let mut input = DealInput {
        custom_fields: draft.custom_fields,
        ..input.clone()
    };
    for (key, value) in &changed {
        if is_writable_contributed(definition, key) {
            input.contributed.insert(key.clone(), value.clone());
        }
    }
    Ok(Some(input.with_record_values(&changed)))
}

fn deal_values(deal: &Deal, contributed: &Map<String, Value>) -> Value {
    with_contributed(deal_snapshot(deal), contributed)
}

fn split_contributed(definition: &EntityDefinition, input: &mut DealInput) {
    let moved: Vec<String> = input
        .custom_fields
        .keys()
        .filter(|key| is_writable_contributed(definition, key))
        .cloned()
        .collect();
    for key in moved {
        if let Some(value) = input.custom_fields.remove(&key) {
            input.contributed.insert(key, value);
        }
    }
    input.contributed = contributed_input(definition, &input.contributed);
}

async fn set_archived(
    ctx: &Ctx<'_>,
    deal_id: &str,
    archived: bool,
    ip: Option<String>,
) -> Result<DealView, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::DEALS_ARCHIVE)?;
    let current = require(ctx, &visibility, deal_id).await?;
    if current.deal.archived_at.is_some() == archived {
        return view(ctx, current).await;
    }
    let now = ctx.now();
    let mut deal = current.deal.clone();
    deal.archived_at = archived.then_some(now);
    deal.updated_at = now;
    let action = if archived {
        DEAL_ARCHIVED
    } else {
        DEAL_UNARCHIVED
    };
    let record = RecordRef::new(RecordType::DEAL, deal.id);
    let mut tx = ctx.begin().await?;
    DealStore::of(ctx).update(&mut tx, &deal).await?;
    finish(
        ctx,
        &mut tx,
        &record,
        Outcome {
            action,
            events: vec![(action, json!({ "title": deal.title }))],
            changes: diff_values(
                &deal_snapshot(&current.deal),
                &deal_snapshot(&deal),
                &DiffOptions::default(),
            )?,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    get(ctx, deal_id).await
}

fn apply_currency(deal: &mut Deal, currency: &str, violations: &mut Vec<FieldViolation>) {
    match normalize_currency(currency) {
        Some(currency) => deal.currency = currency,
        None => violations.push(FieldViolation::new(
            "currency",
            "must be an ISO 4217 code with three letters",
        )),
    }
}

async fn apply_fields(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    deal: &mut Deal,
    input: &DealInput,
    violations: &mut Vec<FieldViolation>,
) -> Result<(), AppError> {
    if let Some(amount) = &input.amount {
        match amount.as_deref().map(str::trim) {
            None | Some("") => deal.amount = None,
            Some(text) => match Decimal::from_str(text)
                .ok()
                .or_else(|| Decimal::from_scientific(text).ok())
                .and_then(normalize_amount)
            {
                Some(amount) => deal.amount = Some(amount),
                None => violations.push(FieldViolation::new(
                    "amount",
                    format!(
                        "must be a non-negative number with at most {AMOUNT_DECIMALS} decimal places"
                    ),
                )),
            },
        }
    }
    if let Some(probability) = input.probability {
        match probability {
            None => deal.probability = None,
            Some(value) => match valid_probability(value) {
                Some(probability) => deal.probability = Some(probability),
                None => violations.push(FieldViolation::new(
                    "probability",
                    "must be an integer between 0 and 100",
                )),
            },
        }
    }
    if let Some(expected) = &input.expected_close_date {
        match expected.as_deref().map(str::trim) {
            None | Some("") => deal.expected_close_date = None,
            Some(text) => match NaiveDate::parse_from_str(text, "%Y-%m-%d") {
                Ok(value) => deal.expected_close_date = Some(value),
                Err(_) => violations.push(FieldViolation::new(
                    "expected_close_date",
                    "must be a date in the YYYY-MM-DD format",
                )),
            },
        }
    }
    if let Some(source) = &input.source {
        match normalize_optional_text(source.as_deref(), MAX_DEAL_SOURCE_LENGTH) {
            Ok(source) => deal.source = source,
            Err(_) => violations.push(FieldViolation::new(
                "source",
                format!("must have at most {MAX_DEAL_SOURCE_LENGTH} characters"),
            )),
        }
    }
    if let Some(contact) = &input.contact_id {
        deal.contact_id = match contact.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => match parse_reference(value, "contact_id", violations) {
                Some(id) if id == deal.contact_id.unwrap_or_default() => Some(id),
                Some(id) => {
                    if DealStore::of(ctx).contact_name(tx, id).await?.is_none() {
                        violations.push(FieldViolation::new("contact_id", "contact was not found"));
                    }
                    Some(id)
                }
                None => deal.contact_id,
            },
        };
    }
    if let Some(company) = &input.company_id {
        deal.company_id = match company.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => match parse_reference(value, "company_id", violations) {
                Some(id) if id == deal.company_id.unwrap_or_default() => Some(id),
                Some(id) => {
                    if DealStore::of(ctx).company_name(tx, id).await?.is_none() {
                        violations.push(FieldViolation::new("company_id", "company was not found"));
                    }
                    Some(id)
                }
                None => deal.company_id,
            },
        };
    }
    if let Some(owner) = &input.owner_id {
        deal.owner_id = match owner.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => match UserId::new(value) {
                Ok(id) if Some(&id) == deal.owner_id.as_ref() => Some(id),
                Ok(id) => {
                    if DealStore::of(ctx).member_name(&id).await?.is_none() {
                        violations.push(FieldViolation::new(
                            "owner_id",
                            "must be an active member of the organization",
                        ));
                    }
                    Some(id)
                }
                Err(_) => {
                    violations.push(FieldViolation::new("owner_id", "must be a valid id"));
                    deal.owner_id.clone()
                }
            },
        };
    }
    apply_contract_input(
        &mut deal.revenue,
        &input.contract_start,
        &input.contract_end,
        violations,
    );
    Ok(())
}

async fn refresh_contract(
    ctx: &Ctx<'_>,
    current: &Deal,
    deal: &mut Deal,
    contract_end_provided: bool,
) -> Result<(), AppError> {
    if !needs_contract_refresh(Some(current), deal, contract_end_provided) {
        return Ok(());
    }
    calculator(ctx)?.refresh_revenue(ctx, deal).await
}

async fn convert_deal(
    ctx: &Ctx<'_>,
    current: Option<&Deal>,
    deal: &mut Deal,
) -> Result<(), AppError> {
    let currency_changed = current.is_none_or(|current| current.currency != deal.currency);
    let frozen = !currency_changed
        && deal.status != DealStatus::Open
        && current.is_some_and(|current| current.status != DealStatus::Open)
        && deal.revenue.exchange_rate.is_some();
    if !frozen {
        let conversion = ctx
            .services()
            .require::<dyn CurrencyConverter>()?
            .conversion(ctx, &deal.currency)
            .await?;
        if currency_changed && !conversion.enabled {
            return Err(currency_not_enabled(&deal.currency));
        }
        deal.revenue.exchange_rate = conversion.rate;
    }
    deal.revenue.amount_base = base_amount(deal.amount, deal.revenue.exchange_rate);
    Ok(())
}

async fn sync_converted_items(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    current: &Deal,
    deal: &Deal,
) -> Result<(), AppError> {
    if current.revenue.exchange_rate != deal.revenue.exchange_rate {
        calculator(ctx)?
            .sync_rate(tx, deal.id, deal.revenue.exchange_rate)
            .await?;
    }
    Ok(())
}

struct Moving<'a> {
    previous: Option<&'a DealRecord>,
    deal: &'a Deal,
    target: &'a StageTarget,
    stage_changed: bool,
    status_changed: bool,
}

impl Moving<'_> {
    fn closing(&self) -> ClosingDeal {
        ClosingDeal {
            deal: self.deal.clone(),
            previous: self.previous.map(|record| record.deal.clone()),
            stage: self.target.stage.clone(),
            lost_reason: self.target.lost_reason.clone(),
        }
    }
}

async fn before_moving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    moving: &Moving<'_>,
) -> Result<(), AppError> {
    if moving.stage_changed {
        let change = StageChange {
            deal: moving.deal.clone(),
            previous: moving.previous.map(|record| record.deal.clone()),
            from: moving.previous.map(|record| record.stage.clone()),
            to: moving.target.stage.clone(),
        };
        ctx.run_hook(DealStageChanging, tx, KEY, &change).await?;
    }
    if moving.status_changed {
        match moving.deal.status {
            DealStatus::Won => {
                ctx.run_hook(DealWinning, tx, KEY, &moving.closing())
                    .await?
            }
            DealStatus::Lost => ctx.run_hook(DealLosing, tx, KEY, &moving.closing()).await?,
            DealStatus::Open => {}
        }
    }
    Ok(())
}

async fn after_moving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    moving: &Moving<'_>,
) -> Result<(), AppError> {
    if !moving.status_changed {
        return Ok(());
    }
    match moving.deal.status {
        DealStatus::Won => ctx.run_hook(DealWon, tx, KEY, &moving.closing()).await,
        DealStatus::Lost => ctx.run_hook(DealLost, tx, KEY, &moving.closing()).await,
        DealStatus::Open => Ok(()),
    }
}

async fn resolve_target(
    ctx: &Ctx<'_>,
    current: Option<&DealRecord>,
    deal: &Deal,
    input: &DealInput,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<StageTarget>, AppError> {
    let status = match input.status.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => {
            let status = DealStatus::parse(value);
            if status.is_none() {
                violations.push(FieldViolation::new("status", "must be open, won or lost"));
            }
            status
        }
    };
    let requested_pipeline = match input.pipeline_id.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => match parse_id(value) {
            Some(id) => match visible_pipeline(ctx, id).await? {
                Some(pipeline) => Some(pipeline),
                None => {
                    violations.push(FieldViolation::new("pipeline_id", "pipeline was not found"));
                    None
                }
            },
            None => {
                violations.push(FieldViolation::new("pipeline_id", "must be a valid id"));
                None
            }
        },
    };

    let stage = match input.stage_id.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => {
            let stage = match parse_id(value) {
                Some(id) => visible_stage(ctx, id).await?,
                None => None,
            };
            match stage {
                Some(stage) => Some(stage),
                None => {
                    violations.push(FieldViolation::new("stage_id", "stage was not found"));
                    return Ok(None);
                }
            }
        }
    };
    if let (Some(stage), Some(pipeline)) = (&stage, &requested_pipeline)
        && stage.pipeline_id != pipeline.id
    {
        violations.push(FieldViolation::new(
            "pipeline_id",
            "must be the pipeline of the stage",
        ));
    }
    if let (Some(stage), Some(status)) = (&stage, status)
        && DealStatus::for_stage(stage.kind) != status
    {
        violations.push(FieldViolation::new(
            "status",
            "does not match the kind of the stage",
        ));
    }

    let stage = match stage {
        Some(stage) => Some(stage),
        None => {
            let pipeline_changed = requested_pipeline.as_ref().is_some_and(|pipeline| {
                current.is_none_or(|record| record.deal.pipeline_id != pipeline.id)
            });
            let status_changed = status
                .is_some_and(|status| current.is_none_or(|record| record.deal.status != status));
            let lost_reason_changed = current.is_some_and(|record| {
                record.deal.status == DealStatus::Lost
                    && (input.lost_reason_id.is_some() || input.lost_reason_note.is_some())
            });
            if current.is_none() || pipeline_changed || status_changed {
                let pipeline = match requested_pipeline {
                    Some(pipeline) => Some(pipeline),
                    None => match current {
                        Some(record) => {
                            PipelineStore::of(ctx)
                                .find_pipeline(record.deal.pipeline_id)
                                .await?
                        }
                        None => default_visible_pipeline(ctx).await?,
                    },
                };
                let Some(pipeline) = pipeline else {
                    if violations.is_empty() {
                        return Err(AppError::conflict_code(
                            NO_DEFAULT_PIPELINE,
                            "the organization has no pipeline",
                        ));
                    }
                    return Ok(None);
                };
                let kind = match status {
                    Some(DealStatus::Won) => StageKind::Won,
                    Some(DealStatus::Lost) => StageKind::Lost,
                    Some(DealStatus::Open) => StageKind::Open,
                    None => current
                        .map(|record| record.stage.kind)
                        .unwrap_or(StageKind::Open),
                };
                let stages = PipelineStore::of(ctx).list_stages(pipeline.id).await?;
                match first_active_of_kind(&stages, kind) {
                    Some(stage) => Some(stage.clone()),
                    None => {
                        violations.push(FieldViolation::new(
                            "stage_id",
                            "the pipeline has no active stage of this kind",
                        ));
                        return Ok(None);
                    }
                }
            } else if lost_reason_changed {
                PipelineStore::of(ctx).find_stage(deal.stage_id).await?
            } else {
                None
            }
        }
    };
    let Some(stage) = stage else {
        if let Some(Some(_)) = &input.lost_reason_id {
            violations.push(FieldViolation::new(
                "lost_reason_id",
                "only lost deals have a lost reason",
            ));
        }
        return Ok(None);
    };

    let mut lost = None;
    let mut lost_reason = None;
    if stage.kind == StageKind::Lost {
        let reason_id = match &input.lost_reason_id {
            Some(Some(value)) => parse_id(value),
            Some(None) => None,
            None => deal.lost_reason_id,
        };
        let note = match &input.lost_reason_note {
            Some(note) => {
                match normalize_optional_text(note.as_deref(), MAX_LOST_REASON_NOTE_LENGTH) {
                    Ok(note) => note,
                    Err(_) => {
                        violations.push(FieldViolation::new(
                            "lost_reason_note",
                            format!("must have at most {MAX_LOST_REASON_NOTE_LENGTH} characters"),
                        ));
                        None
                    }
                }
            }
            None => deal.lost_reason_note.clone(),
        };
        match reason_id {
            Some(reason_id) => match PipelineStore::of(ctx).find_lost_reason(reason_id).await? {
                Some(reason) if reason.is_active || Some(reason.id) == deal.lost_reason_id => {
                    lost = Some(LostDetails {
                        reason_id: reason.id,
                        note,
                    });
                    lost_reason = Some(reason);
                }
                _ => violations.push(FieldViolation::new(
                    "lost_reason_id",
                    "must be an active lost reason",
                )),
            },
            None => violations.push(transition_violation(
                DealTransitionError::LostReasonRequired,
            )),
        }
    } else if let Some(Some(_)) = &input.lost_reason_id {
        violations.push(FieldViolation::new(
            "lost_reason_id",
            "only lost deals have a lost reason",
        ));
    }
    if !stage.is_active && Some(stage.id) != current.map(|record| record.deal.stage_id) {
        violations.push(transition_violation(DealTransitionError::StageInactive));
    }
    if Some(stage.pipeline_id) != current.map(|record| record.deal.pipeline_id) {
        let active = PipelineStore::of(ctx)
            .find_pipeline(stage.pipeline_id)
            .await?
            .is_some_and(|pipeline| pipeline.is_active);
        if !active {
            violations.push(FieldViolation::new("pipeline_id", "pipeline is not active"));
        }
    }
    Ok(Some(StageTarget {
        stage,
        lost,
        lost_reason,
    }))
}

fn closing_events(
    deal: &Deal,
    target: &StageTarget,
    now: DateTime<Utc>,
) -> Vec<(&'static str, Value)> {
    match deal.status {
        DealStatus::Won => vec![(
            DEAL_WON,
            json!({
                "amount": amount_json(deal.amount),
                "currency": deal.currency,
                "closed_at": instant(deal.closed_at.unwrap_or(now)),
            }),
        )],
        DealStatus::Lost => vec![(
            DEAL_LOST,
            json!({
                "lost_reason": target
                    .lost_reason
                    .as_ref()
                    .map(|reason| json!({ "id": reason.id.to_string(), "name": reason.name })),
                "note": deal.lost_reason_note,
                "closed_at": instant(deal.closed_at.unwrap_or(now)),
            }),
        )],
        DealStatus::Open => Vec::new(),
    }
}

fn transition_events(
    current: &DealRecord,
    deal: &Deal,
    target: &StageTarget,
    transition: &StageTransition,
    now: DateTime<Utc>,
) -> Vec<(&'static str, Value)> {
    let mut events = Vec::new();
    if transition.stage_changed {
        events.push((
            DEAL_STAGE_CHANGED,
            json!({
                "from": { "id": current.stage.id.to_string(), "name": current.stage.name },
                "to": stage_ref(&target.stage),
                "stage_entered_at": instant(deal.stage_entered_at),
            }),
        ));
    }
    let reason_changed = deal.status == DealStatus::Lost
        && (current.deal.lost_reason_id != deal.lost_reason_id
            || current.deal.lost_reason_note != deal.lost_reason_note);
    if transition.status_changed() || reason_changed {
        match deal.status {
            DealStatus::Open => {
                events.push((DEAL_REOPENED, json!({ "stage": stage_ref(&target.stage) })))
            }
            _ => events.extend(closing_events(deal, target, now)),
        }
    }
    events
}

struct SpecificEvents {
    events: Vec<(&'static str, Value)>,
    covered: Vec<&'static str>,
}

impl SpecificEvents {
    fn covers(&self, field: &str) -> bool {
        self.covered.contains(&field)
    }

    fn push(
        &mut self,
        event_type: &'static str,
        mut payload: Value,
        fields: &[&'static str],
        changes: &[FieldChange],
    ) {
        let covered: Vec<&FieldChange> = changes
            .iter()
            .filter(|change| fields.contains(&change.field.as_str()))
            .collect();
        payload["changes"] = Value::Array(
            covered
                .iter()
                .map(
                    |change| json!({ "field": change.field, "old": change.old, "new": change.new }),
                )
                .collect(),
        );
        self.covered.extend_from_slice(fields);
        self.events.push((event_type, payload));
    }
}

const AMOUNT_FIELDS: [&str; 4] = ["amount", "currency", "amount_base", "exchange_rate"];

async fn field_events(
    ctx: &Ctx<'_>,
    current: &DealRecord,
    deal: &Deal,
    changes: &[FieldChange],
) -> Result<SpecificEvents, AppError> {
    let mut events = SpecificEvents {
        events: Vec::new(),
        covered: Vec::new(),
    };
    if current.deal.amount != deal.amount || current.deal.currency != deal.currency {
        events.push(
            DEAL_AMOUNT_CHANGED,
            json!({
                "old": amount_json(current.deal.amount),
                "new": amount_json(deal.amount),
                "currency": deal.currency,
            }),
            &AMOUNT_FIELDS,
            changes,
        );
    }
    if current.deal.owner_id != deal.owner_id {
        let new_name = match &deal.owner_id {
            Some(id) => DealStore::of(ctx).member_name(id).await?,
            None => None,
        };
        events.push(
            DEAL_OWNER_CHANGED,
            json!({
                "old": owner_json(current.owner.as_ref().map(|owner| (&owner.id, owner.display_name.as_str()))),
                "new": owner_json(deal.owner_id.as_ref().zip(new_name.as_deref())),
            }),
            &["owner_id"],
            changes,
        );
    }
    if current.deal.expected_close_date != deal.expected_close_date {
        events.push(
            DEAL_EXPECTED_CLOSE_DATE_CHANGED,
            json!({
                "old": current.deal.expected_close_date.map(date),
                "new": deal.expected_close_date.map(date),
            }),
            &["expected_close_date"],
            changes,
        );
    }
    Ok(events)
}

async fn finish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    outcome: Outcome,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let actor = Actor::from_scope(scope);
    let now = ctx.now();
    for (event_type, payload) in outcome.events {
        let event = DomainEvent::new(actor.clone(), event_type, now)
            .with_record(record.clone())
            .with_payload(payload);
        ctx.events().publish(tx, event).await?;
    }
    let entry = AuditEntry::new(actor, outcome.action, now)
        .with_record(record.clone())
        .with_changes(outcome.changes)
        .with_ip(outcome.ip);
    ctx.audit().record(tx, entry).await
}

async fn board_pipeline(ctx: &Ctx<'_>, pipeline_id: Option<&str>) -> Result<Pipeline, AppError> {
    let pipeline = match pipeline_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => match parse_id(id) {
            Some(id) => visible_pipeline(ctx, id).await?,
            None => None,
        },
        None => default_visible_pipeline(ctx).await?,
    };
    pipeline.ok_or_else(|| AppError::not_found_code(PIPELINE_NOT_FOUND, "pipeline was not found"))
}

async fn planner(ctx: &Ctx<'_>, filter: Option<&Filter>) -> Result<Planner, AppError> {
    let scope = ctx.scope;
    let query = ctx.query();
    let definition = query.registry().require(RecordType::DEAL.as_str())?;
    let visibility = scope.record_filter(&definition.read_permission)?;
    let fields = FieldSet::for_scope(query.definitions(), definition, scope).await?;
    let context = ResolveContext {
        now: ctx.now(),
        timezone: query.executor().timezone().await?,
        user_id: scope.user_id().clone(),
    };
    let segments = RecordQueryEngine::new(&query)
        .segments(RecordType::DEAL.as_str(), filter)
        .await?;
    Ok(Planner {
        fields,
        context,
        visibility,
        segments,
    })
}

pub async fn require(
    ctx: &Ctx<'_>,
    visibility: &RecordScopeFilter,
    deal_id: &str,
) -> Result<DealRecord, AppError> {
    let scope = ctx.scope;
    let Some(id) = parse_id(deal_id) else {
        return Err(deal_not_found());
    };
    let record = DealStore::of(ctx)
        .find(scope, visibility, id)
        .await?
        .ok_or_else(deal_not_found)?;
    if visible_pipeline(ctx, record.deal.pipeline_id)
        .await?
        .is_none()
    {
        return Err(deal_not_found());
    }
    Ok(record)
}

async fn visible_pipeline(ctx: &Ctx<'_>, pipeline_id: Uuid) -> Result<Option<Pipeline>, AppError> {
    let scope = ctx.scope;
    Ok(PipelineStore::of(ctx)
        .find_pipeline(pipeline_id)
        .await?
        .filter(|pipeline| pipeline_visible(scope, pipeline)))
}

async fn visible_stage(ctx: &Ctx<'_>, stage_id: Uuid) -> Result<Option<PipelineStage>, AppError> {
    let Some(stage) = PipelineStore::of(ctx).find_stage(stage_id).await? else {
        return Ok(None);
    };
    Ok(visible_pipeline(ctx, stage.pipeline_id)
        .await?
        .map(|_| stage))
}

async fn default_visible_pipeline(ctx: &Ctx<'_>) -> Result<Option<Pipeline>, AppError> {
    let scope = ctx.scope;
    if sees_every_pipeline(scope) {
        return PipelineStore::of(ctx).default_pipeline().await;
    }
    let pipelines: Vec<Pipeline> = PipelineStore::of(ctx)
        .list_pipelines()
        .await?
        .into_iter()
        .filter(|pipeline| pipeline.is_active && pipeline_visible(scope, pipeline))
        .collect();
    Ok(pipelines
        .iter()
        .find(|pipeline| pipeline.is_default)
        .or_else(|| {
            pipelines
                .iter()
                .min_by_key(|pipeline| (pipeline.position, pipeline.id))
        })
        .cloned())
}

async fn pipeline_event(
    ctx: &Ctx<'_>,
    current: &DealRecord,
    deal: &Deal,
) -> Result<Option<(&'static str, Value)>, AppError> {
    if current.deal.pipeline_id == deal.pipeline_id {
        return Ok(None);
    }
    let pipelines = PipelineStore::of(ctx);
    let from = pipelines.find_pipeline(current.deal.pipeline_id).await?;
    let to = pipelines.find_pipeline(deal.pipeline_id).await?;
    let to_stage = pipelines.find_stage(deal.stage_id).await?;
    Ok(Some((
        DEAL_PIPELINE_CHANGED,
        json!({
            "from": from.as_ref().map(pipeline_ref),
            "to": to.as_ref().map(pipeline_ref),
            "from_stage": { "id": current.stage.id.to_string(), "name": current.stage.name },
            "to_stage": to_stage.as_ref().map(stage_ref),
        }),
    )))
}

async fn view(ctx: &Ctx<'_>, record: DealRecord) -> Result<DealView, AppError> {
    let scope = ctx.scope;
    let mut custom = CustomFieldReader::new(&ctx.fields())
        .load(scope, &RecordType::DEAL, &[record.deal.id])
        .await?;
    let mut contributed = load_contributed(
        ctx.pool(),
        ctx.registry().require(RecordType::DEAL.as_str())?,
        &[record.deal.id],
    )
    .await?;
    Ok(DealView {
        custom_fields: custom.remove(&record.deal.id).unwrap_or_default(),
        contributed: contributed.remove(&record.deal.id).unwrap_or_default(),
        record,
    })
}

async fn views(ctx: &Ctx<'_>, ids: &[Uuid]) -> Result<Vec<DealView>, AppError> {
    let scope = ctx.scope;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut records: HashMap<Uuid, DealRecord> = DealStore::of(ctx)
        .load(ids)
        .await?
        .into_iter()
        .map(|record| (record.deal.id, record))
        .collect();
    let mut custom = CustomFieldReader::new(&ctx.fields())
        .load(scope, &RecordType::DEAL, ids)
        .await?;
    let mut contributed = load_contributed(
        ctx.pool(),
        ctx.registry().require(RecordType::DEAL.as_str())?,
        ids,
    )
    .await?;
    Ok(ids
        .iter()
        .filter_map(|id| {
            records.remove(id).map(|record| DealView {
                custom_fields: custom.remove(id).unwrap_or_else(Map::new),
                contributed: contributed.remove(id).unwrap_or_default(),
                record,
            })
        })
        .collect())
}
