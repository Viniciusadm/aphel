use chrono::Duration;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CreatePrivacyRequestCommand, DEFAULT_DUE_DAYS, MAX_DUE_DAYS, MAX_TARGETS,
    PRIVACY_EXPORT_UNAVAILABLE, PRIVACY_REQUEST_CANCELLED, PRIVACY_REQUEST_COMPLETED,
    PRIVACY_REQUEST_CREATED, PRIVACY_REQUEST_DOWNLOADED, PRIVACY_REQUEST_FAILED,
    PRIVACY_REQUEST_NOT_PENDING, PRIVACY_REQUEST_RECORD_TYPE, PRIVACY_REQUEST_RUN_JOB,
    PRIVACY_REQUEST_STARTED, PRIVACY_TARGETS_EMPTY, PrivacyRequest, PrivacyRequestDetail,
    PrivacyRequestFilter, PrivacyRequestKind, PrivacyRequestStatus, RunReport, TargetLabel,
    privacy_request_not_found,
};
use crate::service::subject::parse_subject;
use crate::service::{
    Mutation, acting_scope, audit_only, erasure, package, publish, require_privacy,
};
use crate::store::PrivacyStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::files::{FileRepository, StoredFile, UserReference};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, ByteStream, FieldChange, FileStorage, Job, JobQueue, RecordRef,
};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::infrastructure::storage::MySqlFileRepository;
use aphel_sdk::exports::{GeneratedFile, discard_generated_file, store_generated_file};

pub(crate) const REQUESTER_INACTIVE: &str =
    "the requester is no longer an active member of the organization";
pub(crate) const REQUESTER_NOT_ALLOWED: &str =
    "the requester no longer has the privacy.manage permission";
pub(crate) const GENERIC_FAILURE: &str = "the privacy request could not be completed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunPrivacyRequest {
    pub(crate) request_id: Uuid,
    pub(crate) requested_by: UserId,
}

impl RunPrivacyRequest {
    pub(crate) fn payload(&self) -> Value {
        json!({
            "request_id": self.request_id.to_string(),
            "requested_by": self.requested_by.as_str(),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| {
            payload.get(key).and_then(Value::as_str).ok_or_else(|| {
                AppError::Internal(format!("privacy request job payload misses {key}"))
            })
        };
        Ok(Self {
            request_id: Uuid::parse_str(text("request_id")?).map_err(|_| {
                AppError::Internal("invalid privacy request id in job payload".to_owned())
            })?,
            requested_by: UserId::new(text("requested_by")?)?,
        })
    }
}

pub(crate) struct PrivacyDownload {
    pub(crate) file: StoredFile,
    pub(crate) content: ByteStream,
}

pub(crate) struct ExecutionOutcome {
    pub(crate) report: RunReport,
    pub(crate) file: Option<GeneratedFile>,
}

fn parse_request_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| privacy_request_not_found())
}

fn user_reference(scope: &AccessScope) -> UserReference {
    UserReference {
        id: scope.user_id().clone(),
        display_name: String::new(),
    }
}

pub(crate) fn is_permanent_failure(error: &AppError) -> bool {
    matches!(
        error,
        AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Coded { .. }
            | AppError::Validation(_)
            | AppError::NotFound(_)
            | AppError::Forbidden(_)
    )
}

pub(crate) async fn create_request(
    ctx: &Ctx<'_>,
    command: CreatePrivacyRequestCommand,
) -> Result<PrivacyRequest, AppError> {
    let scope = ctx.scope;
    require_privacy(scope)?;
    let kind = PrivacyRequestKind::parse(&command.kind)?;
    let matcher = parse_subject(&command.term)?;
    let mut violations = Vec::new();
    if command.targets.records.iter().any(|record| {
        !ctx.registry()
            .require(record.record_type.as_str())
            .is_ok_and(|definition| definition.capabilities.has_trait("privacy.personal_data"))
    }) {
        violations.push(FieldViolation::new(
            "targets.records",
            "only record types that keep personal data and can be deleted are supported",
        ));
    }
    if command.targets.len() > MAX_TARGETS {
        violations.push(FieldViolation::new(
            "targets",
            format!("at most {MAX_TARGETS} items per request"),
        ));
    }
    let now = ctx.now();
    let due_at = command
        .due_at
        .unwrap_or_else(|| now + Duration::days(DEFAULT_DUE_DAYS));
    if due_at < now - Duration::days(1) || due_at > now + Duration::days(MAX_DUE_DAYS) {
        violations.push(FieldViolation::new(
            "due_at",
            format!("must be within the next {MAX_DUE_DAYS} days"),
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid privacy request",
            violations,
        ));
    }
    let mut targets = super::sources::validate(ctx, &command.targets).await?;
    if targets.len() > MAX_TARGETS {
        return Err(AppError::Validation(format!(
            "at most {MAX_TARGETS} items per request"
        )));
    }
    targets.records.sort_by(|a, b| {
        (a.record_type.as_str(), a.record_id).cmp(&(b.record_type.as_str(), b.record_id))
    });
    targets.records.dedup();
    targets.items.sort();
    targets.items.dedup();
    let store = PrivacyStore::of(ctx);
    let targets = store.existing_targets(&targets).await?;
    if targets.is_empty() {
        return Err(AppError::validation_code(
            PRIVACY_TARGETS_EMPTY,
            "select at least one existing record or source item",
        ));
    }

    let request = PrivacyRequest {
        id: Uuid::now_v7(),
        kind,
        status: PrivacyRequestStatus::Pending,
        subject_kind: matcher.kind,
        subject_label: matcher.label(),
        subject_hash: matcher.hash(),
        targets,
        due_at,
        requested_by: Some(user_reference(scope)),
        executed_by: None,
        result: None,
        failure_reason: None,
        file_id: None,
        created_at: now,
        updated_at: now,
        started_at: None,
        finished_at: None,
    };
    let mut tx = ctx.begin().await?;
    store.insert_request(&mut tx, &request).await?;
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(scope),
            event_type: PRIVACY_REQUEST_CREATED.into(),
            record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
            payload: json!({
                "request_id": request.id.to_string(),
                "kind": kind.as_str(),
                "subject_kind": matcher.kind.as_str(),
                "due_at": request.due_at.to_rfc3339(),
                "targets": request.targets.len(),
            }),
            changes: vec![
                FieldChange::new("kind", Value::Null, json!(kind.as_str())),
                FieldChange::new("subject_label", Value::Null, json!(request.subject_label)),
                FieldChange::new("targets", Value::Null, request.targets.to_json()),
                FieldChange::new("due_at", Value::Null, json!(request.due_at.to_rfc3339())),
            ],
            now,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_request(&store, request.id).await
}

async fn require_request(
    store: &PrivacyStore,
    request_id: Uuid,
) -> Result<PrivacyRequest, AppError> {
    store
        .find_request(request_id)
        .await?
        .ok_or_else(privacy_request_not_found)
}

pub(crate) async fn list_requests(
    ctx: &Ctx<'_>,
    filter: PrivacyRequestFilter,
    window: ListWindow,
) -> Result<Listing<PrivacyRequest>, AppError> {
    require_privacy(ctx.scope)?;
    PrivacyStore::of(ctx).list_requests(&filter, window).await
}

pub(crate) async fn get_request(
    ctx: &Ctx<'_>,
    request_id: &str,
) -> Result<PrivacyRequestDetail, AppError> {
    require_privacy(ctx.scope)?;
    let store = PrivacyStore::of(ctx);
    let request = require_request(&store, parse_request_id(request_id)?).await?;
    let titles = store.record_titles(&request.targets.records).await?;
    let records = request
        .targets
        .records
        .iter()
        .map(|record| TargetLabel {
            record: record.clone(),
            title: titles.get(record).cloned().unwrap_or_default(),
            exists: titles.contains_key(record),
        })
        .collect();
    let sources = super::sources::available(ctx)
        .await?
        .into_iter()
        .map(|source| crate::domain::PrivacySourceInfo {
            key: source.key().into(),
            labels: source.labels().0,
        })
        .collect();
    Ok(PrivacyRequestDetail {
        request,
        records,
        sources,
    })
}

fn not_pending() -> AppError {
    AppError::conflict_code(
        PRIVACY_REQUEST_NOT_PENDING,
        "only pending requests can be cancelled or executed",
    )
}

fn ensure_pending(request: &PrivacyRequest) -> Result<(), AppError> {
    if request.status != PrivacyRequestStatus::Pending {
        return Err(not_pending());
    }
    Ok(())
}

pub(crate) async fn cancel_request(
    ctx: &Ctx<'_>,
    request_id: &str,
    ip: Option<String>,
) -> Result<PrivacyRequest, AppError> {
    let scope = ctx.scope;
    require_privacy(scope)?;
    let store = PrivacyStore::of(ctx);
    let mut request = require_request(&store, parse_request_id(request_id)?).await?;
    ensure_pending(&request)?;
    let now = ctx.now();
    request.status = PrivacyRequestStatus::Cancelled;
    request.updated_at = now;
    request.finished_at = Some(now);
    let mut tx = ctx.begin().await?;
    if !store
        .save_request(&mut tx, &request, PrivacyRequestStatus::Pending)
        .await?
    {
        return Err(not_pending());
    }
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(scope),
            event_type: PRIVACY_REQUEST_CANCELLED.into(),
            record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
            payload: json!({ "request_id": request.id.to_string(), "kind": request.kind.as_str() }),
            changes: vec![FieldChange::new(
                "status",
                json!(PrivacyRequestStatus::Pending.as_str()),
                json!(request.status.as_str()),
            )],
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_request(&store, request.id).await
}

pub(crate) async fn execute_request(
    ctx: &Ctx<'_>,
    request_id: &str,
    ip: Option<String>,
) -> Result<PrivacyRequest, AppError> {
    let scope = ctx.scope;
    require_privacy(scope)?;
    let store = PrivacyStore::of(ctx);
    let mut request = require_request(&store, parse_request_id(request_id)?).await?;
    ensure_pending(&request)?;
    let now = ctx.now();
    request.status = PrivacyRequestStatus::Processing;
    request.executed_by = Some(user_reference(scope));
    request.started_at = Some(now);
    request.updated_at = now;
    let run = RunPrivacyRequest {
        request_id: request.id,
        requested_by: scope.user_id().clone(),
    };
    let mut tx = ctx.begin().await?;
    if !store
        .save_request(&mut tx, &request, PrivacyRequestStatus::Pending)
        .await?
    {
        return Err(not_pending());
    }
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::from_scope(scope),
            event_type: PRIVACY_REQUEST_STARTED.into(),
            record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
            payload: json!({
                "request_id": request.id.to_string(),
                "kind": request.kind.as_str(),
                "requested_by": scope.user_id().as_str(),
            }),
            changes: vec![FieldChange::new(
                "status",
                json!(PrivacyRequestStatus::Pending.as_str()),
                json!(request.status.as_str()),
            )],
            now,
            ip,
        },
    )
    .await?;
    ctx.jobs()
        .enqueue(
            &mut tx,
            Job::new(PRIVACY_REQUEST_RUN_JOB, run.payload())
                .with_max_attempts(3)
                .with_unique_key(format!("privacy_request:{}", request.id)),
        )
        .await?;
    tx.commit().await?;
    require_request(&store, request.id).await
}

pub(crate) async fn download_export(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    request_id: &str,
    ip: Option<String>,
) -> Result<PrivacyDownload, AppError> {
    let scope = ctx.scope;
    require_privacy(scope)?;
    let store = PrivacyStore::of(ctx);
    let request = require_request(&store, parse_request_id(request_id)?).await?;
    let unavailable = || {
        AppError::not_found_code(
            PRIVACY_EXPORT_UNAVAILABLE,
            "the export file of this request is not available",
        )
    };
    if request.kind != PrivacyRequestKind::Export
        || request.status != PrivacyRequestStatus::Completed
    {
        return Err(unavailable());
    }
    let file_id = request.file_id.ok_or_else(unavailable)?;
    let file = MySqlFileRepository::new(ctx.pool().clone())
        .find_file(file_id)
        .await?
        .ok_or_else(unavailable)?;
    let content = storage.get_stream(&file.storage_key).await?;
    let mut tx = ctx.begin().await?;
    audit_only(
        ctx,
        &mut tx,
        PRIVACY_REQUEST_DOWNLOADED,
        RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
        vec![FieldChange::new(
            "file_id",
            Value::Null,
            json!(file_id.to_string()),
        )],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(PrivacyDownload { file, content })
}

pub(crate) async fn run_request(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    run: &RunPrivacyRequest,
) -> Result<(), AppError> {
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&run.requested_by)
        .await?
    else {
        return fail_request(ctx, run, REQUESTER_INACTIVE, None).await;
    };
    let scope = build_access_scope(run.requested_by.clone(), access);
    if require_privacy(&scope).is_err() {
        return fail_request(ctx, run, REQUESTER_NOT_ALLOWED, None).await;
    }
    let ctx = ctx.with_scope(&scope);
    let Some(request) = PrivacyStore::of(&ctx).find_request(run.request_id).await? else {
        return Ok(());
    };
    if request.status != PrivacyRequestStatus::Processing {
        return Ok(());
    }
    let outcome = match request.kind {
        PrivacyRequestKind::Export => package::build_export(&ctx, storage, &request).await,
        PrivacyRequestKind::Anonymize | PrivacyRequestKind::Delete => {
            erasure::erase(&ctx, storage, &request).await
        }
    };
    match outcome {
        Ok(outcome) => finish(&ctx, storage, request, run, outcome).await,
        Err(error) if is_permanent_failure(&error) => {
            fail_request(&ctx, run, &error.to_string(), None).await
        }
        Err(error) => Err(error),
    }
}

async fn finish(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    mut request: PrivacyRequest,
    run: &RunPrivacyRequest,
    outcome: ExecutionOutcome,
) -> Result<(), AppError> {
    if outcome.report.errors > 0 {
        return fail_request(
            ctx,
            run,
            "some items could not be processed; see the report and run a new request for the remaining data",
            Some(outcome.report),
        )
        .await;
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let stored = match outcome.file {
        Some(file) => Some(
            store_generated_file(
                &MySqlFileRepository::new(ctx.pool().clone()),
                storage,
                &mut tx,
                ctx.scope,
                file,
                now,
            )
            .await?,
        ),
        None => None,
    };
    request.status = PrivacyRequestStatus::Completed;
    request.result = Some(outcome.report.to_json());
    request.file_id = stored.as_ref().map(|stored| stored.file_id);
    request.finished_at = Some(now);
    request.updated_at = now;
    let result = async {
        if !PrivacyStore::of(ctx)
            .save_request(&mut tx, &request, PrivacyRequestStatus::Processing)
            .await?
        {
            return Ok(false);
        }
        publish(
            ctx,
            &mut tx,
            Mutation {
                actor: Actor::System,
                event_type: PRIVACY_REQUEST_COMPLETED.into(),
                record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
                payload: json!({
                    "request_id": request.id.to_string(),
                    "kind": request.kind.as_str(),
                    "requested_by": run.requested_by.as_str(),
                    "file_id": request.file_id.map(|id| id.to_string()),
                    "result": request.result,
                }),
                changes: vec![
                    FieldChange::new(
                        "status",
                        json!(PrivacyRequestStatus::Processing.as_str()),
                        json!(request.status.as_str()),
                    ),
                    FieldChange::new("result", Value::Null, outcome.report.to_json()),
                ],
                now,
                ip: None,
            },
        )
        .await?;
        tx.commit().await?;
        Ok::<bool, AppError>(true)
    }
    .await;
    if !matches!(result, Ok(true))
        && let Some(stored) = stored
    {
        discard_generated_file(storage, &stored.storage_key).await;
    }
    result.map(|_| ())
}

pub(crate) async fn fail_request(
    ctx: &Ctx<'_>,
    run: &RunPrivacyRequest,
    reason: &str,
    report: Option<RunReport>,
) -> Result<(), AppError> {
    let scope = acting_scope(Some(run.requested_by.clone()));
    let ctx = ctx.with_scope(&scope);
    let store = PrivacyStore::of(&ctx);
    let Some(mut request) = store.find_request(run.request_id).await? else {
        return Ok(());
    };
    if request.status != PrivacyRequestStatus::Processing {
        return Ok(());
    }
    let now = ctx.now();
    request.status = PrivacyRequestStatus::Failed;
    request.failure_reason = Some(reason.chars().take(500).collect());
    request.result = report.map(|report| report.to_json());
    request.finished_at = Some(now);
    request.updated_at = now;
    let mut tx = ctx.begin().await?;
    if !store
        .save_request(&mut tx, &request, PrivacyRequestStatus::Processing)
        .await?
    {
        return Ok(());
    }
    publish(
        &ctx,
        &mut tx,
        Mutation {
            actor: Actor::System,
            event_type: PRIVACY_REQUEST_FAILED.into(),
            record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
            payload: json!({
                "request_id": request.id.to_string(),
                "kind": request.kind.as_str(),
                "requested_by": run.requested_by.as_str(),
                "result": request.result,
            }),
            changes: vec![
                FieldChange::new(
                    "status",
                    json!(PrivacyRequestStatus::Processing.as_str()),
                    json!(request.status.as_str()),
                ),
                FieldChange::new("failure_reason", Value::Null, json!(request.failure_reason)),
            ],
            now,
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
