use chrono::Duration;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    APPROVAL_CANCELLED, APPROVAL_NO_APPROVER, APPROVAL_PENDING, APPROVAL_RECORD_NOT_FOUND,
    APPROVAL_REQUESTED, APPROVAL_REQUIRED, ApprovalCheckpoint, ApprovalPolicy, ApprovalRequest,
    ApprovalStatus, ApprovalSubmission, MAX_DECISION_COMMENT_LENGTH, MAX_DUE_HOURS,
    SubmissionOutcome, locks_field,
};
use crate::store::{ApprovalDirectory, ApprovalStore};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::{EntityDefinition, RecordWrite, WriteMode};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};

pub(crate) struct RequestEvent<'a> {
    pub(crate) event_type: &'static str,
    pub(crate) request: &'a ApprovalRequest,
    pub(crate) step: Option<u32>,
    pub(crate) approvers: Option<&'a [UserId]>,
    pub(crate) changes: Vec<FieldChange>,
    pub(crate) ip: Option<String>,
}

impl<'a> RequestEvent<'a> {
    pub(crate) fn new(event_type: &'static str, request: &'a ApprovalRequest) -> Self {
        Self {
            event_type,
            request,
            step: None,
            approvers: None,
            changes: Vec::new(),
            ip: None,
        }
    }

    pub(crate) fn step(mut self, step: u32) -> Self {
        self.step = Some(step);
        self
    }

    pub(crate) fn approvers(mut self, approvers: &'a [UserId]) -> Self {
        self.approvers = Some(approvers);
        self
    }

    pub(crate) fn changes(mut self, changes: Vec<FieldChange>) -> Self {
        self.changes = changes;
        self
    }

    pub(crate) fn ip(mut self, ip: Option<String>) -> Self {
        self.ip = ip;
        self
    }
}

pub(crate) fn acting_user(scope: &AccessScope) -> Option<UserId> {
    Uuid::parse_str(scope.user_id().as_str())
        .is_ok()
        .then(|| scope.user_id().clone())
}

pub(crate) fn acting_actor(scope: &AccessScope) -> Actor {
    match Actor::from_scope(scope) {
        Actor::User(id) if Uuid::parse_str(id.as_str()).is_err() => Actor::System,
        actor => actor,
    }
}

pub(crate) fn record_not_found() -> AppError {
    AppError::not_found_code(APPROVAL_RECORD_NOT_FOUND, "record was not found")
}

pub(crate) fn update_permission(definition: &EntityDefinition) -> Result<Permission, AppError> {
    definition
        .update_permission
        .clone()
        .or_else(|| definition.action_permission("update"))
        .ok_or_else(|| {
            AppError::validation_code(
                APPROVAL_RECORD_NOT_FOUND,
                format!(
                    "records of type {} cannot be edited",
                    definition.record_type.as_str()
                ),
            )
        })
}

pub(crate) fn clean_comment(
    comment: Option<&str>,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let comment = comment.map(str::trim).filter(|value| !value.is_empty())?;
    if comment.chars().count() > MAX_DECISION_COMMENT_LENGTH {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {MAX_DECISION_COMMENT_LENGTH} characters"),
        ));
    }
    Some(comment.to_owned())
}

pub(crate) fn pending_error(request: &ApprovalRequest, fields: Option<Vec<String>>) -> AppError {
    let mut details = json!({
        "request_id": request.id.to_string(),
        "record_type": request.record.record_type.as_str(),
        "record_id": request.record.record_id.to_string(),
        "moment": request.moment.as_str(),
        "target_id": request.target_id.map(|id| id.to_string()),
    });
    if let Some(fields) = fields {
        details["fields"] = json!(fields);
    }
    AppError::with_details(
        CodedKind::Conflict,
        APPROVAL_PENDING,
        "an approval request is pending for this record",
        details,
    )
}

pub(crate) fn required_error(
    checkpoint: &ApprovalCheckpoint,
    policies: &[&ApprovalPolicy],
) -> AppError {
    AppError::with_details(
        CodedKind::Conflict,
        APPROVAL_REQUIRED,
        "this change requires an approval",
        json!({
            "record_type": checkpoint.record.record_type.as_str(),
            "record_id": checkpoint.record.record_id.to_string(),
            "moment": checkpoint.moment.as_str(),
            "target_id": checkpoint.target_id.map(|id| id.to_string()),
            "policies": policies
                .iter()
                .map(|policy| json!({
                    "id": policy.id.to_string(),
                    "key": policy.key,
                    "name": policy.name,
                }))
                .collect::<Vec<_>>(),
        }),
    )
}

fn check_target(checkpoint: &ApprovalCheckpoint) -> Result<(), AppError> {
    let violation = match (checkpoint.moment.requires_target(), checkpoint.target_id) {
        (true, None) => Some("is required for this moment"),
        (false, Some(_)) => Some("must be empty for this moment"),
        _ => None,
    };
    match violation {
        Some(message) => Err(AppError::invalid_fields(
            "invalid approval request",
            vec![FieldViolation::new("target_id", message)],
        )),
        None => Ok(()),
    }
}

fn definition<'r>(ctx: &'r Ctx<'_>, record: &RecordRef) -> Result<&'r EntityDefinition, AppError> {
    ctx.registry().require(record.record_type.as_str())
}

pub(crate) async fn applicable(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    checkpoint: &ApprovalCheckpoint,
) -> Result<Vec<ApprovalPolicy>, AppError> {
    let scope = ctx.scope;
    let policies = ApprovalStore::of(ctx)
        .active_policies(
            tx,
            checkpoint.record.record_type.as_str(),
            checkpoint.moment,
            checkpoint.target_id,
        )
        .await?;
    if policies.is_empty() {
        return Ok(policies);
    }
    let definition = definition(ctx, &checkpoint.record)?;
    let mut applicable = Vec::new();
    for policy in policies {
        if ApprovalDirectory::of(ctx)
            .condition_matches(
                tx,
                scope,
                definition,
                checkpoint.record.record_id,
                &policy.condition,
            )
            .await?
        {
            applicable.push(policy);
        }
    }
    Ok(applicable)
}

pub(crate) async fn checkpoint(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    checkpoint: &ApprovalCheckpoint,
) -> Result<(), AppError> {
    let policies = applicable(ctx, tx, checkpoint).await?;
    if policies.is_empty() {
        return Ok(());
    }
    let existing = ApprovalStore::of(ctx)
        .open_requests(
            tx,
            &checkpoint.record,
            checkpoint.moment,
            checkpoint.target_id,
        )
        .await?;
    let of_policy =
        |policy: &ApprovalPolicy, request: &&ApprovalRequest| request.policy_id == Some(policy.id);
    if let Some(pending) = policies.iter().find_map(|policy| {
        existing
            .iter()
            .filter(|request| of_policy(policy, request))
            .find(|request| request.status == ApprovalStatus::Pending)
    }) {
        return Err(pending_error(pending, None));
    }
    let mut consumed = Vec::new();
    let mut missing = Vec::new();
    for policy in &policies {
        match existing
            .iter()
            .filter(|request| of_policy(policy, request))
            .find(|request| request.is_unconsumed_approval())
        {
            Some(request) => consumed.push(request.id),
            None => missing.push(policy),
        }
    }
    if !missing.is_empty() {
        return Err(required_error(checkpoint, &missing));
    }
    ApprovalStore::of(ctx)
        .mark_consumed(tx, &consumed, ctx.now())
        .await
}

pub(crate) async fn submit(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    submission: &ApprovalSubmission,
    only_policy: Option<&str>,
) -> Result<SubmissionOutcome, AppError> {
    let scope = ctx.scope;
    let checkpoint = &submission.checkpoint;
    check_target(checkpoint)?;
    let mut violations = Vec::new();
    let comment = clean_comment(submission.comment.as_deref(), "comment", &mut violations);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid approval request",
            violations,
        ));
    }
    let definition = definition(ctx, &checkpoint.record)?;
    let visibility = scope.record_filter(&update_permission(definition)?)?;
    let state = ApprovalDirectory::of(ctx)
        .record_state_in(
            tx,
            scope,
            definition,
            checkpoint.record.record_id,
            Some(&visibility),
        )
        .await?
        .ok_or_else(record_not_found)?;
    let policies: Vec<ApprovalPolicy> = applicable(ctx, tx, checkpoint)
        .await?
        .into_iter()
        .filter(|policy| only_policy.is_none_or(|key| policy.key == key))
        .collect();
    if policies.is_empty() {
        return Ok(SubmissionOutcome::NotRequired);
    }
    let existing = ApprovalStore::of(ctx)
        .open_requests(
            tx,
            &checkpoint.record,
            checkpoint.moment,
            checkpoint.target_id,
        )
        .await?;
    let now = ctx.now();
    let mut pending = Vec::new();
    let mut approved = Vec::new();
    for policy in policies {
        let mut current = existing
            .iter()
            .filter(|request| request.policy_id == Some(policy.id));
        if let Some(request) = current
            .clone()
            .find(|request| request.status == ApprovalStatus::Pending)
        {
            pending.push(request.summary());
            continue;
        }
        if let Some(request) = current.find(|request| request.is_unconsumed_approval()) {
            approved.push(request.summary());
            continue;
        }
        let request = ApprovalRequest {
            id: Uuid::now_v7(),
            policy_id: Some(policy.id),
            policy_key: Some(policy.key.clone()),
            policy_name: policy.name.clone(),
            record: checkpoint.record.clone(),
            moment: checkpoint.moment,
            target_id: checkpoint.target_id,
            status: ApprovalStatus::Pending,
            current_step: 0,
            steps: policy.step_snapshots(),
            locked_fields: policy.locked_fields.clone(),
            comment: comment.clone(),
            requested_by: acting_user(scope),
            requested_at: now,
            due_at: policy
                .due_hours
                .filter(|hours| (1..=MAX_DUE_HOURS).contains(hours))
                .map(|hours| now + Duration::hours(i64::from(hours))),
            resolved_at: None,
            resolved_by: None,
            resolution_comment: None,
            consumed_at: None,
            updated_at: now,
        };
        ensure_approvers(ctx, tx, &request, state.owner_id.as_ref()).await?;
        ApprovalStore::of(ctx).insert_request(tx, &request).await?;
        let approvers = start_step(ctx, tx, &request, 0, state.owner_id.as_ref()).await?;
        publish(
            ctx,
            tx,
            RequestEvent::new(APPROVAL_REQUESTED, &request)
                .step(0)
                .approvers(&approvers)
                .changes(vec![FieldChange::new(
                    "status",
                    Value::Null,
                    json!(ApprovalStatus::Pending.as_str()),
                )])
                .ip(submission.ip.clone()),
        )
        .await?;
        pending.push(request.summary());
    }
    Ok(if pending.is_empty() {
        SubmissionOutcome::Approved(approved)
    } else {
        SubmissionOutcome::Pending(pending)
    })
}

pub(crate) async fn owner_of(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
) -> Result<Option<UserId>, AppError> {
    let scope = ctx.scope;
    let Some(definition) = ctx.registry().get(&record.record_type) else {
        return Ok(None);
    };
    Ok(ApprovalDirectory::of(ctx)
        .record_state_in(tx, scope, definition, record.record_id, None)
        .await?
        .and_then(|state| state.owner_id))
}

pub(crate) async fn step_approvers(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    request: &ApprovalRequest,
    step: u32,
    owner: Option<&UserId>,
) -> Result<Vec<UserId>, AppError> {
    let Some(snapshot) = request.steps.get(step as usize) else {
        return Ok(Vec::new());
    };
    let requester = request.requested_by.as_ref();
    let keep = |users: Vec<UserId>| {
        let mut kept: Vec<UserId> = Vec::new();
        for user in users {
            if Some(&user) != requester && !kept.contains(&user) {
                kept.push(user);
            }
        }
        kept
    };
    let directory = ApprovalDirectory::of(ctx);
    let users = keep(directory.approvers(tx, snapshot, owner).await?);
    if !users.is_empty() {
        return Ok(users);
    }
    Ok(keep(directory.managers(tx).await?))
}

pub(crate) fn no_approver_error(request: &ApprovalRequest, step: u32) -> AppError {
    let snapshot = request.steps.get(step as usize);
    AppError::with_details(
        CodedKind::Conflict,
        APPROVAL_NO_APPROVER,
        "no eligible approver for this approval step",
        json!({
            "policy_id": request.policy_id.map(|id| id.to_string()),
            "policy_key": request.policy_key,
            "policy_name": request.policy_name,
            "record_type": request.record.record_type.as_str(),
            "record_id": request.record.record_id.to_string(),
            "step": step,
            "step_name": snapshot.and_then(|snapshot| snapshot.name.clone()),
        }),
    )
}

pub(crate) async fn ensure_approvers(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    request: &ApprovalRequest,
    owner: Option<&UserId>,
) -> Result<(), AppError> {
    for step in 0..request.steps.len() as u32 {
        if step_approvers(ctx, tx, request, step, owner)
            .await?
            .is_empty()
        {
            return Err(no_approver_error(request, step));
        }
    }
    Ok(())
}

pub(crate) async fn start_step(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    request: &ApprovalRequest,
    step: u32,
    owner: Option<&UserId>,
) -> Result<Vec<UserId>, AppError> {
    if request.steps.get(step as usize).is_none() {
        return Ok(Vec::new());
    }
    let users = step_approvers(ctx, tx, request, step, owner).await?;
    if users.is_empty() {
        return Err(no_approver_error(request, step));
    }
    ApprovalStore::of(ctx)
        .insert_approvers(tx, request.id, step, &users)
        .await?;
    Ok(users)
}

pub(crate) async fn cancel_open(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    reason: &str,
) -> Result<u32, AppError> {
    let pending = ApprovalStore::of(ctx)
        .pending_for_record(tx, record)
        .await?;
    let mut cancelled = 0;
    for request in pending {
        cancel(ctx, tx, request, Some(reason.to_owned()), None).await?;
        cancelled += 1;
    }
    let unused: Vec<Uuid> = ApprovalStore::of(ctx)
        .requests_for_record(record)
        .await?
        .into_iter()
        .filter(|request| {
            request.status == ApprovalStatus::Approved && request.consumed_at.is_none()
        })
        .map(|request| request.id)
        .collect();
    let now = ctx.now();
    ApprovalStore::of(ctx)
        .mark_consumed(tx, &unused, now)
        .await?;
    Ok(cancelled)
}

pub(crate) async fn cancel(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    mut request: ApprovalRequest,
    reason: Option<String>,
    ip: Option<String>,
) -> Result<ApprovalRequest, AppError> {
    let scope = ctx.scope;
    let now = ctx.now();
    request.status = ApprovalStatus::Cancelled;
    request.resolved_at = Some(now);
    request.resolved_by = acting_user(scope);
    request.resolution_comment = reason;
    request.updated_at = now;
    ApprovalStore::of(ctx)
        .save_request_state(tx, &request)
        .await?;
    publish(
        ctx,
        tx,
        RequestEvent::new(APPROVAL_CANCELLED, &request)
            .step(request.current_step)
            .changes(status_change(
                ApprovalStatus::Pending,
                ApprovalStatus::Cancelled,
            ))
            .ip(ip),
    )
    .await?;
    Ok(request)
}

pub(crate) async fn guard(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    write: &RecordWrite<'_>,
) -> Result<(), AppError> {
    if write.mode != WriteMode::Update || write.changes.is_empty() {
        return Ok(());
    }
    let pending = ApprovalStore::of(ctx)
        .pending_for_record(tx, write.record)
        .await?;
    for request in &pending {
        let fields: Vec<String> = write
            .changed_fields()
            .filter(|field| locks_field(&request.locked_fields, field))
            .map(str::to_owned)
            .collect();
        if !fields.is_empty() {
            return Err(pending_error(request, Some(fields)));
        }
    }
    let stage = write
        .changed("stage_id")
        .and_then(|change| change.new.as_str())
        .and_then(|value| Uuid::parse_str(value).ok());
    if let Some(stage) = stage {
        checkpoint(
            ctx,
            tx,
            &ApprovalCheckpoint::stage_change(write.record.clone(), stage),
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    event: RequestEvent<'_>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let now = ctx.now();
    let actor = acting_actor(scope);
    let mut payload = event.request.event_payload(event.step);
    if let Some(approvers) = event.approvers {
        payload["approvers"] = json!(approvers.iter().map(UserId::as_str).collect::<Vec<_>>());
    }
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event.event_type, now)
                .with_record(event.request.record.clone())
                .with_payload(payload),
        )
        .await?;
    let mut changes = vec![FieldChange::new(
        "approval_request_id",
        Value::Null,
        json!(event.request.id.to_string()),
    )];
    changes.extend(event.changes);
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event.event_type, now)
                .with_record(event.request.record.clone())
                .with_changes(changes)
                .with_ip(event.ip),
        )
        .await
}

pub(crate) fn status_change(from: ApprovalStatus, to: ApprovalStatus) -> Vec<FieldChange> {
    vec![FieldChange::new(
        "status",
        json!(from.as_str()),
        json!(to.as_str()),
    )]
}
