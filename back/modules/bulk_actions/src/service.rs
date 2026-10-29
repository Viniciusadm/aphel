use serde_json::{Value, json};
use uuid::Uuid;

use crate::store::BulkOperationStore;
use crate::targets;
use crate::{
    BULK_EMPTY_SELECTION, BULK_INLINE_LIMIT, BULK_NOT_SUPPORTED, BULK_OPERATION_COMPLETED,
    BULK_OPERATION_FAILED, BULK_OPERATION_RECORD_TYPE, BULK_OPERATION_REQUESTED, BULK_PAGE_SIZE,
    BULK_RUN_JOB, BULK_TOO_LARGE, BulkAction, BulkExportCommand, BulkItem, BulkOperation,
    BulkQuery, BulkSelection, BulkStatus, ItemOutcome, MAX_BULK_RECORDS, MAX_SELECTED_IDS,
    RunBulkRequest, StartBulkCommand, bulk_capabilities, bulk_operation_not_found, classify_error,
    is_bulk_field,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::fields::permissions::{
    NOT_ACCESSIBLE_MESSAGE, READ_ONLY_MESSAGE, not_accessible, read_only_error,
};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, FilterOperator, ID_FIELD, MAX_LIST_VALUES, RecordQuery,
    RecordQueryEngine, RecordQueryPorts,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, Job, JobQueue, RecordRef,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::repositories::mysql_access::build_requester_scope;
use aphel_kernel::records::RecordCtx;
use exports::{self, ExportJob, ExportQuery, RequestExportCommand, system_scope};

fn require_bulk_type(ctx: &Ctx<'_>, record_type: &str) -> Result<(), AppError> {
    if bulk_capabilities(ctx.registry(), record_type).is_some() {
        Ok(())
    } else {
        Err(AppError::validation_code(
            BULK_NOT_SUPPORTED,
            format!("{record_type} records do not support bulk actions"),
        ))
    }
}

fn record_query(query: &BulkQuery, window: ListWindow) -> RecordQuery {
    RecordQuery {
        filter: query.filter.clone(),
        sort: Vec::new(),
        search: query.search.clone(),
        archived: query.archived,
        window,
    }
}

fn dedupe(ids: &[Uuid]) -> Vec<Uuid> {
    let mut unique = Vec::with_capacity(ids.len());
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if seen.insert(*id) {
            unique.push(*id);
        }
    }
    unique
}

fn empty_selection() -> AppError {
    AppError::validation_code(BULK_EMPTY_SELECTION, "no records were selected")
}

fn too_large() -> AppError {
    AppError::validation_code(
        BULK_TOO_LARGE,
        format!("bulk actions are limited to {MAX_BULK_RECORDS} records; narrow the filter"),
    )
}

pub(crate) async fn start(
    ctx: &Ctx<'_>,
    command: StartBulkCommand,
) -> Result<BulkOperation, AppError> {
    let scope = ctx.scope;
    let record_type = command.record_type.trim().to_owned();
    require_bulk_type(ctx, &record_type)?;
    let action = prepare_action(ctx, &record_type, command.action).await?;
    let (selection, total) = prepare_selection(ctx, &record_type, command.selection).await?;
    let background = total as usize > BULK_INLINE_LIMIT;
    let now = ctx.now();
    let mut operation = BulkOperation {
        id: Uuid::now_v7(),
        record_type,
        action,
        selection,
        status: if background {
            BulkStatus::Queued
        } else {
            BulkStatus::Running
        },
        total,
        processed: 0,
        succeeded: 0,
        skipped: 0,
        failed: 0,
        failure_reason: None,
        requested_by: scope.user_id().clone(),
        created_at: now,
        updated_at: now,
        finished_at: None,
    };
    let mut tx = ctx.begin().await?;
    BulkOperationStore::of(ctx)
        .insert(&mut tx, &operation)
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::from_scope(scope), BULK_OPERATION_REQUESTED, now)
                .with_record(RecordRef::new(BULK_OPERATION_RECORD_TYPE, operation.id))
                .with_changes(vec![
                    FieldChange::new("record_type", Value::Null, json!(operation.record_type)),
                    FieldChange::new("action", Value::Null, json!(operation.action)),
                    FieldChange::new("total", Value::Null, json!(operation.total)),
                ])
                .with_ip(command.ip),
        )
        .await?;
    if background {
        let request = RunBulkRequest {
            operation_id: operation.id,
            requested_by: operation.requested_by.clone(),
            api_token_id: scope.api_token_id().and_then(|id| Uuid::parse_str(id).ok()),
        };
        ctx.jobs()
            .enqueue(
                &mut tx,
                Job::new(BULK_RUN_JOB, request.payload())
                    .with_unique_key(format!("bulk:{}", operation.id)),
            )
            .await?;
    }
    tx.commit().await?;
    if background {
        return Ok(operation);
    }
    if let Err(error) = process(ctx, &mut operation, false).await {
        operation.status = BulkStatus::Failed;
        operation.failure_reason = Some(error.to_string().chars().take(500).collect());
        if let Err(finish_error) = finish(ctx, &mut operation, BULK_OPERATION_FAILED, false).await {
            tracing::error!(
                error = %finish_error,
                operation_id = %operation.id,
                "inline bulk operation could not be marked as failed"
            );
        }
        return Err(error);
    }
    Ok(operation)
}

pub(crate) async fn export(
    ctx: &Ctx<'_>,
    command: BulkExportCommand,
) -> Result<ExportJob, AppError> {
    let record_type = command.record_type.trim().to_owned();
    require_bulk_type(ctx, &record_type)?;
    let query = match command.selection {
        BulkSelection::Query { query } => ExportQuery {
            filter: query.filter,
            sort: Vec::new(),
            search: query.search,
            archived: query.archived,
        },
        BulkSelection::Ids { ids } => {
            let ids = dedupe(&ids);
            if ids.is_empty() {
                return Err(empty_selection());
            }
            if ids.len() > MAX_LIST_VALUES {
                return Err(AppError::validation_code(
                    BULK_TOO_LARGE,
                    format!(
                        "exports of selected records accept at most {MAX_LIST_VALUES} ids; use the filter instead"
                    ),
                ));
            }
            ExportQuery {
                filter: Some(Filter::condition(
                    ID_FIELD,
                    FilterOperator::In,
                    json!(ids.iter().map(Uuid::to_string).collect::<Vec<_>>()),
                )),
                sort: Vec::new(),
                search: None,
                archived: ArchivedFilter::Include,
            }
        }
    };
    exports::service::request(
        ctx,
        RequestExportCommand {
            record_type,
            format: command.format,
            query,
            columns: command.columns,
            ip: command.ip,
        },
    )
    .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, operation_id: &str) -> Result<BulkOperation, AppError> {
    let id = Uuid::parse_str(operation_id.trim()).map_err(|_| bulk_operation_not_found())?;
    let operation = BulkOperationStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(bulk_operation_not_found)?;
    if &operation.requested_by != ctx.scope.user_id() {
        return Err(bulk_operation_not_found());
    }
    Ok(operation)
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    window: ListWindow,
) -> Result<Listing<BulkOperation>, AppError> {
    BulkOperationStore::of(ctx)
        .list(ctx.scope.user_id(), window)
        .await
}

pub(crate) async fn items(
    ctx: &Ctx<'_>,
    operation_id: &str,
    outcome: Option<ItemOutcome>,
    window: ListWindow,
) -> Result<Listing<BulkItem>, AppError> {
    let operation = get(ctx, operation_id).await?;
    BulkOperationStore::of(ctx)
        .items(operation.id, outcome, window)
        .await
}

pub(crate) async fn run(ctx: &Ctx<'_>, request: &RunBulkRequest) -> Result<(), AppError> {
    let Some(scope) =
        build_requester_scope(ctx.pool(), &request.requested_by, request.api_token_id).await?
    else {
        return fail(
            ctx,
            request,
            "the requester is no longer an active member of the organization or the API token was revoked",
        )
        .await;
    };
    let ctx = ctx.with_scope(&scope);
    let store = BulkOperationStore::of(&ctx);
    let Some(mut operation) = store.find(request.operation_id).await? else {
        return Ok(());
    };
    if operation.status.is_finished() || operation.requested_by != request.requested_by {
        return Ok(());
    }
    if let BulkSelection::Query { query } = &operation.selection {
        let ids = collect_ids(&ctx, &operation.record_type, query).await?;
        operation.total = ids.len() as u64;
        operation.selection = BulkSelection::Ids { ids };
    }
    operation.status = BulkStatus::Running;
    operation.updated_at = ctx.now();
    store.save(&operation).await?;
    process(&ctx, &mut operation, true).await
}

pub(crate) async fn fail(
    ctx: &Ctx<'_>,
    request: &RunBulkRequest,
    reason: &str,
) -> Result<(), AppError> {
    let scope = system_scope(request.requested_by.clone())?;
    let ctx = ctx.with_scope(&scope);
    let Some(mut operation) = BulkOperationStore::of(&ctx)
        .find(request.operation_id)
        .await?
    else {
        return Ok(());
    };
    if operation.status.is_finished() {
        return Ok(());
    }
    operation.status = BulkStatus::Failed;
    operation.failure_reason = Some(reason.chars().take(500).collect());
    finish(&ctx, &mut operation, BULK_OPERATION_FAILED, true).await
}

async fn process(
    ctx: &Ctx<'_>,
    operation: &mut BulkOperation,
    background: bool,
) -> Result<(), AppError> {
    let BulkSelection::Ids { ids } = operation.selection.clone() else {
        return Err(AppError::Internal(
            "bulk selection must be resolved before processing".to_owned(),
        ));
    };
    let store = BulkOperationStore::of(ctx);
    let state = ctx.state();
    let record_ctx = RecordCtx::new(ctx.scope, &state);
    let done = store.processed_ids(operation.id).await?;
    for record_id in ids.iter().filter(|id| !done.contains(id)) {
        let item = match targets::apply(
            ctx,
            &record_ctx,
            &operation.record_type,
            *record_id,
            &operation.action,
        )
        .await
        {
            Ok(()) => {
                operation.succeeded += 1;
                BulkItem {
                    record_id: *record_id,
                    outcome: ItemOutcome::Succeeded,
                    code: None,
                    message: None,
                }
            }
            Err(error) => {
                let (outcome, code, message) = classify_error(&error);
                if outcome == ItemOutcome::Failed {
                    tracing::warn!(
                        error = %error,
                        operation_id = %operation.id,
                        record_id = %record_id,
                        "bulk action failed for a record"
                    );
                    operation.failed += 1;
                } else {
                    operation.skipped += 1;
                }
                BulkItem {
                    record_id: *record_id,
                    outcome,
                    code: Some(code),
                    message: Some(message),
                }
            }
        };
        operation.processed += 1;
        operation.updated_at = ctx.now();
        let mut tx = ctx.begin().await?;
        store.record_item(&mut tx, operation, &item).await?;
        tx.commit().await?;
    }
    operation.status = BulkStatus::Completed;
    finish(ctx, operation, BULK_OPERATION_COMPLETED, background).await
}

async fn finish(
    ctx: &Ctx<'_>,
    operation: &mut BulkOperation,
    event_type: &'static str,
    background: bool,
) -> Result<(), AppError> {
    let now = ctx.now();
    operation.finished_at = Some(now);
    operation.updated_at = now;
    let record = RecordRef::new(BULK_OPERATION_RECORD_TYPE, operation.id);
    let mut tx = ctx.begin().await?;
    BulkOperationStore::of(ctx)
        .save_in(&mut tx, operation)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(Actor::System, event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "operation_id": operation.id.to_string(),
                    "requested_by": operation.requested_by.as_str(),
                    "record_type": operation.record_type,
                    "action": operation.action.kind(),
                    "total": operation.total,
                    "succeeded": operation.succeeded,
                    "skipped": operation.skipped,
                    "failed": operation.failed,
                    "background": background,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::System, event_type, now)
                .with_record(record)
                .with_changes(vec![
                    FieldChange::new("status", Value::Null, json!(operation.status.as_str())),
                    FieldChange::new("succeeded", Value::Null, json!(operation.succeeded)),
                    FieldChange::new("skipped", Value::Null, json!(operation.skipped)),
                    FieldChange::new("failed", Value::Null, json!(operation.failed)),
                    FieldChange::new(
                        "failure_reason",
                        Value::Null,
                        json!(operation.failure_reason),
                    ),
                ]),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn prepare_action(
    ctx: &Ctx<'_>,
    record_type: &str,
    action: BulkAction,
) -> Result<BulkAction, AppError> {
    ctx.scope
        .require(&action.required_permission(ctx.registry(), record_type)?)?;
    let invalid = |field: &str, message: &str| {
        AppError::invalid_fields(
            "invalid bulk action",
            vec![FieldViolation::new(field.to_owned(), message.to_owned())],
        )
    };
    match action {
        BulkAction::MoveStage { .. }
            if !bulk_capabilities(ctx.registry(), record_type)
                .is_some_and(|capabilities| capabilities.supports_stages) =>
        {
            Err(invalid("action", "move_stage is only available for deals"))
        }
        BulkAction::EnrollInSequence { .. }
            if !bulk_capabilities(ctx.registry(), record_type)
                .is_some_and(|capabilities| capabilities.supports_sequences) =>
        {
            Err(invalid(
                "action",
                "enroll_in_sequence is only available for contacts",
            ))
        }
        BulkAction::EnrollInSequence { sequence_id } => {
            let sequence_id = sequence_id.trim().to_owned();
            if sequence_id.is_empty() {
                return Err(invalid("sequence_id", "is required"));
            }
            Ok(BulkAction::EnrollInSequence { sequence_id })
        }
        BulkAction::AssignOwner { owner_id } => {
            let owner_id = owner_id.trim().to_owned();
            if owner_id.is_empty() {
                return Err(invalid("owner_id", "is required"));
            }
            Ok(BulkAction::AssignOwner { owner_id })
        }
        BulkAction::SetField { field, value, .. } => {
            let fields = field_set(ctx, record_type).await?;
            let field = field.trim().to_owned();
            if fields.is_hidden(&field) {
                return Err(not_accessible(vec![FieldViolation::new(
                    "field",
                    NOT_ACCESSIBLE_MESSAGE,
                )]));
            }
            if fields
                .find_active(&field)
                .is_some_and(|resolved| !resolved.is_editable())
            {
                return Err(read_only_error(vec![FieldViolation::new(
                    "field",
                    READ_ONLY_MESSAGE,
                )]));
            }
            match fields.find_active(&field) {
                Some(resolved) if resolved.is_system() => {
                    if !is_bulk_field(ctx.registry(), record_type, &field) {
                        return Err(invalid("field", "cannot be changed in bulk"));
                    }
                    Ok(BulkAction::SetField {
                        field,
                        value,
                        custom: false,
                    })
                }
                Some(_) => Ok(BulkAction::SetField {
                    field,
                    value,
                    custom: true,
                }),
                None => Err(invalid("field", "is not a known field")),
            }
        }
        BulkAction::AddTag { tag_id } => Ok(BulkAction::AddTag {
            tag_id: tag_id.trim().to_owned(),
        }),
        BulkAction::RemoveTag { tag_id } => Ok(BulkAction::RemoveTag {
            tag_id: tag_id.trim().to_owned(),
        }),
        BulkAction::MoveStage { stage_id } => Ok(BulkAction::MoveStage {
            stage_id: stage_id.trim().to_owned(),
        }),
        other => Ok(other),
    }
}

async fn prepare_selection(
    ctx: &Ctx<'_>,
    record_type: &str,
    selection: BulkSelection,
) -> Result<(BulkSelection, u64), AppError> {
    let definition = ctx.registry().require(record_type)?;
    ctx.scope.record_filter(&definition.read_permission)?;
    match selection {
        BulkSelection::Ids { ids } => {
            let ids = dedupe(&ids);
            if ids.is_empty() {
                return Err(empty_selection());
            }
            if ids.len() > MAX_SELECTED_IDS {
                return Err(too_large());
            }
            let total = ids.len() as u64;
            Ok((BulkSelection::Ids { ids }, total))
        }
        BulkSelection::Query { query } => {
            let probe = RecordQueryEngine::new(&ctx.query())
                .query(
                    ctx.scope,
                    record_type,
                    record_query(
                        &query,
                        ListWindow {
                            offset: 0,
                            limit: 1,
                        },
                    ),
                )
                .await?;
            if probe.total == 0 {
                return Err(empty_selection());
            }
            if probe.total > MAX_BULK_RECORDS {
                return Err(too_large());
            }
            if probe.total as usize > BULK_INLINE_LIMIT {
                return Ok((BulkSelection::Query { query }, probe.total));
            }
            let ids = collect_ids(ctx, record_type, &query).await?;
            let total = ids.len() as u64;
            Ok((BulkSelection::Ids { ids }, total))
        }
    }
}

async fn collect_ids(
    ctx: &Ctx<'_>,
    record_type: &str,
    query: &BulkQuery,
) -> Result<Vec<Uuid>, AppError> {
    let ports = ctx.query();
    let engine = RecordQueryEngine::new(&ports);
    let mut ids = Vec::new();
    let mut offset = 0;
    loop {
        let page = engine
            .query(
                ctx.scope,
                record_type,
                record_query(
                    query,
                    ListWindow {
                        offset,
                        limit: BULK_PAGE_SIZE,
                    },
                ),
            )
            .await?;
        if page.total > MAX_BULK_RECORDS {
            return Err(too_large());
        }
        if page.items.is_empty() {
            break;
        }
        offset += page.items.len() as u64;
        ids.extend(page.items);
        if offset >= page.total {
            break;
        }
    }
    Ok(dedupe(&ids))
}

async fn field_set(ctx: &Ctx<'_>, record_type: &str) -> Result<FieldSet, AppError> {
    let query = ctx.query();
    let definition = query.registry().require(record_type)?;
    FieldSet::for_scope(query.definitions(), definition, ctx.scope).await
}
