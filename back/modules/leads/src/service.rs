use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_sdk::configuration::{is_config_key, key_from_name};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_changes};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Map;
use sqlx::MySqlPool;

use crate::domain::{
    DEFAULT_REPORT_DAYS, DisqualificationReason, LEAD_STAGE, LIFECYCLE_STAGE_FIELD,
    LeadDisqualification, LifecycleReport, LifecycleSettings, MAX_DISQUALIFICATION_NOTE_LENGTH,
    MAX_DISQUALIFICATION_REASON_NAME_LENGTH, MAX_DISQUALIFICATION_REASONS, MAX_REPORT_DAYS,
    MAX_STAGE_LENGTH, QUALIFIED_STAGE, ReportInput, ReportWindow, StageHistoryEntry, StageOption,
    TransitionRule, TransitionTrigger, build_stage_reports, parse_lifecycle_record_type,
};
use crate::hooks::{
    ConvertLeadCommand, ConvertedLead, DisqualifyingLead, Lead, LeadConverted, LeadConverter,
    LeadDisqualifying, LeadQualifying, LeadState,
};
use crate::store::{HistoryStore, LeadRecords, ReasonStore, SettingsStore};
use aphel_kernel::Ctx;
use aphel_sdk::platform::application::custom_objects::{CatalogEventConsumer, RecordTypeCatalog};

use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::EventConsumer;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::infrastructure::audit::MySqlAuditLog;
use aphel_sdk::platform::infrastructure::events::MySqlEventPublisher;

use aphel_sdk::deals::{DEAL_CREATED, DEAL_DELETED, DEAL_LOST, DEAL_REOPENED, DEAL_WON};

pub(crate) const DISQUALIFICATION_REASON_NOT_FOUND: &str = "disqualification_reason_not_found";
pub(crate) const DISQUALIFICATION_REASON_KEY_TAKEN: &str = "disqualification_reason_key_taken";
pub(crate) const LIFECYCLE_RECORD_NOT_FOUND: &str = "record_not_found";
pub(crate) const DISQUALIFICATION_REASON_RECORD_TYPE: RecordType =
    RecordType::from_static("leads.lead_disqualification_reason");

pub(crate) const LIFECYCLE_SETTINGS_UPDATED: &str = "lifecycle.settings_updated";
pub(crate) const DISQUALIFICATION_REASON_CREATED: &str = "lead_disqualification_reason.created";
pub(crate) const DISQUALIFICATION_REASON_UPDATED: &str = "lead_disqualification_reason.updated";
pub(crate) const DISQUALIFICATION_REASON_DELETED: &str = "lead_disqualification_reason.deleted";
pub(crate) const DISQUALIFICATION_REASONS_REORDERED: &str =
    "lead_disqualification_reason.reordered";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TransitionRuleInput {
    pub(crate) trigger: String,
    pub(crate) enabled: Option<bool>,
    pub(crate) target_stage: Option<String>,
    pub(crate) from_stages: Option<Vec<String>>,
    pub(crate) from_empty: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SaveLifecycleSettingsCommand {
    pub(crate) rules: Vec<TransitionRuleInput>,
    pub(crate) disqualified_stage: Option<Option<String>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CreateDisqualificationReasonCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UpdateDisqualificationReasonCommand {
    pub(crate) name: Option<String>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LifecycleReportQuery {
    pub(crate) record_type: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleSettingsView {
    pub(crate) settings: LifecycleSettings,
    pub(crate) stages: Vec<StageOption>,
}

#[derive(Debug, Serialize)]
struct ReasonSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    is_active: bool,
}

impl<'a> From<&'a DisqualificationReason> for ReasonSnapshot<'a> {
    fn from(reason: &'a DisqualificationReason) -> Self {
        Self {
            key: &reason.key,
            name: &reason.name,
            is_active: reason.is_active,
        }
    }
}

pub(crate) fn settings_json(settings: &LifecycleSettings) -> Value {
    json!({
        "rules": settings
            .rules
            .iter()
            .map(|rule| json!({
                "trigger": rule.trigger.as_str(),
                "enabled": rule.enabled,
                "target_stage": rule.target_stage,
                "from_stages": rule.from_stages,
                "from_empty": rule.from_empty,
            }))
            .collect::<Vec<_>>(),
        "disqualified_stage": settings.disqualified_stage,
    })
}

pub(crate) fn disqualification_reason_not_found() -> AppError {
    AppError::not_found_code(
        DISQUALIFICATION_REASON_NOT_FOUND,
        "disqualification reason was not found",
    )
}

pub(crate) fn lifecycle_record_not_found() -> AppError {
    AppError::not_found_code(LIFECYCLE_RECORD_NOT_FOUND, "record was not found")
}

fn invalid_reason(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid disqualification reason", violations)
}

fn invalid_settings(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid lifecycle settings", violations)
}

pub(crate) fn reason_name(value: &str, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let name = value.trim();
    if name.is_empty() {
        violations.push(FieldViolation::new("name", "is required"));
        return None;
    }
    if name.chars().count() > MAX_DISQUALIFICATION_REASON_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have at most {MAX_DISQUALIFICATION_REASON_NAME_LENGTH} characters"),
        ));
        return None;
    }
    Some(name.to_owned())
}

pub(crate) fn read_permission(record_type: &RecordType) -> Permission {
    if *record_type == RecordType::COMPANY {
        Permission::COMPANIES_READ
    } else {
        Permission::CONTACTS_READ
    }
}

fn parse_date(
    field: &str,
    value: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<NaiveDate> {
    let text = value.map(str::trim).filter(|text| !text.is_empty())?;
    match NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        Ok(date) => Some(date),
        Err(_) => {
            violations.push(FieldViolation::new(field, "must be a date (YYYY-MM-DD)"));
            None
        }
    }
}

pub(crate) fn validate_rules(
    current: &LifecycleSettings,
    command: &SaveLifecycleSettingsCommand,
    stages: &[StageOption],
) -> Result<LifecycleSettings, AppError> {
    let mut violations = Vec::new();
    let mut next = current.clone().normalized();
    let active = |value: &str| {
        stages
            .iter()
            .any(|option| option.is_active && option.value == value)
    };
    let known = |value: &str| stages.iter().any(|option| option.value == value);
    let mut seen = BTreeSet::new();
    for (index, input) in command.rules.iter().enumerate() {
        let path = |property: &str| format!("rules.{index}.{property}");
        let Some(trigger) = TransitionTrigger::parse(input.trigger.trim()) else {
            violations.push(FieldViolation::new(
                path("trigger"),
                "must be deal_created, deal_won or all_deals_lost",
            ));
            continue;
        };
        if !seen.insert(trigger) {
            violations.push(FieldViolation::new(path("trigger"), "is repeated"));
            continue;
        }
        let mut rule: TransitionRule = next.rule(trigger);
        if let Some(enabled) = input.enabled {
            rule.enabled = enabled;
        }
        if let Some(target) = &input.target_stage {
            let target = target.trim();
            if !active(target) {
                violations.push(FieldViolation::new(
                    path("target_stage"),
                    "must be one of the active lifecycle stages",
                ));
            }
            rule.target_stage = target.to_owned();
        }
        if let Some(from_stages) = &input.from_stages {
            let mut values = Vec::new();
            for stage in from_stages {
                let stage = stage.trim();
                if stage.is_empty() || stage.chars().count() > MAX_STAGE_LENGTH || !known(stage) {
                    violations.push(FieldViolation::new(
                        path("from_stages"),
                        "must list lifecycle stages of the field",
                    ));
                    break;
                }
                if !values.iter().any(|value: &String| value == stage) {
                    values.push(stage.to_owned());
                }
            }
            rule.from_stages = values;
        }
        if let Some(from_empty) = input.from_empty {
            rule.from_empty = from_empty;
        }
        next.rules.retain(|existing| existing.trigger != trigger);
        next.rules.push(rule);
    }
    if let Some(stage) = &command.disqualified_stage {
        let stage = stage
            .as_deref()
            .map(str::trim)
            .filter(|stage| !stage.is_empty());
        if let Some(stage) = stage
            && !active(stage)
        {
            violations.push(FieldViolation::new(
                "disqualified_stage",
                "must be one of the active lifecycle stages",
            ));
        }
        next.disqualified_stage = stage.map(str::to_owned);
    }
    if !violations.is_empty() {
        return Err(invalid_settings(violations));
    }
    Ok(next.normalized())
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<LifecycleSettingsView, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::LIFECYCLE_MANAGE)?;
    Ok(LifecycleSettingsView {
        settings: SettingsStore::new(ctx.pool().clone())
            .get()
            .await?
            .normalized(),
        stages: LeadRecords::of(ctx)
            .stage_options(&RecordType::CONTACT)
            .await?,
    })
}

pub(crate) async fn save_settings(
    ctx: &Ctx<'_>,
    command: SaveLifecycleSettingsCommand,
) -> Result<LifecycleSettingsView, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::LIFECYCLE_MANAGE)?;
    let current = SettingsStore::new(ctx.pool().clone())
        .get()
        .await?
        .normalized();
    if let Some(updated_at) = current.updated_at {
        ensure_version(command.expected_updated_at, updated_at)?;
    }
    let stages = LeadRecords::of(ctx)
        .stage_options(&RecordType::CONTACT)
        .await?;
    let mut next = validate_rules(&current, &command, &stages)?;
    let changes = diff_changes(
        &settings_json(&current),
        &settings_json(&next),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() && current.updated_at.is_some() {
        return Ok(LifecycleSettingsView {
            settings: current,
            stages,
        });
    }
    let now = ctx.now();
    next.updated_at = Some(now);
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    SettingsStore::new(ctx.pool().clone())
        .save(&mut tx, &next, Some(scope.user_id()), now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), LIFECYCLE_SETTINGS_UPDATED, now)
                .with_payload(settings_json(&next)),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, LIFECYCLE_SETTINGS_UPDATED, now)
                .with_changes(changes)
                .with_ip(command.ip),
        )
        .await?;
    tx.commit().await?;
    Ok(LifecycleSettingsView {
        settings: next,
        stages,
    })
}

pub(crate) async fn list_reasons(
    ctx: &Ctx<'_>,
    include_inactive: bool,
) -> Result<Vec<DisqualificationReason>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::CONTACTS_READ)?;
    ReasonStore::new(ctx.pool().clone())
        .list(include_inactive)
        .await
}

pub(crate) async fn create_reason(
    ctx: &Ctx<'_>,
    command: CreateDisqualificationReasonCommand,
) -> Result<DisqualificationReason, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::LIFECYCLE_MANAGE)?;
    let mut violations = Vec::new();
    let name = reason_name(&command.name, &mut violations);
    let explicit_key = command
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    if let Some(key) = explicit_key
        && !is_config_key(key)
    {
        violations.push(FieldViolation::new(
            "key",
            "must start with a lowercase letter and use only lowercase letters, digits and underscores",
        ));
    }
    let existing = ReasonStore::new(ctx.pool().clone()).list(true).await?;
    if existing.len() >= MAX_DISQUALIFICATION_REASONS {
        violations.push(FieldViolation::new(
            "name",
            format!(
                "an organization can have at most {MAX_DISQUALIFICATION_REASONS} disqualification reasons"
            ),
        ));
    }
    let (Some(name), true) = (name, violations.is_empty()) else {
        return Err(invalid_reason(violations));
    };
    let taken = |key: &str| existing.iter().any(|reason| reason.key == key);
    let key = match explicit_key {
        Some(key) if taken(key) => {
            return Err(AppError::conflict_code(
                DISQUALIFICATION_REASON_KEY_TAKEN,
                "another disqualification reason already uses this key",
            ));
        }
        Some(key) => key.to_owned(),
        None => {
            let base = key_from_name(&name);
            let mut candidate = base.clone();
            let mut suffix = 2;
            while taken(&candidate) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            candidate
        }
    };
    let now = ctx.now();
    let reason = DisqualificationReason {
        id: Uuid::now_v7(),
        key,
        name,
        is_active: true,
        position: existing
            .iter()
            .map(|reason| reason.position)
            .max()
            .map_or(0, |position| position + 1),
        created_at: now,
        updated_at: now,
    };
    let changes = diff_changes(
        &Value::Null,
        &ReasonSnapshot::from(&reason),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ReasonStore::new(ctx.pool().clone())
        .insert(&mut tx, &reason)
        .await?;
    publish_reason(
        ctx,
        &mut tx,
        &reason,
        DISQUALIFICATION_REASON_CREATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(reason)
}

pub(crate) async fn update_reason(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateDisqualificationReasonCommand,
) -> Result<DisqualificationReason, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::LIFECYCLE_MANAGE)?;
    let current = find_reason(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut next = current.clone();
    if let Some(name) = &command.name
        && let Some(name) = reason_name(name, &mut violations)
    {
        next.name = name;
    }
    if let Some(is_active) = command.is_active {
        next.is_active = is_active;
    }
    if !violations.is_empty() {
        return Err(invalid_reason(violations));
    }
    let changes = diff_changes(
        &ReasonSnapshot::from(&current),
        &ReasonSnapshot::from(&next),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    next.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    ReasonStore::new(ctx.pool().clone())
        .update(&mut tx, &next)
        .await?;
    publish_reason(
        ctx,
        &mut tx,
        &next,
        DISQUALIFICATION_REASON_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub(crate) async fn delete_reason(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::LIFECYCLE_MANAGE)?;
    let current = find_reason(ctx, id).await?;
    let changes = diff_changes(
        &ReasonSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ReasonStore::new(ctx.pool().clone())
        .delete(&mut tx, current.id)
        .await?;
    publish_reason(
        ctx,
        &mut tx,
        &current,
        DISQUALIFICATION_REASON_DELETED,
        changes,
        ip,
    )
    .await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn reorder_reasons(
    ctx: &Ctx<'_>,
    ids: &[String],
    ip: Option<String>,
) -> Result<Vec<DisqualificationReason>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::LIFECYCLE_MANAGE)?;
    let existing = ReasonStore::new(ctx.pool().clone()).list(true).await?;
    let invalid_order = || {
        AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(
                "ids",
                "must list every disqualification reason of the organization exactly once",
            )],
        )
    };
    let ordered = ids
        .iter()
        .map(|id| Uuid::parse_str(id.trim()).map_err(|_| invalid_order()))
        .collect::<Result<Vec<_>, _>>()?;
    let requested: BTreeSet<Uuid> = ordered.iter().copied().collect();
    let known: BTreeSet<Uuid> = existing.iter().map(|reason| reason.id).collect();
    if requested.len() != ordered.len() || requested != known {
        return Err(invalid_order());
    }
    let previous: Vec<String> = existing.iter().map(|reason| reason.key.clone()).collect();
    let keys: Vec<String> = ordered
        .iter()
        .filter_map(|id| existing.iter().find(|reason| reason.id == *id))
        .map(|reason| reason.key.clone())
        .collect();
    if previous == keys {
        return Ok(existing);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    ReasonStore::new(ctx.pool().clone())
        .reorder(&mut tx, &ordered, now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), DISQUALIFICATION_REASONS_REORDERED, now)
                .with_payload(json!({ "keys": keys })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, DISQUALIFICATION_REASONS_REORDERED, now)
                .with_changes(vec![FieldChange::new(
                    "order",
                    json!(previous),
                    json!(keys),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    ReasonStore::new(ctx.pool().clone()).list(true).await
}

async fn require_lifecycle_field(ctx: &Ctx<'_>, record_type: &RecordType) -> Result<(), AppError> {
    if !aphel_sdk::records::enabled(ctx, record_type) {
        return Err(lifecycle_record_not_found());
    }
    let fields = ctx.field_set(ctx.scope, record_type.as_str()).await?;
    if fields
        .find_active(LIFECYCLE_STAGE_FIELD)
        .is_none_or(|field| field.is_hidden())
    {
        return Err(AppError::Forbidden(
            "the lifecycle stage field is not readable".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn history(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<StageHistoryEntry>, AppError> {
    let scope = ctx.scope;
    let record_type =
        parse_lifecycle_record_type(record_type).ok_or_else(lifecycle_record_not_found)?;
    scope.record_filter(&read_permission(&record_type))?;
    require_lifecycle_field(ctx, &record_type).await?;
    let record_id = Uuid::parse_str(record_id.trim()).map_err(|_| lifecycle_record_not_found())?;
    if !LeadRecords::of(ctx)
        .visible(scope, &record_type, record_id)
        .await?
    {
        return Err(lifecycle_record_not_found());
    }
    HistoryStore::new(ctx.pool().clone())
        .list(&record_type, record_id)
        .await
}

pub(crate) async fn report(
    ctx: &Ctx<'_>,
    query: LifecycleReportQuery,
) -> Result<LifecycleReport, AppError> {
    let scope = ctx.scope;
    let mut violations = Vec::new();
    let record_type = match query.record_type.as_deref().map(str::trim) {
        None | Some("") => RecordType::CONTACT,
        Some(value) => match parse_lifecycle_record_type(value) {
            Some(record_type) => record_type,
            None => {
                violations.push(FieldViolation::new(
                    "record_type",
                    "must be contact or company",
                ));
                RecordType::CONTACT
            }
        },
    };
    let today = ctx.now().date_naive();
    let to = parse_date("to", query.to.as_deref(), &mut violations).unwrap_or(today);
    let from = parse_date("from", query.from.as_deref(), &mut violations)
        .unwrap_or(to - Duration::days(DEFAULT_REPORT_DAYS - 1));
    if from > to {
        violations.push(FieldViolation::new("from", "must not be after to"));
    } else if (to - from).num_days() >= MAX_REPORT_DAYS {
        violations.push(FieldViolation::new(
            "from",
            format!("the period can have at most {MAX_REPORT_DAYS} days"),
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid report", violations));
    }
    let visibility = scope.record_filter(&read_permission(&record_type))?;
    require_lifecycle_field(ctx, &record_type).await?;
    let window = ReportWindow { from, to };
    let records = LeadRecords::of(ctx);
    let options = records.stage_options(&record_type).await?;
    let spans = records
        .report_spans(scope, &record_type, &visibility, window.start())
        .await?;
    let current = records
        .current_counts(scope, &record_type, &visibility)
        .await?;
    let disqualifications = if record_type == RecordType::CONTACT {
        records
            .disqualification_counts(scope, &visibility, &window)
            .await?
    } else {
        Vec::new()
    };
    let stages = build_stage_reports(&ReportInput {
        options: &options,
        window: &window,
        spans: &spans,
        current: &current,
    });
    Ok(LifecycleReport {
        record_type,
        from,
        to,
        stages,
        disqualifications,
    })
}

async fn find_reason(ctx: &Ctx<'_>, id: &str) -> Result<DisqualificationReason, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| disqualification_reason_not_found())?;
    ReasonStore::new(ctx.pool().clone())
        .find(id)
        .await?
        .ok_or_else(disqualification_reason_not_found)
}

async fn publish_reason(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    reason: &DisqualificationReason,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(DISQUALIFICATION_REASON_RECORD_TYPE, reason.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "key": reason.key,
                    "name": reason.name,
                    "is_active": reason.is_active,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

pub(crate) const CONTACT_NOT_LEAD: &str = "contact_not_lead";

pub(crate) const LEAD_QUALIFIED: &str = "lead.qualified";
pub(crate) const LEAD_DISQUALIFIED: &str = "lead.disqualified";
pub(crate) const LEAD_CONVERTED: &str = "lead.converted";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DisqualifyLeadCommand {
    pub(crate) reason_id: String,
    pub(crate) note: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeadActionResult {
    pub(crate) contact_id: Uuid,
    pub(crate) lifecycle_stage: Option<String>,
    pub(crate) deal_id: Option<Uuid>,
}

fn not_lead() -> AppError {
    AppError::conflict_code(
        CONTACT_NOT_LEAD,
        "the contact is not in a lifecycle stage that allows this action",
    )
}

enum StageAction {
    Qualify,
    Disqualify(LeadDisqualification, Box<DisqualifyingLead>),
    Convert(Box<ConvertedLead>),
}

struct StageMove {
    lead: Lead,
    to_stage: Option<String>,
    event_type: &'static str,
    payload: Value,
    action: StageAction,
    ip: Option<String>,
}

async fn lead(ctx: &Ctx<'_>, contact_id: &str) -> Result<Lead, AppError> {
    ctx.scope.require(&Permission::CONTACTS_UPDATE)?;
    let id = Uuid::parse_str(contact_id).map_err(|_| not_lead())?;
    if !ctx
        .can_access_record(
            &RecordRef::new(RecordType::CONTACT, id),
            &Permission::CONTACTS_UPDATE,
        )
        .await?
    {
        return Err(not_lead());
    }
    Lead::from_record(
        ctx.load_record(RecordType::CONTACT.as_str(), id)
            .await?
            .ok_or_else(not_lead)?,
    )
}

pub(crate) async fn qualify_lead(
    ctx: &Ctx<'_>,
    contact_id: &str,
    ip: Option<String>,
) -> Result<LeadActionResult, AppError> {
    let lead = lead(ctx, contact_id).await?;
    if lead.stage() != Some(LEAD_STAGE) {
        return Err(not_lead());
    }
    let contact_id = lead.id();
    move_stage(
        ctx,
        StageMove {
            lead,
            to_stage: Some(QUALIFIED_STAGE.to_owned()),
            event_type: LEAD_QUALIFIED,
            payload: json!({}),
            action: StageAction::Qualify,
            ip,
        },
    )
    .await?;
    Ok(LeadActionResult {
        contact_id,
        lifecycle_stage: Some(QUALIFIED_STAGE.to_owned()),
        deal_id: None,
    })
}

pub(crate) async fn disqualify_lead(
    ctx: &Ctx<'_>,
    contact_id: &str,
    command: DisqualifyLeadCommand,
) -> Result<LeadActionResult, AppError> {
    let scope = ctx.scope;
    let lead = lead(ctx, contact_id).await?;
    if !lead.is_open() {
        return Err(not_lead());
    }
    let mut violations = Vec::new();
    let reasons = ReasonStore::new(ctx.pool().clone());
    let reason = match Uuid::parse_str(command.reason_id.trim()) {
        Ok(id) => reasons.find(id).await?,
        Err(_) => None,
    };
    let reason = match reason {
        Some(reason) if reason.is_active => Some(reason),
        Some(_) => {
            violations.push(FieldViolation::new("reason_id", "the reason is inactive"));
            None
        }
        None => {
            violations.push(FieldViolation::new(
                "reason_id",
                "must be a disqualification reason of the organization",
            ));
            None
        }
    };
    let note = command
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
        .map(str::to_owned);
    if note
        .as_ref()
        .is_some_and(|note| note.chars().count() > MAX_DISQUALIFICATION_NOTE_LENGTH)
    {
        violations.push(FieldViolation::new(
            "notes.note",
            format!("must have at most {MAX_DISQUALIFICATION_NOTE_LENGTH} characters"),
        ));
    }
    let Some(reason) = reason.filter(|_| violations.is_empty()) else {
        return Err(AppError::invalid_fields(
            "invalid disqualification",
            violations,
        ));
    };
    let to_stage = SettingsStore::new(ctx.pool().clone())
        .get()
        .await?
        .disqualified_stage;
    let actor = Actor::from_scope(scope);
    let contact_id = lead.id();
    let disqualification = LeadDisqualification {
        id: Uuid::now_v7(),
        contact_id,
        reason_id: Some(reason.id),
        reason_name: reason.name.clone(),
        note: note.clone(),
        from_stage: lead.stage().map(str::to_owned),
        actor_type: actor.kind().to_owned(),
        actor_id: actor.id().map(str::to_owned),
        at: ctx.now(),
    };
    let hook_args = DisqualifyingLead {
        lead: lead.clone(),
        reason: reason.clone(),
        note: note.clone(),
    };
    move_stage(
        ctx,
        StageMove {
            lead,
            to_stage: to_stage.clone(),
            event_type: LEAD_DISQUALIFIED,
            payload: json!({
                "reason": { "id": reason.id, "key": reason.key, "name": reason.name },
                "has_note": note.is_some(),
            }),
            action: StageAction::Disqualify(disqualification, Box::new(hook_args)),
            ip: command.ip,
        },
    )
    .await?;
    Ok(LeadActionResult {
        contact_id,
        lifecycle_stage: to_stage,
        deal_id: None,
    })
}

pub(crate) async fn convert_lead(
    ctx: &Ctx<'_>,
    contact_id: &str,
    command: ConvertLeadCommand,
) -> Result<LeadActionResult, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::DEALS_CREATE)?;
    let lead = lead(ctx, contact_id).await?;
    if !lead.is_open() {
        return Err(not_lead());
    }
    let mut tx = ctx.begin().await?;
    let updated_at = aphel_sdk::records::lock_entity_version(
        &mut tx,
        ctx.registry().require(RecordType::CONTACT.as_str())?,
        lead.id(),
    )
    .await?
    .ok_or_else(not_lead)?;
    ensure_version(Some(lead.updated_at), updated_at)?;
    let to_stage = SettingsStore::new(ctx.pool().clone())
        .get()
        .await?
        .converted_stage();
    let conversion = ctx
        .services()
        .require::<dyn LeadConverter>()?
        .convert(ctx, &mut tx, &lead, &command)
        .await?;
    let contact_id = lead.id();
    let pending = move_stage_in_tx(
        ctx,
        &mut tx,
        StageMove {
            lead: lead.clone(),
            to_stage: Some(to_stage.clone()),
            event_type: LEAD_CONVERTED,
            payload: json!({ "deal_id": conversion.deal_id }),
            action: StageAction::Convert(Box::new(ConvertedLead {
                lead,
                deal_id: conversion.deal_id,
                company_id: conversion.company_id,
            })),
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    for created in conversion.pending {
        created.committed(ctx).await;
    }
    if let Some(pending) = pending {
        pending.committed(ctx).await;
    }
    Ok(LeadActionResult {
        contact_id,
        lifecycle_stage: Some(to_stage),
        deal_id: Some(conversion.deal_id),
    })
}

async fn move_stage(ctx: &Ctx<'_>, stage_move: StageMove) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    let pending = move_stage_in_tx(ctx, &mut tx, stage_move).await?;
    tx.commit().await?;
    if let Some(pending) = pending {
        pending.committed(ctx).await;
    }
    Ok(())
}

async fn move_stage_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut aphel_sdk::platform::database::transaction::MySqlTx,
    stage_move: StageMove,
) -> Result<Option<aphel_kernel::records::PendingCommit>, AppError> {
    let scope = ctx.scope;
    let record = RecordRef::new(RecordType::CONTACT, stage_move.lead.id());
    match &stage_move.action {
        StageAction::Qualify => {
            ctx.run_hook(LeadQualifying, tx, super::KEY, &stage_move.lead)
                .await?
        }
        StageAction::Disqualify(_, args) => {
            ctx.run_hook(LeadDisqualifying, tx, super::KEY, args)
                .await?
        }
        StageAction::Convert(_) => {}
    }
    let pending = ctx.update_record_in_tx(tx, RecordType::CONTACT.as_str(), stage_move.lead.id(), json!({ LIFECYCLE_STAGE_FIELD: stage_move.to_stage, "_expected_updated_at": stage_move.lead.updated_at })).await?;
    let changes = pending
        .as_ref()
        .map(|pending| pending.changes.clone())
        .unwrap_or_default();
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut audit_changes = changes.clone();
    if let StageAction::Disqualify(disqualification, _) = &stage_move.action {
        audit_changes.push(FieldChange::new(
            "disqualification_reason",
            Value::Null,
            json!(disqualification.reason_name),
        ));
        if let Some(note) = &disqualification.note {
            audit_changes.push(FieldChange::new(
                "disqualification_note",
                Value::Null,
                json!(note),
            ));
        }
    }
    let mut payload = stage_move.payload;
    if let Value::Object(map) = &mut payload {
        map.insert("name".to_owned(), json!(stage_move.lead.name()));
        map.insert(
            "from_stage".to_owned(),
            stage_move
                .lead
                .values
                .get(LIFECYCLE_STAGE_FIELD)
                .cloned()
                .unwrap_or(Value::Null),
        );
        map.insert("to_stage".to_owned(), json!(stage_move.to_stage));
    }
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), stage_move.event_type, now)
                .with_record(record.clone())
                .with_payload(payload.clone()),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, stage_move.event_type, now)
                .with_record(record.clone())
                .with_changes(audit_changes)
                .with_ip(stage_move.ip),
        )
        .await?;
    match &stage_move.action {
        StageAction::Disqualify(disqualification, _) => {
            ReasonStore::new(ctx.pool().clone())
                .record_disqualification(tx, disqualification)
                .await?
        }
        StageAction::Convert(converted) => {
            ctx.run_hook(LeadConverted, tx, super::KEY, converted)
                .await?
        }
        StageAction::Qualify => {}
    }
    Ok(pending)
}

pub(crate) const LIFECYCLE_TRANSITIONS_CONSUMER: &str = "lifecycle.transitions";

pub(crate) fn transition_trigger(event_type: &str) -> Option<TransitionTrigger> {
    match event_type {
        DEAL_CREATED | DEAL_REOPENED => Some(TransitionTrigger::DealCreated),
        DEAL_WON => Some(TransitionTrigger::DealWon),
        DEAL_LOST | DEAL_DELETED => Some(TransitionTrigger::AllDealsLost),
        _ => None,
    }
}

fn is_skippable(error: &AppError) -> bool {
    matches!(
        error,
        AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Validation(_)
            | AppError::NotFound(_)
            | AppError::Coded { .. }
            | AppError::Conflict(_)
            | AppError::Forbidden(_)
    )
}

pub(crate) struct LifecycleTransitions {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl LifecycleTransitions {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    fn ctx<'s>(&self, scope: &'s AccessScope) -> Ctx<'s> {
        Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.registry.clone(),
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        )
    }

    async fn set_stage(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        record_id: Uuid,
        stage: &str,
    ) -> Result<(), AppError> {
        let ctx = self.ctx(scope);
        let mut contributed = Map::new();
        contributed.insert(LIFECYCLE_STAGE_FIELD.to_owned(), json!(stage));
        ctx.update_record(record_type.as_str(), record_id, Value::Object(contributed))
            .await?;
        Ok(())
    }

    async fn apply(
        &self,
        scope: &AccessScope,
        rule: &TransitionRule,
        record_type: RecordType,
        record_id: Uuid,
    ) -> Result<(), AppError> {
        let records = LeadRecords::new(self.pool.clone(), self.registry.clone());
        if rule.trigger == TransitionTrigger::AllDealsLost {
            let Some(outcomes) = records.deal_outcomes(&record_type, record_id).await? else {
                return Ok(());
            };
            if outcomes.open > 0 || outcomes.won > 0 || outcomes.lost == 0 {
                return Ok(());
            }
        }
        let Some(current) = records.current_stage(&record_type, record_id).await? else {
            return Ok(());
        };
        if !rule.applies_from(current.as_deref()) {
            return Ok(());
        }
        match self
            .set_stage(scope, &record_type, record_id, &rule.target_stage)
            .await
        {
            Err(error) if is_skippable(&error) => {
                tracing::warn!(
                    record_type = record_type.as_str(),
                    %record_id,
                    target = rule.target_stage,
                    "lifecycle transition skipped: {error}"
                );
                Ok(())
            }
            other => other,
        }
    }
}

#[async_trait]
impl EventConsumer for LifecycleTransitions {
    fn name(&self) -> &str {
        LIFECYCLE_TRANSITIONS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        transition_trigger(event_type).is_some()
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(trigger) = transition_trigger(&event.event_type) else {
            return Ok(());
        };
        let Some(record) = event
            .record
            .as_ref()
            .filter(|record| record.record_type == RecordType::DEAL)
        else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let rule = SettingsStore::new(self.pool.clone())
            .get()
            .await?
            .rule(trigger);
        if !rule.enabled {
            return Ok(());
        }
        let Some(parties) = LeadRecords::new(self.pool.clone(), self.registry.clone())
            .deal_parties(record.record_id)
            .await?
        else {
            return Ok(());
        };
        if let Some(contact_id) = parties.contact_id {
            self.apply(&scope, &rule, RecordType::CONTACT, contact_id)
                .await?;
        }
        if let Some(company_id) = parties.company_id {
            self.apply(&scope, &rule, RecordType::COMPANY, company_id)
                .await?;
        }
        Ok(())
    }
}

pub(crate) fn lifecycle_transitions_consumer(
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
) -> Arc<dyn EventConsumer> {
    CatalogEventConsumer::shared(
        catalog,
        Arc::new(move |registry| {
            Arc::new(LifecycleTransitions::new(pool.clone(), registry)) as Arc<dyn EventConsumer>
        }),
    )
}

pub async fn record_stage_history(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    from_stage: Option<String>,
    to_stage: Option<String>,
) -> Result<(), AppError> {
    if !crate::domain::is_lifecycle_record_type(&record.record_type) || from_stage == to_stage {
        return Ok(());
    }
    let actor = Actor::from_scope(ctx.scope);
    HistoryStore::new(ctx.pool().clone())
        .record(
            tx,
            &crate::domain::StageChange {
                record_type: record.record_type.clone(),
                record_id: record.record_id,
                from_stage,
                to_stage,
                actor_type: actor.kind().to_owned(),
                actor_id: actor.id().map(str::to_owned),
                at: ctx.now(),
            },
        )
        .await
}
