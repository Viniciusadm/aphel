use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::store::{ExportStore, RecordLabelStore, RecordRowStore};
use crate::{
    CSV_MIME_TYPE, ColumnSpec, EXPORT_COMPLETED, EXPORT_FAILED, EXPORT_PAGE_SIZE,
    EXPORT_RECORD_TYPE, EXPORT_REQUESTED, EXPORT_RUN_JOB, ExportCell, ExportFormat, ExportJob,
    ExportQuery, ExportStatus, GeneratedFile, ID_COLUMN, LabelSet, MAX_EXPORT_ROWS,
    RequestExportCommand, RunExportRequest, XLSX_MIME_TYPE, collect_references,
    discard_generated_file, encode_csv, encode_xlsx, export_permission, format_cell, is_exportable,
    resolve_columns, store_generated_file, system_scope,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::{AccessRepository, Authorizer, build_access_scope};
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, ID_FIELD, MAX_LIST_VALUES, RecordQuery,
    RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts,
};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, Job, JobQueue, RecordRef,
};
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_sdk::platform::infrastructure::storage::MySqlFileRepository;

pub const EXPORT_NOT_FOUND: &str = "export_not_found";
pub const EXPORT_NOT_SUPPORTED: &str = "export_not_supported";
pub const EXPORT_TOO_LARGE: &str = "export_too_large";

pub fn export_not_found() -> AppError {
    AppError::not_found_code(EXPORT_NOT_FOUND, "export was not found")
}

fn require_exportable(scope: &AccessScope, record_type: &str) -> Result<(), AppError> {
    if !is_exportable(record_type) {
        return Err(AppError::validation_code(
            EXPORT_NOT_SUPPORTED,
            format!("{record_type} records cannot be exported"),
        ));
    }
    let permission = export_permission(record_type).ok_or_else(|| {
        AppError::validation_code(EXPORT_NOT_SUPPORTED, "unsupported record type")
    })?;
    scope.require(&permission).map(|_| ())
}

fn record_query(query: &ExportQuery, window: ListWindow) -> RecordQuery {
    RecordQuery {
        filter: query.filter.clone(),
        sort: query.sort.clone(),
        search: query.search.clone(),
        archived: query.archived,
        window,
    }
}

pub async fn request(ctx: &Ctx<'_>, command: RequestExportCommand) -> Result<ExportJob, AppError> {
    let scope = ctx.scope;
    require_exportable(scope, &command.record_type)?;
    let fields = field_set(ctx, &command.record_type).await?;
    let columns = resolve_columns(&fields, &command.columns)?;
    let query = ctx.query();
    let probe = RecordQueryEngine::new(&query)
        .query(
            scope,
            &command.record_type,
            record_query(
                &command.query,
                ListWindow {
                    offset: 0,
                    limit: 1,
                },
            ),
        )
        .await?;
    if probe.total > MAX_EXPORT_ROWS {
        return Err(AppError::validation_code(
            EXPORT_TOO_LARGE,
            format!("exports are limited to {MAX_EXPORT_ROWS} records; narrow the filter"),
        ));
    }

    let now = ctx.now();
    let job = ExportJob {
        id: Uuid::now_v7(),
        record_type: command.record_type,
        format: command.format,
        status: ExportStatus::Queued,
        query: command.query,
        columns: columns.into_iter().map(|column| column.key).collect(),
        file_id: None,
        row_count: None,
        failure_reason: None,
        requested_by: scope.user_id().clone(),
        created_at: now,
        updated_at: now,
        finished_at: None,
    };
    let request = RunExportRequest {
        export_id: job.id,
        requested_by: job.requested_by.clone(),
    };
    let mut tx = ctx.begin().await?;
    ExportStore::of(ctx).insert(&mut tx, &job).await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::from_scope(scope), EXPORT_REQUESTED, now)
                .with_record(RecordRef::new(EXPORT_RECORD_TYPE, job.id))
                .with_changes(vec![
                    FieldChange::new("record_type", Value::Null, json!(job.record_type)),
                    FieldChange::new("format", Value::Null, json!(job.format.as_str())),
                    FieldChange::new("columns", Value::Null, json!(job.columns)),
                    FieldChange::new("query", Value::Null, json!(job.query)),
                    FieldChange::new("estimated_rows", Value::Null, json!(probe.total)),
                ])
                .with_ip(command.ip),
        )
        .await?;
    ctx.jobs()
        .enqueue(
            &mut tx,
            Job::new(EXPORT_RUN_JOB, request.payload())
                .with_unique_key(format!("export:{}", job.id)),
        )
        .await?;
    tx.commit().await?;
    Ok(job)
}

pub(crate) async fn get(ctx: &Ctx<'_>, export_id: &str) -> Result<ExportJob, AppError> {
    let id = Uuid::parse_str(export_id.trim()).map_err(|_| export_not_found())?;
    let job = ExportStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(export_not_found)?;
    if &job.requested_by != ctx.scope.user_id() {
        return Err(export_not_found());
    }
    Ok(job)
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    window: ListWindow,
) -> Result<Listing<ExportJob>, AppError> {
    ExportStore::of(ctx).list(ctx.scope.user_id(), window).await
}

pub(crate) async fn run(ctx: &Ctx<'_>, request: &RunExportRequest) -> Result<(), AppError> {
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&request.requested_by)
        .await?
    else {
        return fail(
            ctx,
            request,
            "the requester is no longer an active member of the organization",
        )
        .await;
    };
    let scope = build_access_scope(request.requested_by.clone(), access);
    let ctx = ctx.with_scope(&scope);
    let ctx = &ctx;
    let store = ExportStore::of(ctx);
    let Some(mut job) = store.find(request.export_id).await? else {
        return Ok(());
    };
    if job.status.is_finished() || job.requested_by != request.requested_by {
        return Ok(());
    }
    if require_exportable(&scope, &job.record_type).is_err() {
        return fail(
            ctx,
            request,
            "the requester can no longer export these records",
        )
        .await;
    }
    let now = ctx.now();
    job.status = ExportStatus::Running;
    job.updated_at = now;
    let mut tx = ctx.begin().await?;
    store.save(&mut tx, &job).await?;
    tx.commit().await?;

    let fields = field_set(ctx, &job.record_type).await?;
    let columns = match resolve_columns(&fields, &job.columns) {
        Ok(columns) => columns,
        Err(_) => {
            let available: Vec<String> = job
                .columns
                .iter()
                .filter(|key| key.as_str() == ID_COLUMN || fields.find_active(key).is_some())
                .cloned()
                .collect();
            if available.is_empty() {
                return fail(
                    ctx,
                    request,
                    "the requested columns are no longer accessible",
                )
                .await;
            }
            resolve_columns(&fields, &available)?
        }
    };
    let rows = match collect_rows(ctx, &job, &columns).await {
        Ok(rows) => rows,
        Err(error @ (AppError::InvalidFields { .. } | AppError::Coded { .. })) => {
            return fail(ctx, request, &error.to_string()).await;
        }
        Err(error) => return Err(error),
    };
    let headers: Vec<String> = columns.iter().map(|column| column.label.clone()).collect();
    let (content, mime_type) = match job.format {
        ExportFormat::Csv => (encode_csv(&headers, &rows, b',')?, CSV_MIME_TYPE),
        ExportFormat::Xlsx => (encode_xlsx(&headers, &rows)?, XLSX_MIME_TYPE),
    };
    let now = ctx.now();
    let file = GeneratedFile {
        name: format!(
            "{}-{}.{}",
            job.record_type,
            now.format("%Y%m%d-%H%M%S"),
            job.format.as_str()
        ),
        mime_type: mime_type.to_owned(),
        content,
    };
    let row_count = rows.len() as u64;
    let record = RecordRef::new(EXPORT_RECORD_TYPE, job.id);
    let storage = ctx.storage()?;
    let files = MySqlFileRepository::new(ctx.pool().clone());
    let mut tx = ctx.begin().await?;
    let stored = store_generated_file(&files, storage.as_ref(), &mut tx, &scope, file, now).await?;
    job.status = ExportStatus::Completed;
    job.file_id = Some(stored.file_id);
    job.row_count = Some(row_count);
    job.finished_at = Some(now);
    job.updated_at = now;
    let result = async {
        store.save(&mut tx, &job).await?;
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(Actor::System, EXPORT_COMPLETED, now)
                    .with_record(record.clone())
                    .with_payload(json!({
                        "export_id": job.id.to_string(),
                        "requested_by": job.requested_by.as_str(),
                        "record_type": job.record_type,
                        "format": job.format.as_str(),
                        "file_id": stored.file_id.to_string(),
                        "row_count": row_count,
                    })),
            )
            .await?;
        ctx.audit()
            .record(
                &mut tx,
                AuditEntry::new(Actor::System, EXPORT_COMPLETED, now)
                    .with_record(record)
                    .with_changes(vec![
                        FieldChange::new("file_id", Value::Null, json!(stored.file_id.to_string())),
                        FieldChange::new("row_count", Value::Null, json!(row_count)),
                    ]),
            )
            .await?;
        tx.commit().await?;
        Ok::<(), AppError>(())
    }
    .await;
    if let Err(error) = result {
        discard_generated_file(storage.as_ref(), &stored.storage_key).await;
        return Err(error);
    }
    Ok(())
}

pub(crate) async fn fail(
    ctx: &Ctx<'_>,
    request: &RunExportRequest,
    reason: &str,
) -> Result<(), AppError> {
    let scope = system_scope(request.requested_by.clone())?;
    let ctx = ctx.with_scope(&scope);
    let store = ExportStore::of(&ctx);
    let Some(mut job) = store.find(request.export_id).await? else {
        return Ok(());
    };
    if job.status.is_finished() {
        return Ok(());
    }
    let now = ctx.now();
    job.status = ExportStatus::Failed;
    job.failure_reason = Some(reason.chars().take(500).collect());
    job.finished_at = Some(now);
    job.updated_at = now;
    let record = RecordRef::new(EXPORT_RECORD_TYPE, job.id);
    let mut tx = ctx.begin().await?;
    store.save(&mut tx, &job).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(Actor::System, EXPORT_FAILED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "export_id": job.id.to_string(),
                    "requested_by": job.requested_by.as_str(),
                    "record_type": job.record_type,
                    "format": job.format.as_str(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::System, EXPORT_FAILED, now)
                .with_record(record)
                .with_changes(vec![FieldChange::new(
                    "failure_reason",
                    Value::Null,
                    json!(job.failure_reason),
                )]),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn collect_rows(
    ctx: &Ctx<'_>,
    job: &ExportJob,
    columns: &[ColumnSpec],
) -> Result<Vec<Vec<ExportCell>>, AppError> {
    let scope = ctx.scope;
    let query = ctx.query();
    let engine = RecordQueryEngine::new(&query);
    let fields = field_set(ctx, &job.record_type).await?;
    let timezone = query.executor().timezone().await?;
    let rows_store = RecordRowStore::of(ctx);
    let labels_store = RecordLabelStore::of(ctx);
    let snapshot = engine
        .query(
            scope,
            &job.record_type,
            record_query(
                &job.query,
                ListWindow {
                    offset: 0,
                    limit: MAX_EXPORT_ROWS + 1,
                },
            ),
        )
        .await?;
    if snapshot.total > MAX_EXPORT_ROWS || snapshot.items.len() as u64 > MAX_EXPORT_ROWS {
        return Err(AppError::validation_code(
            EXPORT_TOO_LARGE,
            format!("exports are limited to {MAX_EXPORT_ROWS} records"),
        ));
    }
    let mut rows = Vec::with_capacity(snapshot.items.len());
    for chunk in snapshot.items.chunks(EXPORT_PAGE_SIZE as usize) {
        let loaded = rows_store.load_rows(&fields, chunk).await?;
        let ordered: Vec<(Uuid, Map<String, Value>)> = chunk
            .iter()
            .filter_map(|id| loaded.get(id).map(|values| (*id, values.clone())))
            .collect();
        let values: Vec<_> = ordered.iter().map(|(_, values)| values.clone()).collect();
        let labels = labels(ctx, &labels_store, columns, &values).await?;
        for (id, values) in &ordered {
            rows.push(
                columns
                    .iter()
                    .map(|column| {
                        format_cell(column, *id, values.get(&column.key), &labels, timezone)
                    })
                    .collect(),
            );
        }
    }
    Ok(rows)
}

async fn labels(
    ctx: &Ctx<'_>,
    store: &RecordLabelStore,
    columns: &[ColumnSpec],
    rows: &[Map<String, Value>],
) -> Result<LabelSet, AppError> {
    let references = collect_references(columns, rows);
    let mut labels = LabelSet::default();
    if !references.users.is_empty() {
        let ids: Vec<String> = references.users.into_iter().collect();
        labels.users = store.user_labels(&ids).await?;
    }
    for (record_type, ids) in references.records {
        let ids: Vec<Uuid> = ids.into_iter().collect();
        let ids = if ctx.registry().require(&record_type).is_ok() {
            readable_related(ctx, &record_type, &ids).await?
        } else {
            ids
        };
        if ids.is_empty() {
            continue;
        }
        for (id, label) in store.record_labels(&record_type, &ids).await? {
            labels.records.insert((record_type.clone(), id), label);
        }
    }
    Ok(labels)
}

async fn readable_related(
    ctx: &Ctx<'_>,
    record_type: &str,
    ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    let query = ctx.query();
    let definition = query.registry().require(record_type)?;
    if ctx.scope.filter_for(&definition.read_permission).is_none() {
        return Ok(Vec::new());
    }
    let Some(title_field) = definition.title_field.clone() else {
        return Ok(Vec::new());
    };
    let fields = FieldSet::for_scope(query.definitions(), definition, ctx.scope).await?;
    if fields.is_hidden(&title_field) {
        return Ok(Vec::new());
    }
    let engine = RecordQueryEngine::new(&query);
    let mut readable = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(MAX_LIST_VALUES) {
        let page = engine
            .query(
                ctx.scope,
                record_type,
                RecordQuery {
                    filter: Some(Filter::condition(
                        ID_FIELD,
                        FilterOperator::In,
                        json!(chunk.iter().map(Uuid::to_string).collect::<Vec<_>>()),
                    )),
                    sort: Vec::new(),
                    search: None,
                    archived: ArchivedFilter::Include,
                    window: ListWindow {
                        offset: 0,
                        limit: chunk.len() as u64,
                    },
                },
            )
            .await;
        match page {
            Ok(page) => readable.extend(page.items),
            Err(error) if is_access_error(&error) => return Ok(Vec::new()),
            Err(error) => return Err(error),
        }
    }
    Ok(readable)
}

fn is_access_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Forbidden(_)
            | AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Coded { .. }
    )
}

async fn field_set(ctx: &Ctx<'_>, record_type: &str) -> Result<FieldSet, AppError> {
    let query = ctx.query();
    let definition = query.registry().require(record_type)?;
    FieldSet::for_scope(query.definitions(), definition, ctx.scope).await
}
