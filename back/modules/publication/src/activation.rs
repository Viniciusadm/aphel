use chrono::SecondsFormat;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::definitions;
use crate::domain::{
    APPROVALS_UNAVAILABLE, NOTHING_PUBLISHED, PublicationRequest, REQUESTER_UNAUTHORIZED,
    error_code, is_transient, revision_not_found,
};
use crate::service::publish_permission;
use crate::store;
use aphel_contract_publication::{
    ACTIVATE_MOMENT, ACTIVATED, MODULE, REQUEST_BLOCKED, REQUEST_FAILED, REQUEST_SUPERSEDED,
    RequestAction, RequestStatus, STATE_PUBLISHED, STATE_WITHDRAWN, WITHDRAWN,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::approvals::{
    APPROVAL_PENDING, APPROVAL_REQUIRED, ApprovalCheckpoint, ApprovalMoment, ApprovalSubmission,
    SubmissionOutcome, approval_gate,
};
use aphel_sdk::platform::application::access::{AccessRepository, Authorizer, build_access_scope};
use aphel_sdk::platform::application::auth::UserRepository;
use aphel_sdk::platform::application::shared::{Actor, ExecutionContext};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_sdk::platform::infrastructure::repositories::mysql_users::MySqlUserRepository;
use aphel_sdk::records::RecordCtx;

const APPROVALS_MODULE: &str = "approvals";

fn module_scope() -> AccessScope {
    AccessScope::system().acting_as("module", MODULE)
}

async fn emit(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    request: &PublicationRequest,
    event_type: &'static str,
    extra: Value,
) -> Result<(), AppError> {
    let mut payload = json!({
        "request_id": request.id.to_string(),
        "action": request.action.as_str(),
        "revision_id": request.revision_id.map(|id| id.to_string()),
        "generation": request.generation,
        "requested_by": request.requested_by,
    });
    if let (Some(target), Some(source)) = (payload.as_object_mut(), extra.as_object()) {
        target.extend(source.clone());
    }
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: request.record.clone(),
            event_type,
            payload,
            changes: Vec::new(),
            ip: None,
        },
    )
    .await
}

pub(crate) async fn settle(
    request_id: Uuid,
    status: RequestStatus,
    reason: &str,
) -> Result<(), AppError> {
    let state = aphel_sdk::runtime()?;
    let scope = module_scope();
    let ctx = Ctx::from_state(state.as_ref(), &scope).await?;
    let mut tx = ctx.begin().await?;
    let Some(mut request) = store::lock_request(&mut tx, request_id).await? else {
        return Ok(());
    };
    if !request.status.is_open() {
        return Ok(());
    }
    request.status = status;
    request.reason_code = Some(reason.chars().take(64).collect());
    request.completed_at = Some(ctx.now());
    store::save_request(&mut tx, &request).await?;
    let event = if status == RequestStatus::Failed {
        REQUEST_FAILED
    } else {
        REQUEST_BLOCKED
    };
    emit(
        &ctx,
        &mut tx,
        &request,
        event,
        json!({ "reason": request.reason_code }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn requester_scope(
    ctx: &Ctx<'_>,
    request: &PublicationRequest,
    inline: Option<AccessScope>,
) -> Result<Option<AccessScope>, AppError> {
    if let Some(scope) = inline {
        return Ok(Some(scope));
    }
    let Some(user) = request
        .requested_by
        .as_deref()
        .and_then(|id| UserId::new(id).ok())
    else {
        return Ok(None);
    };
    if MySqlUserRepository::new(ctx.pool().clone())
        .find_by_id(&user)
        .await?
        .filter(|found| found.is_active())
        .is_none()
    {
        return Ok(None);
    }
    Ok(MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&user)
        .await?
        .map(|access| build_access_scope(user, access)))
}

enum Step {
    Done(RequestStatus),
    Settle(RequestStatus, String),
}

pub(crate) async fn process(
    request_id: Uuid,
    inline: Option<AccessScope>,
) -> Result<RequestStatus, AppError> {
    match attempt(request_id, inline).await {
        Ok(Step::Done(status)) => Ok(status),
        Ok(Step::Settle(status, reason)) => {
            settle(request_id, status, &reason).await?;
            Ok(status)
        }
        Err(error) if is_transient(&error) => Err(error),
        Err(error) => {
            settle(request_id, RequestStatus::Blocked, error_code(&error)).await?;
            Ok(RequestStatus::Blocked)
        }
    }
}

async fn attempt(request_id: Uuid, inline: Option<AccessScope>) -> Result<Step, AppError> {
    let state = aphel_sdk::runtime()?;
    let system = module_scope();
    let base = Ctx::from_state(state.as_ref(), &system).await?;
    let mut tx = base.begin().await?;
    let Some(mut request) = store::lock_request(&mut tx, request_id).await? else {
        return Ok(Step::Done(RequestStatus::Cancelled));
    };
    match request.status {
        RequestStatus::Ready => {}
        RequestStatus::Scheduled if request.run_at.is_none_or(|at| at <= base.now()) => {}
        other => return Ok(Step::Done(other)),
    }
    let subject = store::lock_subject(&mut tx, &request.record).await?;
    if subject.generation != request.generation
        || subject.active_revision_id != request.expected_active_revision_id
    {
        request.status = RequestStatus::Superseded;
        request.completed_at = Some(base.now());
        store::save_request(&mut tx, &request).await?;
        emit(&base, &mut tx, &request, REQUEST_SUPERSEDED, json!({})).await?;
        tx.commit().await?;
        return Ok(Step::Done(RequestStatus::Superseded));
    }
    let Some(requester) = requester_scope(&base, &request, inline).await? else {
        return Ok(Step::Settle(
            RequestStatus::Blocked,
            REQUESTER_UNAUTHORIZED.to_owned(),
        ));
    };
    let definition = definitions::find(base.registry(), request.record.record_type.as_str())?;
    if Authorizer::require(&requester, &publish_permission(&definition)?).is_err() {
        return Ok(Step::Settle(
            RequestStatus::Blocked,
            REQUESTER_UNAUTHORIZED.to_owned(),
        ));
    }
    let actor = Actor::from_scope(&requester);
    let scope = requester.acting_as("module", MODULE);
    let ctx = Ctx::from_state(state.as_ref(), &scope).await?;
    let now = ctx.now();
    let timestamp = now.to_rfc3339_opts(SecondsFormat::Micros, true);
    let mut values = Map::new();
    let mut revision = None;
    match request.action {
        RequestAction::Activate => {
            let revision_id = request.revision_id.ok_or_else(revision_not_found)?;
            let found = store::revision_in(&mut tx, revision_id)
                .await?
                .filter(|found| found.record == request.record)
                .ok_or_else(revision_not_found)?;
            if base.app().is_enabled(APPROVALS_MODULE) {
                let gate = approval_gate(&ctx).ok_or_else(|| AppError::Coded {
                    kind: aphel_kernel::errors::CodedKind::Unavailable,
                    code: APPROVALS_UNAVAILABLE,
                    message: "approvals are enabled but cannot be evaluated".to_owned(),
                })?;
                let checkpoint = ApprovalCheckpoint::new(
                    request.record.clone(),
                    ApprovalMoment::from_static(ACTIVATE_MOMENT),
                    Some(revision_id),
                );
                match gate.checkpoint(&ctx, &mut tx, &checkpoint).await {
                    Ok(()) => {}
                    Err(error) if error_code(&error) == APPROVAL_PENDING => {
                        return await_approval(tx, request).await;
                    }
                    Err(error) if error_code(&error) == APPROVAL_REQUIRED => {
                        let outcome = gate
                            .submit(
                                &ctx,
                                &mut tx,
                                &ApprovalSubmission {
                                    checkpoint: checkpoint.clone(),
                                    comment: None,
                                    ip: None,
                                },
                            )
                            .await?;
                        match outcome {
                            SubmissionOutcome::Approved(_) | SubmissionOutcome::NotRequired => {
                                gate.checkpoint(&ctx, &mut tx, &checkpoint).await?;
                            }
                            SubmissionOutcome::Pending(_) => {
                                return await_approval(tx, request).await;
                            }
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
            let fields = found
                .payload
                .get("fields")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for mapping in &definition.fields {
                values.insert(
                    mapping.published.clone(),
                    fields.get(&mapping.draft).cloned().unwrap_or(Value::Null),
                );
            }
            values.insert(definition.state.clone(), json!(STATE_PUBLISHED));
            values.insert(definition.revised_at.clone(), json!(timestamp));
            if subject.first_published_at.is_none() {
                values.insert(definition.published_at.clone(), json!(timestamp));
            }
            revision = Some(found);
        }
        RequestAction::Withdraw => {
            if subject.active_revision_id.is_none() {
                request.status = RequestStatus::Completed;
                request.reason_code = Some(NOTHING_PUBLISHED.to_owned());
                request.completed_at = Some(now);
                store::save_request(&mut tx, &request).await?;
                tx.commit().await?;
                return Ok(Step::Done(RequestStatus::Completed));
            }
            values.insert(definition.state.clone(), json!(STATE_WITHDRAWN));
        }
    }
    let execution = ExecutionContext::module(MODULE, "publication.activate", Some(&actor))?;
    let handler = ctx
        .registry()
        .handler(request.record.record_type.as_str())?;
    let record_ctx = RecordCtx::from_runtime(&scope, &state);
    let pending = execution
        .scope(handler.update_in_tx(
            &record_ctx,
            &mut tx,
            request.record.record_id,
            Value::Object(values),
        ))
        .await?
        .ok_or_else(crate::domain::record_not_found)?;
    let active = revision.as_ref().map(|found| found.id);
    store::set_active(
        &mut tx,
        &request.record,
        active,
        active.map(|_| subject.first_published_at.unwrap_or(now)),
        now,
    )
    .await?;
    request.status = RequestStatus::Completed;
    request.completed_at = Some(now);
    request.attempts += 1;
    store::save_request(&mut tx, &request).await?;
    let event = match request.action {
        RequestAction::Activate => ACTIVATED,
        RequestAction::Withdraw => WITHDRAWN,
    };
    emit(
        &ctx,
        &mut tx,
        &request,
        event,
        json!({ "payload_hash": revision.as_ref().map(|found| found.payload_hash.clone()) }),
    )
    .await?;
    tx.commit().await?;
    pending.committed(&ctx).await;
    Ok(Step::Done(RequestStatus::Completed))
}

async fn await_approval(
    mut tx: MySqlTx,
    mut request: PublicationRequest,
) -> Result<Step, AppError> {
    request.status = RequestStatus::AwaitingApproval;
    store::save_request(&mut tx, &request).await?;
    tx.commit().await?;
    Ok(Step::Done(RequestStatus::AwaitingApproval))
}
