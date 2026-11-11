use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::definitions;
use crate::domain::{
    ACTIVATE_JOB, IDEMPOTENCY_CONFLICT, MAX_ATTEMPTS, MAX_IDEMPOTENCY_KEY_CHARS, MAX_NOTE_CHARS,
    MAX_PAYLOAD_BYTES, MAX_SCHEDULE_DAYS, PAYLOAD_TOO_LARGE, PublicationRequest,
    REQUEST_NOT_FAILED, REQUEST_NOT_OPEN, Revision, Subject, coded, record_not_found,
    request_not_found, revision_not_found,
};
use crate::store;
use aphel_contract_publication::{
    PublishableDefinition, REQUEST_CANCELLED, REQUEST_SUPERSEDED, REQUESTED, REVISION_CAPTURED,
    RequestAction, RequestStatus,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::EntityDefinition;
use aphel_sdk::platform::application::shared::{Job, JobQueue, RecordRef, RecordType};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::records::{lock_version, snapshot};
use aphel_sdk::records::RecordCtx;

pub(crate) struct CaptureInput {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) expected_version: Option<DateTime<Utc>>,
    pub(crate) note: Option<String>,
}

pub(crate) struct RequestInput {
    pub(crate) revision_id: String,
    pub(crate) at: Option<DateTime<Utc>>,
    pub(crate) idempotency_key: String,
}

pub(crate) struct WithdrawInput {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) idempotency_key: String,
}

pub(crate) struct RestoreInput {
    pub(crate) revision_id: String,
    pub(crate) expected_version: Option<DateTime<Utc>>,
}

pub(crate) struct StatusView {
    pub(crate) record: RecordRef,
    pub(crate) subject: Option<Subject>,
    pub(crate) active: Option<Revision>,
    pub(crate) requests: Vec<PublicationRequest>,
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid publication command",
        vec![FieldViolation::new(field, message)],
    )
}

fn parse_id(value: &str, not_found: fn() -> AppError) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| not_found())
}

fn require_user(ctx: &Ctx<'_>) -> Result<String, AppError> {
    if ctx.scope.is_system() {
        return Err(AppError::Validation(
            "publication commands need a user actor".to_owned(),
        ));
    }
    Ok(ctx.scope.user_id().as_str().to_owned())
}

fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

pub(crate) fn hash(value: &Value) -> String {
    let digest = Sha256::digest(canonical(value).to_string().as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn collect_file_ids(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if let Ok(id) = Uuid::parse_str(text.trim()) {
                found.push(id.to_string());
            } else if let Ok(parsed) = serde_json::from_str::<Value>(text)
                && (parsed.is_array() || parsed.is_object())
            {
                collect_file_ids(&parsed, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_file_ids(item, found)),
        Value::Object(map) => map.values().for_each(|item| collect_file_ids(item, found)),
        _ => {}
    }
}

pub(crate) fn publish_permission(
    definition: &PublishableDefinition,
) -> Result<Permission, AppError> {
    Permission::parse(definition.permission.clone()).map_err(|_| {
        coded(
            CodedKind::Conflict,
            crate::domain::DEFINITION_INVALID,
            format!("invalid permission {}", definition.permission),
        )
    })
}

pub(crate) struct Target<'r> {
    pub(crate) definition: PublishableDefinition,
    pub(crate) entity: &'r EntityDefinition,
    pub(crate) record: RecordRef,
}

pub(crate) async fn visible_target<'r>(
    ctx: &'r Ctx<'_>,
    record_type: &str,
    record_id: Uuid,
) -> Result<Target<'r>, AppError> {
    let definition = definitions::find(ctx.registry(), record_type)?;
    let entity = ctx.registry().require(record_type)?;
    let record = RecordRef::new(RecordType::parse(record_type)?, record_id);
    let state = aphel_sdk::runtime()?;
    let handler = ctx.registry().handler(record_type)?;
    if handler
        .load(&RecordCtx::from_runtime(ctx.scope, &state), record_id)
        .await?
        .is_none()
    {
        return Err(record_not_found());
    }
    Ok(Target {
        definition,
        entity,
        record,
    })
}

fn require_update(ctx: &Ctx<'_>, entity: &EntityDefinition) -> Result<(), AppError> {
    let permission = entity.update_permission.as_ref().ok_or_else(|| {
        AppError::forbidden_code("permission_denied", "the record type cannot be edited")
    })?;
    ctx.scope.require(permission).map(|_| ())
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    event_type: &'static str,
    payload: Value,
) -> Result<(), AppError> {
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: record.clone(),
            event_type,
            payload,
            changes: Vec::new(),
            ip: None,
        },
    )
    .await
}

pub(crate) async fn capture(ctx: &Ctx<'_>, input: CaptureInput) -> Result<Revision, AppError> {
    let user = require_user(ctx)?;
    let record_id = parse_id(&input.record_id, record_not_found)?;
    let target = visible_target(ctx, input.record_type.trim(), record_id).await?;
    require_update(ctx, target.entity)?;
    let expected = input.expected_version.ok_or_else(|| {
        invalid(
            "expected_version",
            "is required: send the version of the saved draft",
        )
    })?;
    let note = input
        .note
        .map(|note| note.trim().to_owned())
        .filter(|note| !note.is_empty());
    if note
        .as_ref()
        .is_some_and(|note| note.chars().count() > MAX_NOTE_CHARS)
    {
        return Err(invalid("note", "is too long"));
    }
    let mut tx = ctx.begin().await?;
    lock_version(&mut tx, &target.entity.table, record_id, Some(expected)).await?;
    let stored = snapshot::capture(&mut tx, ctx.pool(), ctx.registry(), &target.record)
        .await?
        .ok_or_else(record_not_found)?;
    let values = stored.get("values").cloned().unwrap_or(Value::Null);
    let mut fields = Map::new();
    let mut files = Vec::new();
    for mapping in &target.definition.fields {
        let value = values.get(&mapping.draft).cloned().unwrap_or(Value::Null);
        if mapping.file {
            collect_file_ids(&value, &mut files);
        }
        fields.insert(mapping.draft.clone(), value);
    }
    files.sort();
    files.dedup();
    let payload = json!({ "format": target.definition.format, "fields": Value::Object(fields) });
    if payload.to_string().len() > MAX_PAYLOAD_BYTES {
        return Err(coded(
            CodedKind::PayloadTooLarge,
            PAYLOAD_TOO_LARGE,
            "the content is too large to publish",
        ));
    }
    store::lock_subject(&mut tx, &target.record).await?;
    let revision = Revision {
        id: Uuid::now_v7(),
        record: target.record.clone(),
        number: store::next_revision_number(&mut tx, &target.record).await?,
        format: target.definition.format,
        payload_hash: hash(&payload),
        payload,
        draft_version: expected,
        note,
        created_by: Some(user),
        created_at: ctx.now(),
    };
    store::insert_revision(&mut tx, &revision, &files).await?;
    publish(
        ctx,
        &mut tx,
        &revision.record,
        REVISION_CAPTURED,
        json!({ "revision_id": revision.id.to_string(), "number": revision.number, "payload_hash": revision.payload_hash }),
    )
    .await?;
    tx.commit().await?;
    Ok(revision)
}

fn check_key(key: &str) -> Result<String, AppError> {
    let key = key.trim();
    if key.is_empty() || key.chars().count() > MAX_IDEMPOTENCY_KEY_CHARS {
        return Err(invalid(
            "idempotency_key",
            "must have between 1 and 80 characters",
        ));
    }
    Ok(key.to_owned())
}

async fn replay(
    tx: &mut MySqlTx,
    user: &str,
    key: &str,
    input_hash: &str,
) -> Result<Option<PublicationRequest>, AppError> {
    match store::request_by_key(tx, user, key).await? {
        Some(existing) if existing.input_hash == input_hash => Ok(Some(existing)),
        Some(_) => Err(coded(
            CodedKind::Conflict,
            IDEMPOTENCY_CONFLICT,
            "the idempotency key was already used for a different command",
        )),
        None => Ok(None),
    }
}

async fn supersede_open(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    status: RequestStatus,
) -> Result<(), AppError> {
    for mut open in store::open_requests(tx, record).await? {
        open.status = status;
        open.completed_at = Some(ctx.now());
        store::save_request(tx, &open).await?;
        publish(
            ctx,
            tx,
            record,
            if status == RequestStatus::Cancelled {
                REQUEST_CANCELLED
            } else {
                REQUEST_SUPERSEDED
            },
            json!({ "request_id": open.id.to_string(), "generation": open.generation }),
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn enqueue(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    request: &PublicationRequest,
) -> Result<(), AppError> {
    let mut job = Job::new(
        ACTIVATE_JOB,
        json!({ "request_id": request.id.to_string() }),
    )
    .with_max_attempts(MAX_ATTEMPTS)
    .with_unique_key(format!(
        "publication:{}:{}",
        request.id,
        ctx.now().timestamp_micros()
    ));
    if let Some(at) = request.run_at.filter(|at| *at > ctx.now()) {
        job = job.run_at(at);
    }
    ctx.jobs().enqueue(tx, job).await?;
    Ok(())
}

struct NewRequest {
    action: RequestAction,
    revision_id: Option<Uuid>,
    run_at: Option<DateTime<Utc>>,
    user: String,
    key: String,
    input_hash: String,
}

async fn open_request(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    new: NewRequest,
) -> Result<PublicationRequest, AppError> {
    let NewRequest {
        action,
        revision_id,
        run_at,
        user,
        key,
        input_hash,
    } = new;
    let subject = store::lock_subject(tx, record).await?;
    supersede_open(
        ctx,
        tx,
        record,
        if action == RequestAction::Withdraw {
            RequestStatus::Cancelled
        } else {
            RequestStatus::Superseded
        },
    )
    .await?;
    let generation = store::bump_generation(tx, record, ctx.now()).await?;
    let request = PublicationRequest {
        id: Uuid::now_v7(),
        record: record.clone(),
        action,
        revision_id,
        generation,
        expected_active_revision_id: subject.active_revision_id,
        status: if run_at.is_some() {
            RequestStatus::Scheduled
        } else {
            RequestStatus::Ready
        },
        run_at,
        requested_by: Some(user),
        idempotency_key: key,
        input_hash,
        attempts: 0,
        reason_code: None,
        created_at: ctx.now(),
        completed_at: None,
    };
    store::insert_request(tx, &request).await?;
    enqueue(ctx, tx, &request).await?;
    publish(
        ctx,
        tx,
        record,
        REQUESTED,
        json!({
            "request_id": request.id.to_string(),
            "action": action.as_str(),
            "revision_id": revision_id.map(|id| id.to_string()),
            "generation": generation,
            "run_at": run_at,
        }),
    )
    .await?;
    Ok(request)
}

async fn run_inline(
    ctx: &Ctx<'_>,
    request: PublicationRequest,
) -> Result<PublicationRequest, AppError> {
    if request.status == RequestStatus::Ready
        && let Err(error) = crate::activation::process(request.id, Some(ctx.scope.clone())).await
    {
        tracing::warn!(request_id = %request.id, %error, "publication will be retried by the worker");
    }
    store::request(ctx.pool(), request.id)
        .await?
        .ok_or_else(request_not_found)
}

pub(crate) async fn request(
    ctx: &Ctx<'_>,
    input: RequestInput,
) -> Result<PublicationRequest, AppError> {
    let user = require_user(ctx)?;
    let key = check_key(&input.idempotency_key)?;
    let revision_id = parse_id(&input.revision_id, revision_not_found)?;
    let revision = store::revision(ctx.pool(), revision_id)
        .await?
        .ok_or_else(revision_not_found)?;
    let target = visible_target(
        ctx,
        revision.record.record_type.as_str(),
        revision.record.record_id,
    )
    .await?;
    ctx.scope
        .require(&publish_permission(&target.definition)?)?;
    let now = ctx.now();
    let run_at = input.at.filter(|at| *at > now);
    if run_at.is_some_and(|at| at > now + Duration::days(MAX_SCHEDULE_DAYS)) {
        return Err(invalid("at", "must be within one year"));
    }
    let input_hash = hash(
        &json!({ "action": "activate", "revision_id": revision_id.to_string(), "at": run_at }),
    );
    let mut tx = ctx.begin().await?;
    if let Some(existing) = replay(&mut tx, &user, &key, &input_hash).await? {
        return Ok(existing);
    }
    let request = open_request(
        ctx,
        &mut tx,
        &target.record,
        NewRequest {
            action: RequestAction::Activate,
            revision_id: Some(revision_id),
            run_at,
            user,
            key,
            input_hash,
        },
    )
    .await?;
    tx.commit().await?;
    run_inline(ctx, request).await
}

pub(crate) async fn withdraw(
    ctx: &Ctx<'_>,
    input: WithdrawInput,
) -> Result<PublicationRequest, AppError> {
    let user = require_user(ctx)?;
    let key = check_key(&input.idempotency_key)?;
    let record_id = parse_id(&input.record_id, record_not_found)?;
    let target = visible_target(ctx, input.record_type.trim(), record_id).await?;
    ctx.scope
        .require(&publish_permission(&target.definition)?)?;
    let input_hash = hash(&json!({
        "action": "withdraw",
        "record_type": target.record.record_type.as_str(),
        "record_id": record_id.to_string(),
    }));
    let mut tx = ctx.begin().await?;
    if let Some(existing) = replay(&mut tx, &user, &key, &input_hash).await? {
        return Ok(existing);
    }
    let request = open_request(
        ctx,
        &mut tx,
        &target.record,
        NewRequest {
            action: RequestAction::Withdraw,
            revision_id: None,
            run_at: None,
            user,
            key,
            input_hash,
        },
    )
    .await?;
    tx.commit().await?;
    run_inline(ctx, request).await
}

async fn authorized_request(ctx: &Ctx<'_>, id: &str) -> Result<PublicationRequest, AppError> {
    let id = parse_id(id, request_not_found)?;
    let request = store::request(ctx.pool(), id)
        .await?
        .ok_or_else(request_not_found)?;
    let target = visible_target(
        ctx,
        request.record.record_type.as_str(),
        request.record.record_id,
    )
    .await
    .map_err(|_| request_not_found())?;
    ctx.scope
        .require(&publish_permission(&target.definition)?)?;
    Ok(request)
}

pub(crate) async fn cancel(ctx: &Ctx<'_>, id: &str) -> Result<PublicationRequest, AppError> {
    require_user(ctx)?;
    let request = authorized_request(ctx, id).await?;
    let mut tx = ctx.begin().await?;
    let mut locked = store::lock_request(&mut tx, request.id)
        .await?
        .ok_or_else(request_not_found)?;
    if !locked.status.is_open() {
        return Err(coded(
            CodedKind::Conflict,
            REQUEST_NOT_OPEN,
            "the publication request is no longer open",
        ));
    }
    locked.status = RequestStatus::Cancelled;
    locked.completed_at = Some(ctx.now());
    store::save_request(&mut tx, &locked).await?;
    publish(
        ctx,
        &mut tx,
        &locked.record,
        REQUEST_CANCELLED,
        json!({ "request_id": locked.id.to_string(), "generation": locked.generation }),
    )
    .await?;
    tx.commit().await?;
    Ok(locked)
}

pub(crate) async fn retry(ctx: &Ctx<'_>, id: &str) -> Result<PublicationRequest, AppError> {
    require_user(ctx)?;
    let request = authorized_request(ctx, id).await?;
    let mut tx = ctx.begin().await?;
    let mut locked = store::lock_request(&mut tx, request.id)
        .await?
        .ok_or_else(request_not_found)?;
    if locked.status != RequestStatus::Failed {
        return Err(coded(
            CodedKind::Conflict,
            REQUEST_NOT_FAILED,
            "only a failed publication request can be retried",
        ));
    }
    locked.status = RequestStatus::Ready;
    locked.attempts = 0;
    locked.reason_code = None;
    locked.completed_at = None;
    locked.run_at = None;
    store::save_request(&mut tx, &locked).await?;
    enqueue(ctx, &mut tx, &locked).await?;
    tx.commit().await?;
    run_inline(ctx, locked).await
}

pub(crate) async fn restore(ctx: &Ctx<'_>, input: RestoreInput) -> Result<Value, AppError> {
    require_user(ctx)?;
    let revision_id = parse_id(&input.revision_id, revision_not_found)?;
    let revision = store::revision(ctx.pool(), revision_id)
        .await?
        .ok_or_else(revision_not_found)?;
    let target = visible_target(
        ctx,
        revision.record.record_type.as_str(),
        revision.record.record_id,
    )
    .await?;
    require_update(ctx, target.entity)?;
    let expected = input.expected_version.ok_or_else(|| {
        invalid(
            "expected_version",
            "is required: send the version of the saved draft",
        )
    })?;
    let mut values = revision
        .payload
        .get("fields")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    values.retain(|key, _| {
        target
            .definition
            .fields
            .iter()
            .any(|mapping| &mapping.draft == key)
    });
    values.insert("_expected_updated_at".to_owned(), json!(expected));
    let state = aphel_sdk::runtime()?;
    ctx.registry()
        .handler(target.record.record_type.as_str())?
        .update(
            &RecordCtx::from_runtime(ctx.scope, &state),
            target.record.record_id,
            Value::Object(values),
        )
        .await
}

pub(crate) async fn status(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<StatusView, AppError> {
    let record_id = parse_id(record_id, record_not_found)?;
    let target = visible_target(ctx, record_type.trim(), record_id).await?;
    let subject = store::subject(ctx.pool(), &target.record).await?;
    let active = match subject
        .as_ref()
        .and_then(|subject| subject.active_revision_id)
    {
        Some(id) => store::revision(ctx.pool(), id).await?,
        None => None,
    };
    let requests = store::latest_requests(ctx.pool(), &target.record, 5).await?;
    Ok(StatusView {
        record: target.record,
        subject,
        active,
        requests,
    })
}

pub(crate) async fn revisions(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    limit: u32,
    offset: u32,
) -> Result<(Vec<Revision>, u64), AppError> {
    let record_id = parse_id(record_id, record_not_found)?;
    let target = visible_target(ctx, record_type.trim(), record_id).await?;
    store::revisions(ctx.pool(), &target.record, limit, offset).await
}

pub(crate) async fn revision(ctx: &Ctx<'_>, id: &str) -> Result<Revision, AppError> {
    let id = parse_id(id, revision_not_found)?;
    let revision = store::revision(ctx.pool(), id)
        .await?
        .ok_or_else(revision_not_found)?;
    visible_target(
        ctx,
        revision.record.record_type.as_str(),
        revision.record.record_id,
    )
    .await
    .map_err(|_| revision_not_found())?;
    Ok(revision)
}
