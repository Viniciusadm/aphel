use crate::gateway::AutomationContext;
use aphel_contract_crm::CrmPermissions;
use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::clock::automation_clock;
use crate::engine::{DiscardSteps, EngineInput, EngineMode, EngineOutcome};
use crate::model::*;
use crate::runtime::{RunJobPayload, run_job};
use crate::store::{
    AutomationRunStore, AutomationStore, AutomationTimeStore, CancelTarget, RunInsert,
};
use crate::validation::{AutomationValidator, ValidationLevel};
use aphel_contract_automation::application::actions::{
    ActionRegistry, SECRET_OWNER_TYPE, configured_flag, secret_key,
};
use aphel_contract_automation::application::context::TriggerContext;
use aphel_contract_automation::application::definition::{
    AutomationDefinition, StepBody, TriggerSpec, parse_definition, parse_trigger,
};
use aphel_contract_automation::application::gateway::AutomationGatewayFactory;
use aphel_contract_automation::application::templates::TemplateScope;
use aphel_contract_automation::application::triggers::{MANUAL, TriggerRegistry};
use aphel_contract_automation::domain::{AutomationStatus, RunStatus, SkipReason};
use aphel_kernel::Ctx;
use aphel_kernel::domain::Secret;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_changes};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::secrets::SecretVault;
use aphel_sdk::platform::application::security::Clock;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, JobQueue, RecordRef,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;

use aphel_sdk::platform::application::shared::parse_uuid as parse_id;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CreateAutomationCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) trigger: Value,
    pub(crate) definition: Value,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UpdateAutomationCommand {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) trigger: Option<Value>,
    pub(crate) definition: Option<Value>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct DryRunCommand {
    pub(crate) record_id: String,
    pub(crate) use_draft: bool,
    pub(crate) changed_fields: Vec<String>,
    pub(crate) previous: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DryRunResult {
    pub(crate) version: Option<u32>,
    pub(crate) record: RecordRef,
    pub(crate) record_available: bool,
    pub(crate) outcome: EngineOutcome,
    pub(crate) steps: Vec<RunStep>,
}

#[derive(Serialize)]
struct AutomationSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    description: Option<&'a str>,
    record_type: &'a str,
    trigger: &'a TriggerSpec,
    definition: &'a AutomationDefinition,
    status: &'static str,
    version: u32,
}

impl<'a> From<&'a Automation> for AutomationSnapshot<'a> {
    fn from(automation: &'a Automation) -> Self {
        Self {
            key: &automation.key,
            name: &automation.name,
            description: automation.description.as_deref(),
            record_type: &automation.record_type,
            trigger: &automation.trigger,
            definition: &automation.definition,
            status: automation.status.as_str(),
            version: automation.version,
        }
    }
}

struct SecretChange {
    step_id: String,
    field: String,
    value: Option<String>,
}

pub(crate) fn automation_not_found() -> AppError {
    AppError::not_found_code(AUTOMATION_NOT_FOUND, "automation not found")
}

fn run_not_found() -> AppError {
    AppError::not_found_code(AUTOMATION_RUN_NOT_FOUND, "automation run not found")
}

fn invalid(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields_code(INVALID_AUTOMATION, "invalid automation", violations)
}

pub(crate) fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY_LENGTH
        && key.starts_with(|character: char| character.is_ascii_lowercase())
        && key.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

pub(crate) fn key_from_name(name: &str) -> String {
    let mut key = String::new();
    for character in name.to_lowercase().chars() {
        let mapped = match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            character if character.is_ascii_alphanumeric() => character,
            _ => '_',
        };
        if mapped == '_' && (key.is_empty() || key.ends_with('_')) {
            continue;
        }
        key.push(mapped);
    }
    let mut key: String = key.trim_end_matches('_').chars().take(48).collect();
    if !key.starts_with(|character: char| character.is_ascii_lowercase()) {
        key = format!("automation_{key}").trim_end_matches('_').to_owned();
    }
    key
}

fn normalize_description(description: Option<String>) -> Option<String> {
    description
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn check_texts(name: &str, description: Option<&str>, violations: &mut Vec<FieldViolation>) {
    let length = name.chars().count();
    if length == 0 || length > MAX_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have 1 to {MAX_NAME_LENGTH} characters"),
        ));
    }
    if description.is_some_and(|text| text.chars().count() > MAX_DESCRIPTION_LENGTH) {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
        ));
    }
}

fn extract_secrets(
    definition: &mut AutomationDefinition,
    previous: Option<&AutomationDefinition>,
    secret_fields: impl Fn(&str) -> &'static [&'static str],
) -> Vec<SecretChange> {
    let mut changes = Vec::new();
    for step in &mut definition.steps {
        let StepBody::Action { action, config, .. } = &mut step.body else {
            continue;
        };
        let fields = secret_fields(action);
        if fields.is_empty() {
            continue;
        }
        let previous_config = previous
            .and_then(|definition| definition.step(&step.id))
            .and_then(|step| match &step.body {
                StepBody::Action { config, .. } => Some(config.clone()),
                _ => None,
            });
        let Some(map) = config.as_object_mut() else {
            continue;
        };
        for field in fields {
            let flag = configured_flag(field);
            match map.remove(*field) {
                Some(Value::String(secret)) if !secret.trim().is_empty() => {
                    changes.push(SecretChange {
                        step_id: step.id.clone(),
                        field: (*field).to_owned(),
                        value: Some(secret.trim().to_owned()),
                    });
                    map.insert(flag, Value::Bool(true));
                }
                Some(Value::Null) => {
                    changes.push(SecretChange {
                        step_id: step.id.clone(),
                        field: (*field).to_owned(),
                        value: None,
                    });
                    map.insert(flag, Value::Bool(false));
                }
                _ => {
                    let configured = previous_config
                        .as_ref()
                        .and_then(|config| config.get(&flag))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    map.insert(flag, Value::Bool(configured));
                }
            }
        }
    }
    changes
}

fn current_time() -> DateTime<Utc> {
    automation_clock().now()
}

fn secret_fields(actions: &ActionRegistry, action: &str) -> &'static [&'static str] {
    actions
        .get(action)
        .map(|action| action.secret_fields())
        .unwrap_or(&[])
}

async fn validate(
    ctx: &Ctx<'_>,
    record_type: &str,
    trigger: &TriggerSpec,
    definition: &AutomationDefinition,
    level: ValidationLevel,
) -> Result<Vec<FieldViolation>, AppError> {
    let gateway = ctx.automation_gateways().gateway(None);
    let triggers = TriggerRegistry::builtin();
    let actions = ActionRegistry::builtin();
    AutomationValidator {
        scope: ctx.scope,
        gateway: gateway.as_ref(),
        triggers: &triggers,
        actions: &actions,
    }
    .validate(record_type, trigger, definition, level)
    .await
}

pub(crate) fn require_manage(ctx: &Ctx<'_>) -> Result<(), AppError> {
    ctx.scope
        .require(&Permission::AUTOMATIONS_MANAGE)
        .map(|_| ())
}

pub(crate) fn catalog(ctx: &Ctx<'_>) -> Result<(TriggerRegistry, ActionRegistry), AppError> {
    require_manage(ctx)?;
    Ok((TriggerRegistry::builtin(), ActionRegistry::builtin()))
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    filter: AutomationListFilter,
    window: ListWindow,
) -> Result<Listing<AutomationSummary>, AppError> {
    require_manage(ctx)?;
    let listing = AutomationStore::of(ctx).list(&filter, window).await?;
    let ids: Vec<Uuid> = listing
        .items
        .iter()
        .map(|automation| automation.id)
        .collect();
    let since = current_time() - Duration::days(STATS_WINDOW_DAYS);
    let mut stats = AutomationRunStore::of(ctx).stats(&ids, since).await?;
    Ok(Listing {
        total: listing.total,
        items: listing
            .items
            .into_iter()
            .map(|automation| AutomationSummary {
                stats: stats.remove(&automation.id).unwrap_or_default(),
                automation,
            })
            .collect(),
    })
}

async fn cancel_waiting(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    automation_id: Uuid,
    reason: SkipReason,
) -> Result<u64, AppError> {
    AutomationTimeStore::of(ctx)
        .cancel_waiting(
            tx,
            &CancelTarget::Automation(automation_id),
            reason,
            current_time(),
        )
        .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<Automation, AppError> {
    require_manage(ctx)?;
    let id = parse_id(id).ok_or_else(automation_not_found)?;
    AutomationStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(automation_not_found)
}

pub(crate) async fn versions(ctx: &Ctx<'_>, id: &str) -> Result<Vec<AutomationVersion>, AppError> {
    let automation = get(ctx, id).await?;
    AutomationStore::of(ctx).list_versions(automation.id).await
}

async fn unique_key(store: &AutomationStore, base: &str) -> Result<String, AppError> {
    for attempt in 1..=50 {
        let candidate = if attempt == 1 {
            base.to_owned()
        } else {
            format!("{base}_{attempt}")
        };
        if store.find_by_key(&candidate).await?.is_none() {
            return Ok(candidate);
        }
    }
    Ok(format!("{base}_{}", Uuid::now_v7().simple()))
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateAutomationCommand,
) -> Result<Automation, AppError> {
    require_manage(ctx)?;
    let store = AutomationStore::of(ctx);
    if store.count().await? >= MAX_AUTOMATIONS {
        return Err(AppError::conflict_code(
            AUTOMATION_LIMIT_REACHED,
            format!("an organization can have at most {MAX_AUTOMATIONS} automations"),
        ));
    }
    let name = command.name.trim().to_owned();
    let description = normalize_description(command.description);
    let mut violations = Vec::new();
    check_texts(&name, description.as_deref(), &mut violations);
    let key = match command
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        Some(key) if valid_key(key) => {
            if store.find_by_key(key).await?.is_some() {
                return Err(AppError::conflict_code(
                    AUTOMATION_KEY_TAKEN,
                    "another automation already uses this key",
                ));
            }
            key.to_owned()
        }
        Some(_) => {
            violations.push(FieldViolation::new(
                "key",
                "must start with a letter and have up to 64 lowercase letters, digits or '_'",
            ));
            String::new()
        }
        None => unique_key(&store, &key_from_name(&name)).await?,
    };
    let trigger = parse_trigger(&command.trigger).map_err(|violation| invalid(vec![violation]))?;
    let mut definition =
        parse_definition(&command.definition).map_err(|violation| invalid(vec![violation]))?;
    let record_type = command.record_type.trim().to_owned();
    violations.extend(
        validate(
            ctx,
            &record_type,
            &trigger,
            &definition,
            ValidationLevel::Save,
        )
        .await?,
    );
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    let actions = ActionRegistry::builtin();
    let secrets = extract_secrets(&mut definition, None, |action| {
        secret_fields(&actions, action)
    });
    let now = current_time();
    let automation = Automation {
        id: Uuid::now_v7(),
        key,
        name,
        description,
        record_type,
        trigger,
        definition,
        status: AutomationStatus::Draft,
        version: 0,
        created_by: Some(ctx.scope.user_id().clone()),
        updated_by: Some(ctx.scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let changes = diff_changes(
        &Value::Null,
        &AutomationSnapshot::from(&automation),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    store.insert(&mut tx, &automation).await?;
    store_secrets(ctx, &mut tx, automation.id, secrets).await?;
    record(
        ctx,
        &mut tx,
        AUTOMATION_CREATED,
        &automation,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(automation)
}

async fn store_secrets(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    automation_id: Uuid,
    changes: Vec<SecretChange>,
) -> Result<(), AppError> {
    let secrets = ctx.secrets()?;
    for change in changes {
        let key = secret_key(automation_id, &change.step_id, &change.field);
        match change.value {
            Some(value) => {
                secrets
                    .store(tx, ctx.scope, &key, &Secret::new(value))
                    .await?
            }
            None => secrets.remove(tx, ctx.scope, &key).await?,
        }
    }
    Ok(())
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateAutomationCommand,
) -> Result<Automation, AppError> {
    let current = get(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut updated = current.clone();
    if let Some(name) = &command.name {
        updated.name = name.trim().to_owned();
    }
    if let Some(description) = command.description {
        updated.description = normalize_description(description);
    }
    if let Some(trigger) = &command.trigger {
        updated.trigger = parse_trigger(trigger).map_err(|violation| invalid(vec![violation]))?;
    }
    if let Some(definition) = &command.definition {
        updated.definition =
            parse_definition(definition).map_err(|violation| invalid(vec![violation]))?;
    }
    let mut violations = Vec::new();
    check_texts(
        &updated.name,
        updated.description.as_deref(),
        &mut violations,
    );
    violations.extend(
        validate(
            ctx,
            &updated.record_type,
            &updated.trigger,
            &updated.definition,
            ValidationLevel::Save,
        )
        .await?,
    );
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    let actions = ActionRegistry::builtin();
    let secrets = extract_secrets(
        &mut updated.definition,
        Some(&current.definition),
        |action| secret_fields(&actions, action),
    );
    updated.updated_by = Some(ctx.scope.user_id().clone());
    updated.updated_at = current_time();
    let changes = diff_changes(
        &AutomationSnapshot::from(&current),
        &AutomationSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() && secrets.is_empty() {
        return Ok(current);
    }
    let mut tx = ctx.begin().await?;
    AutomationStore::of(ctx).update(&mut tx, &updated).await?;
    store_secrets(ctx, &mut tx, updated.id, secrets).await?;
    record(
        ctx,
        &mut tx,
        AUTOMATION_UPDATED,
        &updated,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(updated)
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let current = get(ctx, id).await?;
    let changes = diff_changes(
        &AutomationSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ctx.secrets()?
        .remove_owner(
            &mut tx,
            ctx.scope,
            SECRET_OWNER_TYPE,
            &current.id.to_string(),
        )
        .await?;
    AutomationStore::of(ctx).delete(&mut tx, current.id).await?;
    record(ctx, &mut tx, AUTOMATION_DELETED, &current, changes, ip).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn publish(
    ctx: &Ctx<'_>,
    id: &str,
    expected_updated_at: Option<DateTime<Utc>>,
    ip: Option<String>,
) -> Result<Automation, AppError> {
    let current = get(ctx, id).await?;
    ensure_version(expected_updated_at, current.updated_at)?;
    let violations = validate(
        ctx,
        &current.record_type,
        &current.trigger,
        &current.definition,
        ValidationLevel::Publish,
    )
    .await?;
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    let now = current_time();
    let mut updated = current.clone();
    updated.version = current.version + 1;
    updated.status = AutomationStatus::Active;
    updated.updated_by = Some(ctx.scope.user_id().clone());
    updated.updated_at = now;
    let version = AutomationVersion {
        id: Uuid::now_v7(),
        automation_id: current.id,
        version: updated.version,
        name: updated.name.clone(),
        record_type: updated.record_type.clone(),
        trigger: updated.trigger.clone(),
        definition: updated.definition.clone(),
        published_by: Some(ctx.scope.user_id().clone()),
        published_at: now,
    };
    let changes = vec![
        FieldChange::new("version", json!(current.version), json!(updated.version)),
        FieldChange::new(
            "status",
            json!(current.status.as_str()),
            json!(updated.status.as_str()),
        ),
    ];
    let store = AutomationStore::of(ctx);
    let mut tx = ctx.begin().await?;
    store.insert_version(&mut tx, &version).await?;
    store.update(&mut tx, &updated).await?;
    if version.definition.options().cancel_waiting_on_publish {
        cancel_waiting(ctx, &mut tx, current.id, SkipReason::VersionPublished).await?;
    }
    record(ctx, &mut tx, AUTOMATION_PUBLISHED, &updated, changes, ip).await?;
    tx.commit().await?;
    Ok(updated)
}

pub(crate) async fn set_active(
    ctx: &Ctx<'_>,
    id: &str,
    active: bool,
    ip: Option<String>,
) -> Result<Automation, AppError> {
    let current = get(ctx, id).await?;
    if current.version == 0 {
        return Err(AppError::conflict_code(
            AUTOMATION_NOT_PUBLISHED,
            "publish the automation before activating or pausing it",
        ));
    }
    let status = if active {
        AutomationStatus::Active
    } else {
        AutomationStatus::Paused
    };
    if current.status == status {
        return Ok(current);
    }
    let mut updated = current.clone();
    updated.status = status;
    updated.updated_by = Some(ctx.scope.user_id().clone());
    updated.updated_at = current_time();
    let changes = vec![FieldChange::new(
        "status",
        json!(current.status.as_str()),
        json!(status.as_str()),
    )];
    let action = if active {
        AUTOMATION_ACTIVATED
    } else {
        AUTOMATION_PAUSED
    };
    let store = AutomationStore::of(ctx);
    let cancel_on_pause = !active
        && store
            .find_version(current.id, current.version)
            .await?
            .is_none_or(|version| version.definition.options().cancel_waiting_on_pause);
    let mut tx = ctx.begin().await?;
    store.update(&mut tx, &updated).await?;
    if cancel_on_pause {
        cancel_waiting(ctx, &mut tx, current.id, SkipReason::AutomationPaused).await?;
    }
    record(ctx, &mut tx, action, &updated, changes, ip).await?;
    tx.commit().await?;
    Ok(updated)
}

pub(crate) async fn dry_run(
    ctx: &Ctx<'_>,
    id: &str,
    command: DryRunCommand,
) -> Result<DryRunResult, AppError> {
    let automation = get(ctx, id).await?;
    let (definition, trigger, version) = if command.use_draft || automation.version == 0 {
        (
            automation.definition.clone(),
            automation.trigger.clone(),
            None,
        )
    } else {
        let version = AutomationStore::of(ctx)
            .find_version(automation.id, automation.version)
            .await?
            .ok_or_else(automation_not_found)?;
        (version.definition, version.trigger, Some(version.version))
    };
    let record_id = parse_id(&command.record_id).ok_or_else(|| {
        AppError::invalid_fields(
            "invalid dry run",
            vec![FieldViolation::new("record_id", "must be a UUID")],
        )
    })?;
    let record = RecordRef::parse(&automation.record_type, &record_id.to_string())?;
    let scope = ctx.scope;
    let gateway = ctx.automation_gateways().gateway(None);
    let fields = gateway.fields(scope, &automation.record_type).await?;
    let record_data = gateway.load_record(scope, &record).await?;
    let record_available = record_data.is_some();
    let record_data = record_data.unwrap_or_else(|| json!({ "id": record_id.to_string() }));
    let mut context = TriggerContext::new(trigger.kind.clone());
    context.actor_type = Some(Actor::USER.to_owned());
    context.actor_id = Some(scope.user_id().as_str().to_owned());
    context.occurred_at = Some(current_time());
    context.changed_fields = command.changed_fields;
    context.previous = command.previous;
    let run_id = Uuid::now_v7();
    let templates = template_scope(
        &automation,
        run_id,
        0,
        &record_data,
        &context,
        &fields,
        gateway.timezone().await?,
        current_time(),
    );
    let actions = ActionRegistry::builtin();
    let input = EngineInput {
        ctx,
        gateway: gateway.as_ref(),
        actions: &actions,
        clock: automation_clock(),
        definition: &definition,
        record: &record,
        record_data: &record_data,
        record_available,
        fields: &fields,
        templates: &templates,
        automation_id: automation.id,
        run_id,
        mode: EngineMode::DryRun,
        previous: &[],
    };
    let (outcome, steps) = input.run(&DiscardSteps).await?;
    Ok(DryRunResult {
        version,
        record,
        record_available,
        outcome,
        steps,
    })
}

pub(crate) async fn run_now(
    ctx: &Ctx<'_>,
    id: &str,
    record_id: &str,
) -> Result<AutomationRun, AppError> {
    let automation = get(ctx, id).await?;
    if automation.version == 0 {
        return Err(AppError::conflict_code(
            AUTOMATION_NOT_PUBLISHED,
            "publish the automation before running it",
        ));
    }
    if automation.status != AutomationStatus::Active {
        return Err(AppError::conflict_code(
            AUTOMATION_NOT_ACTIVE,
            "activate the automation before running it",
        ));
    }
    let version = AutomationStore::of(ctx)
        .find_version(automation.id, automation.version)
        .await?
        .ok_or_else(automation_not_found)?;
    if version.trigger.kind != MANUAL {
        return Err(AppError::conflict_code(
            AUTOMATION_NOT_MANUAL,
            "only automations with the manual trigger can be run on demand; use the dry run to test the others",
        ));
    }
    let record_id = parse_id(record_id).ok_or_else(|| {
        AppError::invalid_fields(
            "invalid run",
            vec![FieldViolation::new("record_id", "must be a UUID")],
        )
    })?;
    let record = RecordRef::parse(&automation.record_type, &record_id.to_string())?;
    let gateway = ctx.automation_gateways().gateway(None);
    if gateway.load_record(ctx.scope, &record).await?.is_none() {
        return Err(AppError::invalid_fields(
            "invalid run",
            vec![FieldViolation::new("record_id", "record was not found")],
        ));
    }
    let now = current_time();
    let mut context = TriggerContext::new(MANUAL);
    context.actor_type = Some(Actor::USER.to_owned());
    context.actor_id = Some(ctx.scope.user_id().as_str().to_owned());
    context.occurred_at = Some(now);
    let trigger_key = format!("manual:{}", Uuid::now_v7());
    let run = AutomationRun {
        id: Uuid::now_v7(),
        automation_id: automation.id,
        version: version.version,
        trigger_key: trigger_key.clone(),
        trigger_event_id: None,
        trigger_type: MANUAL.to_owned(),
        record: record.clone(),
        status: RunStatus::Running,
        depth: 0,
        chain: Vec::new(),
        reason: None,
        error: None,
        attempts: 0,
        started_at: now,
        finished_at: None,
        resume_at: None,
        waiting_step_id: None,
        context: Some(context.clone()),
    };
    if let RunInsert::Existing(existing) = AutomationRunStore::of(ctx).insert_run(&run).await? {
        return Ok(*existing);
    }
    let payload = RunJobPayload {
        automation_id: automation.id,
        version: version.version,
        trigger_key,
        record_type: automation.record_type.clone(),
        record_id,
        context,
        causation: None,
    };
    ctx.jobs().enqueue_now(run_job(&payload)?).await?;
    Ok(run)
}

pub(crate) async fn runs(
    ctx: &Ctx<'_>,
    id: &str,
    filter: RunListFilter,
    window: ListWindow,
) -> Result<Listing<AutomationRun>, AppError> {
    let automation = get(ctx, id).await?;
    AutomationRunStore::of(ctx)
        .list_runs(automation.id, &filter, window)
        .await
}

pub(crate) async fn run_details(
    ctx: &Ctx<'_>,
    id: &str,
    run_id: &str,
) -> Result<RunDetails, AppError> {
    let automation = get(ctx, id).await?;
    let run_id = parse_id(run_id).ok_or_else(run_not_found)?;
    let runs = AutomationRunStore::of(ctx);
    let run = runs
        .find_run(run_id)
        .await?
        .filter(|run| run.automation_id == automation.id)
        .ok_or_else(run_not_found)?;
    let steps = runs.steps(run.id).await?;
    Ok(RunDetails { run, steps })
}

async fn record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    action: &str,
    automation: &Automation,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let now = current_time();
    let record = RecordRef::new(AUTOMATION_RECORD_TYPE, automation.id);
    let event = DomainEvent::new(actor.clone(), action, now)
        .with_record(record.clone())
        .with_payload(json!({
            "key": automation.key,
            "name": automation.name,
            "record_type": automation.record_type,
            "trigger_type": automation.trigger.kind,
            "status": automation.status.as_str(),
            "version": automation.version,
        }));
    let entry = AuditEntry::new(actor, action, now)
        .with_record(record)
        .with_changes(changes)
        .with_ip(ip);
    ctx.events().publish(tx, event).await?;
    ctx.audit().record(tx, entry).await
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn template_scope(
    automation: &Automation,
    run_id: Uuid,
    depth: u32,
    record_data: &Value,
    context: &TriggerContext,
    fields: &aphel_sdk::platform::application::fields::FieldSet,
    timezone: chrono_tz::Tz,
    now: DateTime<Utc>,
) -> TemplateScope {
    TemplateScope {
        record: record_data.clone(),
        trigger: context.json(),
        automation: json!({
            "id": automation.id.to_string(),
            "key": automation.key,
            "name": automation.name,
        }),
        run: json!({ "id": run_id.to_string(), "depth": depth }),
        now,
        timezone,
        sensitive_fields: fields
            .active()
            .filter(|field| field.definition.is_sensitive)
            .map(|field| field.key().to_owned())
            .collect::<BTreeSet<_>>(),
    }
}
