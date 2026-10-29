use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    EXPORT_FILE_TTL_DAYS, MAX_POLICIES, MAX_THRESHOLD_DAYS, MIN_THRESHOLD_DAYS,
    PRIVACY_EXPORT_EXPIRED, PRIVACY_REQUEST_RECORD_TYPE, PRIVACY_RETENTION_RUN_JOB,
    RETENTION_POLICY_CREATED, RETENTION_POLICY_DELETED, RETENTION_POLICY_DISABLED,
    RETENTION_POLICY_ENABLED, RETENTION_POLICY_LIMIT, RETENTION_POLICY_RECORD_TYPE,
    RETENTION_POLICY_UPDATED, RETENTION_PREVIEW_REQUIRED, RETENTION_RUN_COMPLETED,
    RETENTION_RUN_FAILED, RETENTION_RUN_RECORD_TYPE, RETENTION_RUN_REQUESTED, RetentionAction,
    RetentionPolicy, RetentionPolicyCommand, RetentionPreview, RetentionRun, RetentionRunFilter,
    RetentionRunStatus, RetentionTrigger, StoredFileRef, retention_policy_not_found,
    supports_retention,
};
use crate::service::erasure::erased_event;
use crate::service::{Mutation, definition, publish, remove_stored_files, require_privacy};
use crate::store::{PrivacyStore, RetentionStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{CustomFieldWriter, RecordTypeDefinition};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, EnqueueOutcome, FieldChange, FileStorage, Job, JobQueue, RecordRef,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};
use trash::{TrashStore, is_trashable};

pub(crate) const RETENTION_BATCH_SIZE: u32 = 200;
pub(crate) const LOG_PURGE_BATCH_SIZE: u32 = 5_000;
pub(crate) const MAX_NAME_LENGTH: usize = 140;
pub(crate) const MIN_HOURS_BETWEEN_SCHEDULED_RUNS: i64 = 20;
pub(crate) const RUN_STALE_MINUTES: i64 = 30;
pub(crate) const RUN_ABANDON_HOURS: i64 = 24;
const RUN_FAILURE: &str = "the retention run could not be completed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RetentionSummary {
    pub(crate) runs: usize,
    pub(crate) expired_exports: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SweepSummary {
    pub(crate) resumed: usize,
    pub(crate) abandoned: usize,
}

fn run_job(run_id: Uuid) -> Job {
    Job::new(
        PRIVACY_RETENTION_RUN_JOB,
        RunRetentionRequest { run_id }.payload(),
    )
    .with_max_attempts(2)
    .with_unique_key(format!("retention_run:{run_id}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunRetentionRequest {
    pub(crate) run_id: Uuid,
}

impl RunRetentionRequest {
    pub(crate) fn payload(&self) -> Value {
        json!({
            "run_id": self.run_id.to_string(),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::Internal(format!("retention job payload misses {key}")))
        };
        Ok(Self {
            run_id: Uuid::parse_str(text("run_id")?)
                .map_err(|_| AppError::Internal("invalid retention run id".to_owned()))?,
        })
    }
}

fn policy_json(policy: &RetentionPolicy) -> Value {
    json!({
        "name": policy.name,
        "action": policy.action.as_str(),
        "record_type": policy.record_type,
        "threshold_days": policy.threshold_days,
        "is_enabled": policy.is_enabled,
    })
}

fn parse_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| retention_policy_not_found())
}

fn clean_name(name: Option<&str>) -> Option<String> {
    name.map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn validate_policy(
    ctx: &Ctx<'_>,
    name: &str,
    action: RetentionAction,
    record_type: Option<&str>,
    threshold_days: u32,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_NAME_LENGTH} characters"),
        ));
    }
    match (action.needs_record_type(), record_type) {
        (true, Some(record_type))
            if ctx
                .registry()
                .require(record_type)
                .is_ok_and(supports_retention)
                && ctx.registry().handler(record_type).is_ok() => {}
        (true, _) => violations.push(FieldViolation::new(
            "record_type",
            "must be a record type that can be archived and deleted for this action",
        )),
        (false, Some(_)) => violations.push(FieldViolation::new(
            "record_type",
            "is not accepted for this action",
        )),
        (false, None) => {}
    }
    if !(MIN_THRESHOLD_DAYS..=MAX_THRESHOLD_DAYS).contains(&threshold_days) {
        violations.push(FieldViolation::new(
            "threshold_days",
            format!("must be between {MIN_THRESHOLD_DAYS} and {MAX_THRESHOLD_DAYS}"),
        ));
    }
    violations
}

fn invalid_policy(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid retention policy", violations)
}

fn preview_required() -> AppError {
    AppError::conflict_code(
        RETENTION_PREVIEW_REQUIRED,
        "preview the policy (less than an hour ago and after its last change) before enabling or running it",
    )
}

pub(crate) async fn list_policies(ctx: &Ctx<'_>) -> Result<Vec<RetentionPolicy>, AppError> {
    require_privacy(ctx.scope)?;
    RetentionStore::of(ctx).list_policies().await
}

async fn require_policy(ctx: &Ctx<'_>, policy_id: Uuid) -> Result<RetentionPolicy, AppError> {
    RetentionStore::of(ctx)
        .find_policy(policy_id)
        .await?
        .ok_or_else(retention_policy_not_found)
}

fn policy_definition(
    ctx: &Ctx<'_>,
    policy_record_type: Option<&str>,
) -> Result<Option<RecordTypeDefinition>, AppError> {
    policy_record_type
        .map(|record_type| definition(ctx, record_type))
        .transpose()
}

fn user_reference(ctx: &Ctx<'_>) -> Option<UserReference> {
    Some(UserReference {
        id: ctx.scope.user_id().clone(),
        display_name: String::new(),
    })
}

pub(crate) async fn create_policy(
    ctx: &Ctx<'_>,
    command: RetentionPolicyCommand,
) -> Result<RetentionPolicy, AppError> {
    require_privacy(ctx.scope)?;
    let name = clean_name(command.name.as_deref()).unwrap_or_default();
    let action = command
        .action
        .as_deref()
        .and_then(RetentionAction::parse)
        .ok_or_else(|| {
            invalid_policy(vec![FieldViolation::new(
                "action",
                "must be archive_inactive, purge_archived, purge_audit_logs, purge_security_events, purge_domain_events or purge_orphan_files",
            )])
        })?;
    let record_type = command
        .record_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let threshold_days = command.threshold_days.unwrap_or(0);
    let violations = validate_policy(ctx, &name, action, record_type.as_deref(), threshold_days);
    if !violations.is_empty() {
        return Err(invalid_policy(violations));
    }
    let store = RetentionStore::of(ctx);
    if store.count_policies().await? >= MAX_POLICIES {
        return Err(AppError::validation_code(
            RETENTION_POLICY_LIMIT,
            format!("an organization can have at most {MAX_POLICIES} retention policies"),
        ));
    }
    let now = ctx.now();
    let policy = RetentionPolicy {
        id: Uuid::now_v7(),
        name,
        action,
        record_type,
        threshold_days,
        is_enabled: false,
        previewed_count: None,
        previewed_at: None,
        last_run_at: None,
        created_by: user_reference(ctx),
        created_at: now,
        updated_at: now,
    };
    let mut tx = ctx.begin().await?;
    store.insert_policy(&mut tx, &policy).await?;
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: RETENTION_POLICY_CREATED.into(),
            record: RecordRef::new(RETENTION_POLICY_RECORD_TYPE, policy.id),
            payload: policy_json(&policy),
            changes: vec![FieldChange::new(
                "policy",
                Value::Null,
                policy_json(&policy),
            )],
            now,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_policy(ctx, policy.id).await
}

pub(crate) async fn update_policy(
    ctx: &Ctx<'_>,
    policy_id: &str,
    command: RetentionPolicyCommand,
) -> Result<RetentionPolicy, AppError> {
    require_privacy(ctx.scope)?;
    let current = require_policy(ctx, parse_id(policy_id)?).await?;
    let mut policy = current.clone();
    if let Some(name) = command.name.as_deref() {
        policy.name = name.trim().to_owned();
    }
    if let Some(threshold_days) = command.threshold_days {
        policy.threshold_days = threshold_days;
    }
    let violations = validate_policy(
        ctx,
        &policy.name,
        policy.action,
        policy.record_type.as_deref(),
        policy.threshold_days,
    );
    if !violations.is_empty() {
        return Err(invalid_policy(violations));
    }
    if policy == current {
        return Ok(current);
    }
    let now = ctx.now();
    if policy.threshold_days != current.threshold_days {
        policy.is_enabled = false;
        policy.previewed_at = None;
        policy.previewed_count = None;
    }
    policy.updated_at = now;
    let mut tx = ctx.begin().await?;
    RetentionStore::of(ctx)
        .save_policy(&mut tx, &policy)
        .await?;
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: RETENTION_POLICY_UPDATED.into(),
            record: RecordRef::new(RETENTION_POLICY_RECORD_TYPE, policy.id),
            payload: policy_json(&policy),
            changes: vec![FieldChange::new(
                "policy",
                policy_json(&current),
                policy_json(&policy),
            )],
            now,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_policy(ctx, policy.id).await
}

pub(crate) async fn delete_policy(
    ctx: &Ctx<'_>,
    policy_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    require_privacy(ctx.scope)?;
    let policy = require_policy(ctx, parse_id(policy_id)?).await?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if !RetentionStore::of(ctx)
        .delete_policy(&mut tx, policy.id)
        .await?
    {
        return Err(retention_policy_not_found());
    }
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: RETENTION_POLICY_DELETED.into(),
            record: RecordRef::new(RETENTION_POLICY_RECORD_TYPE, policy.id),
            payload: policy_json(&policy),
            changes: vec![FieldChange::new(
                "policy",
                policy_json(&policy),
                Value::Null,
            )],
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn preview_policy(
    ctx: &Ctx<'_>,
    policy_id: &str,
) -> Result<RetentionPreview, AppError> {
    require_privacy(ctx.scope)?;
    let policy = require_policy(ctx, parse_id(policy_id)?).await?;
    let now = ctx.now();
    let threshold_at = policy.threshold(now);
    let definition = policy_definition(ctx, policy.record_type.as_deref())?;
    let store = RetentionStore::of(ctx);
    let affected = store
        .count_candidates(policy.action, definition.as_ref(), threshold_at)
        .await?;
    store.save_preview(policy.id, affected, now).await?;
    Ok(RetentionPreview {
        affected,
        threshold_at,
        previewed_at: now,
    })
}

pub(crate) async fn set_policy_enabled(
    ctx: &Ctx<'_>,
    policy_id: &str,
    enabled: bool,
    ip: Option<String>,
) -> Result<RetentionPolicy, AppError> {
    require_privacy(ctx.scope)?;
    let current = require_policy(ctx, parse_id(policy_id)?).await?;
    if current.is_enabled == enabled {
        return Ok(current);
    }
    let now = ctx.now();
    if enabled && !current.has_fresh_preview(now) {
        return Err(preview_required());
    }
    let mut policy = current.clone();
    policy.is_enabled = enabled;
    let mut tx = ctx.begin().await?;
    RetentionStore::of(ctx)
        .save_policy(&mut tx, &policy)
        .await?;
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: if enabled {
                RETENTION_POLICY_ENABLED.into()
            } else {
                RETENTION_POLICY_DISABLED.into()
            },
            record: RecordRef::new(RETENTION_POLICY_RECORD_TYPE, policy.id),
            payload: json!({
                "name": policy.name,
                "action": policy.action.as_str(),
                "previewed_count": policy.previewed_count,
            }),
            changes: vec![
                FieldChange::new("is_enabled", json!(current.is_enabled), json!(enabled)),
                FieldChange::new(
                    "previewed_count",
                    Value::Null,
                    json!(policy.previewed_count),
                ),
            ],
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_policy(ctx, policy.id).await
}

pub(crate) async fn request_run(
    ctx: &Ctx<'_>,
    policy_id: &str,
    ip: Option<String>,
) -> Result<RetentionRun, AppError> {
    require_privacy(ctx.scope)?;
    let policy = require_policy(ctx, parse_id(policy_id)?).await?;
    let now = ctx.now();
    if !policy.has_fresh_preview(now) {
        return Err(preview_required());
    }
    let run = RetentionRun::for_policy(&policy, RetentionTrigger::Manual, user_reference(ctx), now);
    let store = RetentionStore::of(ctx);
    let mut tx = ctx.begin().await?;
    store.insert_run(&mut tx, &run).await?;
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: RETENTION_RUN_REQUESTED.into(),
            record: RecordRef::new(RETENTION_RUN_RECORD_TYPE, run.id),
            payload: json!({
                "run_id": run.id.to_string(),
                "policy_id": policy.id.to_string(),
                "action": policy.action.as_str(),
                "previewed_count": policy.previewed_count,
            }),
            changes: vec![FieldChange::new(
                "policy",
                Value::Null,
                policy_json(&policy),
            )],
            now,
            ip,
        },
    )
    .await?;
    ctx.jobs().enqueue(&mut tx, run_job(run.id)).await?;
    tx.commit().await?;
    store
        .find_run(run.id)
        .await?
        .ok_or_else(|| AppError::Internal("the retention run was not stored".to_owned()))
}

pub(crate) async fn list_runs(
    ctx: &Ctx<'_>,
    filter: RetentionRunFilter,
    window: ListWindow,
) -> Result<Listing<RetentionRun>, AppError> {
    require_privacy(ctx.scope)?;
    RetentionStore::of(ctx).list_runs(&filter, window).await
}

pub(crate) async fn run_queued(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    request: &RunRetentionRequest,
) -> Result<(), AppError> {
    let scope = AccessScope::system();
    let ctx = &ctx.with_scope(&scope);
    let store = RetentionStore::of(ctx);
    let Some(mut run) = store.find_run(request.run_id).await? else {
        return Ok(());
    };
    if run.status.is_finished() {
        return Ok(());
    }
    let policy = match run.policy_id {
        Some(policy_id) => store.find_policy(policy_id).await?,
        None => None,
    };
    let Some(policy) = policy else {
        run.status = RetentionRunStatus::Running;
        return finish_run(ctx, None, run, Some("the policy was deleted".to_owned())).await;
    };
    execute_run(ctx, storage, policy, run).await
}

pub(crate) async fn fail_queued_run(
    ctx: &Ctx<'_>,
    request: &RunRetentionRequest,
) -> Result<(), AppError> {
    let scope = AccessScope::system();
    let ctx = &ctx.with_scope(&scope);
    let Some(mut run) = RetentionStore::of(ctx).find_run(request.run_id).await? else {
        return Ok(());
    };
    if run.status.is_finished() {
        return Ok(());
    }
    run.status = RetentionRunStatus::Running;
    finish_run(ctx, None, run, Some(RUN_FAILURE.to_owned())).await
}

pub(crate) async fn run_scheduled(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
) -> Result<RetentionSummary, AppError> {
    let mut summary = RetentionSummary::default();
    let scope = AccessScope::system();
    let ctx = &ctx.with_scope(&scope);
    let store = RetentionStore::of(ctx);
    let now = ctx.now();
    for policy in store.list_policies().await? {
        let recent = policy
            .last_run_at
            .is_some_and(|last| now - last < Duration::hours(MIN_HOURS_BETWEEN_SCHEDULED_RUNS));
        if !policy.is_enabled || recent || store.has_unfinished_run(policy.id).await? {
            continue;
        }
        let run = RetentionRun::for_policy(&policy, RetentionTrigger::Schedule, None, now);
        let mut tx = ctx.begin().await?;
        store.insert_run(&mut tx, &run).await?;
        ctx.jobs().enqueue(&mut tx, run_job(run.id)).await?;
        tx.commit().await?;
        summary.runs += 1;
    }
    summary.expired_exports = expire_exports(ctx, storage).await?;
    Ok(summary)
}

pub(crate) async fn sweep_stale_runs(ctx: &Ctx<'_>) -> Result<SweepSummary, AppError> {
    let scope = AccessScope::system();
    let ctx = &ctx.with_scope(&scope);
    let store = RetentionStore::of(ctx);
    let now = ctx.now();
    let mut summary = SweepSummary::default();
    for mut run in store
        .stale_runs(now - Duration::minutes(RUN_STALE_MINUTES))
        .await?
    {
        if now - run.created_at > Duration::hours(RUN_ABANDON_HOURS) {
            run.status = RetentionRunStatus::Running;
            finish_run(ctx, None, run, Some(RUN_FAILURE.to_owned())).await?;
            summary.abandoned += 1;
            continue;
        }
        store.touch_run(run.id).await?;
        let mut tx = ctx.begin().await?;
        let outcome = ctx.jobs().enqueue(&mut tx, run_job(run.id)).await?;
        tx.commit().await?;
        if matches!(outcome, EnqueueOutcome::Enqueued(_)) {
            summary.resumed += 1;
        }
    }
    Ok(summary)
}

async fn checkpoint(ctx: &Ctx<'_>, run: &RetentionRun) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    RetentionStore::of(ctx).save_run(&mut tx, run).await?;
    tx.commit().await?;
    Ok(())
}

async fn execute_run(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    policy: RetentionPolicy,
    mut run: RetentionRun,
) -> Result<(), AppError> {
    let now = ctx.now();
    run.status = RetentionRunStatus::Running;
    run.started_at.get_or_insert(now);
    checkpoint(ctx, &run).await?;
    let threshold = policy.threshold(now);
    let failure = match apply_policy(ctx, storage, &policy, &mut run, threshold).await {
        Ok(()) => None,
        Err(error) => {
            tracing::error!(error = %error, "retention policy execution failed");
            Some(RUN_FAILURE.to_owned())
        }
    };
    finish_run(ctx, Some(policy), run, failure).await
}

async fn apply_policy(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    policy: &RetentionPolicy,
    run: &mut RetentionRun,
    threshold: DateTime<Utc>,
) -> Result<(), AppError> {
    let store = RetentionStore::of(ctx);
    match policy.action {
        RetentionAction::ArchiveInactive | RetentionAction::PurgeArchived => {
            let definition = policy_definition(ctx, policy.record_type.as_deref())?
                .ok_or_else(|| AppError::Internal("policy without record type".to_owned()))?;
            loop {
                let candidates = store
                    .record_candidates(policy.action, &definition, threshold, RETENTION_BATCH_SIZE)
                    .await?;
                let mut progressed = false;
                for (record_id, title) in &candidates {
                    let outcome = if policy.action == RetentionAction::ArchiveInactive {
                        archive_for_retention(ctx, &definition, *record_id, title, run)
                            .await
                            .map(|archived| (archived, 0))
                    } else {
                        purge_for_retention(ctx, storage, &definition, *record_id, run).await
                    };
                    match outcome {
                        Ok((true, files)) => {
                            run.affected += 1;
                            run.files_removed += files;
                            progressed = true;
                        }
                        Ok((false, _)) => {}
                        Err(error) => {
                            tracing::warn!(error = %error, "a record could not be processed by a retention policy");
                            run.errors += 1;
                        }
                    }
                }
                checkpoint(ctx, run).await?;
                if candidates.len() < RETENTION_BATCH_SIZE as usize || !progressed {
                    break;
                }
            }
        }
        RetentionAction::PurgeAuditLogs
        | RetentionAction::PurgeSecurityEvents
        | RetentionAction::PurgeDomainEvents => loop {
            let removed = store
                .purge_log_batch(policy.action, threshold, LOG_PURGE_BATCH_SIZE)
                .await?;
            run.affected += removed;
            checkpoint(ctx, run).await?;
            if removed < u64::from(LOG_PURGE_BATCH_SIZE) {
                break;
            }
        },
        RetentionAction::PurgeOrphanFiles => loop {
            let files = store.orphan_files(threshold, RETENTION_BATCH_SIZE).await?;
            if files.is_empty() {
                break;
            }
            let ids: Vec<Uuid> = files.iter().map(|file| file.file_id).collect();
            let mut tx = ctx.begin().await?;
            PrivacyStore::of(ctx)
                .delete_file_rows(&mut tx, &ids)
                .await?;
            tx.commit().await?;
            let keys: Vec<String> = files.into_iter().map(|file| file.storage_key).collect();
            run.affected += ids.len() as u64;
            run.files_removed += remove_stored_files(storage, &keys).await;
            checkpoint(ctx, run).await?;
            if ids.len() < RETENTION_BATCH_SIZE as usize {
                break;
            }
        },
    }
    Ok(())
}

async fn archive_for_retention(
    ctx: &Ctx<'_>,
    definition: &RecordTypeDefinition,
    record_id: Uuid,
    title: &str,
    run: &RetentionRun,
) -> Result<bool, AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if !RetentionStore::of(ctx)
        .archive_record(&mut tx, definition, record_id, now)
        .await?
    {
        return Ok(false);
    }
    let event_type = format!("{}.archived", definition.record_type.as_str());
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::System,
            event_type: event_type.into(),
            record: RecordRef::new(definition.record_type.clone(), record_id),
            payload: json!({
                "name": title,
                "retention_run_id": run.id.to_string(),
                "retention_policy_id": run.policy_id.map(|id| id.to_string()),
            }),
            changes: vec![FieldChange::new(
                "archived_at",
                Value::Null,
                json!(now.to_rfc3339()),
            )],
            now,
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(true)
}

async fn purge_for_retention(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    definition: &RecordTypeDefinition,
    record_id: Uuid,
    run: &RetentionRun,
) -> Result<(bool, u64), AppError> {
    if !is_trashable(definition) {
        return Ok((false, 0));
    }
    let record = RecordRef::new(definition.record_type.clone(), record_id);
    let now = ctx.now();
    let privacy = PrivacyStore::of(ctx);
    let fields = ctx.fields();
    let mut tx = ctx.begin().await?;
    let purged = CustomFieldWriter::new(&fields)
        .purge_record(&mut tx, ctx.scope, &record)
        .await?;
    privacy
        .mark_deleted(&mut tx, definition, record_id, now)
        .await?;
    if !TrashStore::of(ctx)
        .purge(&mut tx, definition, record_id)
        .await?
    {
        return Ok((false, 0));
    }
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::System,
            event_type: erased_event(definition.record_type.as_str()).into(),
            record,
            payload: json!({
                "retention_run_id": run.id.to_string(),
                "retention_policy_id": run.policy_id.map(|id| id.to_string()),
            }),
            changes: vec![FieldChange::redacted("record")],
            now,
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok((
        true,
        remove_stored_files(storage, &purged.orphan_storage_keys).await,
    ))
}

async fn finish_run(
    ctx: &Ctx<'_>,
    policy: Option<RetentionPolicy>,
    mut run: RetentionRun,
    failure: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    run.status = if failure.is_some() {
        RetentionRunStatus::Failed
    } else {
        RetentionRunStatus::Completed
    };
    run.error_message = failure.or_else(|| {
        (run.errors > 0).then(|| format!("{} items could not be processed", run.errors))
    });
    run.finished_at = Some(now);
    let store = RetentionStore::of(ctx);
    let mut tx = ctx.begin().await?;
    store.save_run(&mut tx, &run).await?;
    if let Some(policy) = policy {
        store.mark_policy_run(&mut tx, policy.id, now).await?;
    }
    let requested_by = run
        .requested_by
        .as_ref()
        .map(|user| user.id.as_str().to_owned());
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::System,
            event_type: if run.status == RetentionRunStatus::Completed {
                RETENTION_RUN_COMPLETED.into()
            } else {
                RETENTION_RUN_FAILED.into()
            },
            record: RecordRef::new(RETENTION_RUN_RECORD_TYPE, run.id),
            payload: json!({
                "run_id": run.id.to_string(),
                "policy_id": run.policy_id.map(|id| id.to_string()),
                "action": run.action.as_str(),
                "record_type": run.record_type,
                "trigger": run.trigger.as_str(),
                "affected": run.affected,
                "files_removed": run.files_removed,
                "errors": run.errors,
                "requested_by": requested_by,
            }),
            changes: vec![
                FieldChange::new("status", Value::Null, json!(run.status.as_str())),
                FieldChange::new("affected", Value::Null, json!(run.affected)),
                FieldChange::new("files_removed", Value::Null, json!(run.files_removed)),
                FieldChange::new("errors", Value::Null, json!(run.errors)),
            ],
            now,
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn expire_exports(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
) -> Result<usize, AppError> {
    let threshold = ctx.now() - Duration::days(EXPORT_FILE_TTL_DAYS);
    let mut expired = 0;
    let scope = AccessScope::system();
    let ctx = &ctx.with_scope(&scope);
    let privacy = PrivacyStore::of(ctx);
    loop {
        let batch = privacy
            .expired_exports(threshold, RETENTION_BATCH_SIZE)
            .await?;
        let mut progressed = false;
        for request_id in &batch {
            let Some(request) = privacy.find_request(*request_id).await? else {
                continue;
            };
            let Some(file_id) = request.file_id else {
                continue;
            };
            let file: Option<StoredFileRef> = privacy.file_ref(file_id).await?;
            let now = ctx.now();
            let mut tx = ctx.begin().await?;
            privacy.delete_file_rows(&mut tx, &[file_id]).await?;
            publish(
                ctx,
                &mut tx,
                Mutation {
                    actor: Actor::System,
                    event_type: PRIVACY_EXPORT_EXPIRED.into(),
                    record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
                    payload: json!({ "request_id": request.id.to_string() }),
                    changes: vec![FieldChange::new(
                        "file_id",
                        json!(file_id.to_string()),
                        Value::Null,
                    )],
                    now,
                    ip: None,
                },
            )
            .await?;
            tx.commit().await?;
            if let Some(file) = file {
                remove_stored_files(storage, &[file.storage_key]).await;
            }
            expired += 1;
            progressed = true;
        }
        if batch.len() < RETENTION_BATCH_SIZE as usize || !progressed {
            break;
        }
    }
    Ok(expired)
}
