use aphel_sdk::imports::ImportTraits;
use std::collections::HashMap;
use std::time::Instant;

use futures::{StreamExt, stream};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::hooks::{WritingRow, writing_row};
use crate::service::evaluate::{
    CreationPlan, PlannedRow, PreparedRow, ResolutionCache, ResolveMode, RowEvaluator, describe,
    duplicate_value, file_stem,
};
use crate::service::{
    IMPORT_WRITE_CONCURRENCY, RUN_TIME_BUDGET, RunImportRequest, RunProgress, field_set,
    import_targets, is_importable, is_row_error, require_importable, timezone,
};
use crate::store::{ImportStore, RecordLookups};
use crate::{
    DuplicateKey, DuplicateStrategy, IMPORT_BATCH_SIZE, IMPORT_COMPLETED, IMPORT_FAILED,
    IMPORT_RECORD_TYPE, ImportJob, ImportRow, ImportStatus, RecordValues, RowAction, RowError,
    RowOutcome, RowStatus, create_record, normalize_label, update_record,
};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, JobQueue, RecordRef,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::build_requester_scope;
use aphel_kernel::infrastructure::storage::MySqlFileRepository;
use aphel_kernel::records::normalize_domain;
use exports::{
    CSV_MIME_TYPE, ExportCell, GeneratedFile, discard_generated_file, encode_csv,
    store_generated_file, system_scope,
};

const ERROR_COLUMN: &str = "Erros";
const ERROR_REPORT_PAGE: u64 = 1000;

pub(crate) async fn run(
    ctx: &Ctx<'_>,
    request: &RunImportRequest,
) -> Result<RunProgress, AppError> {
    let Some(scope) =
        build_requester_scope(ctx.pool(), &request.requested_by, request.api_token_id).await?
    else {
        fail(
            ctx,
            request,
            "the requester is no longer an active member of the organization or the API token was revoked",
        )
        .await?;
        return Ok(RunProgress::Finished);
    };
    let ctx = &ctx.with_scope(&scope);
    let store = ImportStore::of(ctx);
    let Some(mut job) = store.find_job(request.import_id).await? else {
        return Ok(RunProgress::Finished);
    };
    if job.created_by != request.requested_by
        || job.privacy_redacted_at.is_some()
        || matches!(job.status, ImportStatus::Draft)
        || job.status.is_finished()
    {
        return Ok(RunProgress::Finished);
    }
    if require_importable(ctx, &job.record_type).is_err() {
        fail(
            ctx,
            request,
            "the requester can no longer create these records",
        )
        .await?;
        return Ok(RunProgress::Finished);
    }
    if job.status == ImportStatus::Queued {
        let now = ctx.now();
        job.status = ImportStatus::Running;
        job.started_at.get_or_insert(now);
        job.updated_at = now;
        let mut tx = ctx.begin().await?;
        store.save_job(&mut tx, &job).await?;
        tx.commit().await?;
    }

    let fields = field_set(ctx, &job.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    let evaluator = RowEvaluator {
        fields: &fields,
        targets: &targets,
        headers: &job.headers,
        mapping: &job.mapping,
        timezone: timezone(ctx).await?,
    };
    let started = Instant::now();
    let mut cache = ResolutionCache::default();
    loop {
        let rows = store.pending_rows(job.id, IMPORT_BATCH_SIZE).await?;
        if rows.is_empty() {
            finalize(ctx, job).await?;
            return Ok(RunProgress::Finished);
        }
        let mut groups: Vec<Vec<PreparedRow>> = Vec::new();
        let mut group_of: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            let prepared = prepare_row(ctx, &job, &evaluator, row, &mut cache).await?;
            let key = match &prepared {
                PreparedRow::Ready {
                    duplicate: Some(key),
                    ..
                } => Some(key.clone()),
                _ => None,
            };
            match key {
                Some(key) => match group_of.get(&key) {
                    Some(&index) => groups[index].push(prepared),
                    None => {
                        group_of.insert(key, groups.len());
                        groups.push(vec![prepared]);
                    }
                },
                None => groups.push(vec![prepared]),
            }
        }
        let results: Vec<Result<(), AppError>> = stream::iter(
            groups
                .into_iter()
                .map(|group| write_group(ctx, &job, &evaluator, group)),
        )
        .buffer_unordered(IMPORT_WRITE_CONCURRENCY)
        .collect()
        .await;
        results.into_iter().collect::<Result<Vec<()>, AppError>>()?;
        let totals = store.count_rows(job.id).await?;
        store.update_totals(job.id, &totals).await?;
        job.totals = totals;
        if started.elapsed() > RUN_TIME_BUDGET && totals.pending > 0 {
            return Ok(RunProgress::Continue {
                processed: totals.processed(),
            });
        }
    }
}

pub(crate) async fn continue_later(
    ctx: &Ctx<'_>,
    request: &RunImportRequest,
    processed: u64,
) -> Result<(), AppError> {
    ctx.jobs()
        .enqueue_now(request.job(processed))
        .await
        .map(|_| ())
}

pub(crate) async fn fail(
    ctx: &Ctx<'_>,
    request: &RunImportRequest,
    reason: &str,
) -> Result<(), AppError> {
    let scope = system_scope(request.requested_by.clone())?;
    let ctx = &ctx.with_scope(&scope);
    let store = ImportStore::of(ctx);
    let Some(mut job) = store.find_job(request.import_id).await? else {
        return Ok(());
    };
    if job.status.is_finished() || job.status == ImportStatus::Draft {
        return Ok(());
    }
    let now = ctx.now();
    job.totals = store.count_rows(job.id).await?;
    job.status = ImportStatus::Failed;
    job.failure_reason = Some(reason.chars().take(500).collect());
    job.finished_at = Some(now);
    job.updated_at = now;
    let record = RecordRef::new(IMPORT_RECORD_TYPE, job.id);
    let mut tx = ctx.begin().await?;
    store.save_job(&mut tx, &job).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(Actor::System, IMPORT_FAILED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "import_job_id": job.id.to_string(),
                    "requested_by": job.created_by.as_str(),
                    "record_type": job.record_type,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::System, IMPORT_FAILED, now)
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

async fn finalize(ctx: &Ctx<'_>, mut job: ImportJob) -> Result<(), AppError> {
    let now = ctx.now();
    let store = ImportStore::of(ctx);
    let totals = store.count_rows(job.id).await?;
    let report = if totals.failed > 0 {
        Some(error_report(ctx, &job).await?)
    } else {
        None
    };
    let record = RecordRef::new(IMPORT_RECORD_TYPE, job.id);
    let storage = ctx.storage()?;
    let mut tx = ctx.begin().await?;
    let stored = match report {
        Some(report) => Some(
            store_generated_file(
                &MySqlFileRepository::new(ctx.pool().clone()),
                storage.as_ref(),
                &mut tx,
                ctx.scope,
                report,
                now,
            )
            .await?,
        ),
        None => None,
    };
    job.status = ImportStatus::Completed;
    job.totals = totals;
    job.error_file_id = stored.as_ref().map(|stored| stored.file_id);
    job.finished_at = Some(now);
    job.updated_at = now;
    let result = async {
        store.save_job(&mut tx, &job).await?;
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(Actor::System, IMPORT_COMPLETED, now)
                    .with_record(record.clone())
                    .with_payload(json!({
                        "import_job_id": job.id.to_string(),
                        "requested_by": job.created_by.as_str(),
                        "record_type": job.record_type,
                        "totals": {
                            "total": totals.total,
                            "imported": totals.imported,
                            "updated": totals.updated,
                            "skipped": totals.skipped,
                            "failed": totals.failed,
                        },
                    })),
            )
            .await?;
        ctx.audit()
            .record(
                &mut tx,
                AuditEntry::new(Actor::System, IMPORT_COMPLETED, now)
                    .with_record(record)
                    .with_changes(vec![FieldChange::new("totals", Value::Null, json!(totals))]),
            )
            .await?;
        tx.commit().await.map_err(AppError::from)
    }
    .await;
    if let Err(error) = result {
        if let Some(stored) = stored {
            discard_generated_file(storage.as_ref(), &stored.storage_key).await;
        }
        return Err(error);
    }
    Ok(())
}

async fn error_report(ctx: &Ctx<'_>, job: &ImportJob) -> Result<GeneratedFile, AppError> {
    let store = ImportStore::of(ctx);
    let mut headers = job.headers.clone();
    headers.push(ERROR_COLUMN.to_owned());
    let mut rows = Vec::new();
    let mut offset = 0;
    loop {
        let page = store
            .list_rows(
                job.id,
                Some(RowStatus::Failed),
                ListWindow {
                    offset,
                    limit: ERROR_REPORT_PAGE,
                },
            )
            .await?;
        let fetched = page.items.len() as u64;
        for row in page.items {
            let mut cells: Vec<ExportCell> = (0..job.headers.len())
                .map(|index| {
                    row.cells
                        .get(index)
                        .filter(|cell| !cell.is_empty())
                        .map_or(ExportCell::Empty, |cell| ExportCell::Text(cell.clone()))
                })
                .collect();
            cells.push(ExportCell::Text(
                row.errors
                    .iter()
                    .map(RowError::describe)
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
            rows.push(cells);
        }
        offset += fetched;
        if fetched < ERROR_REPORT_PAGE {
            break;
        }
    }
    Ok(GeneratedFile {
        name: format!("{}-erros.csv", file_stem(&job.file_name)),
        mime_type: CSV_MIME_TYPE.to_owned(),
        content: encode_csv(&headers, &rows, job.delimiter)?,
    })
}

async fn prepare_row(
    ctx: &Ctx<'_>,
    job: &ImportJob,
    evaluator: &RowEvaluator<'_>,
    row: &ImportRow,
    cache: &mut ResolutionCache,
) -> Result<PreparedRow, AppError> {
    let mut planned = evaluator.plan(&row.cells);
    if planned.errors.is_empty() {
        resolve(ctx, job, &mut planned, cache, ResolveMode::Run).await?;
    }
    if !planned.errors.is_empty() {
        return Ok(PreparedRow::Done(RowOutcome::failed(
            row.row_index,
            planned.errors,
        )));
    }
    let duplicate = if job.options.checks_duplicates() {
        duplicate_value(
            job.options.duplicate_key,
            &evaluator.fields.record_type,
            &planned.values,
        )
    } else {
        None
    };
    Ok(PreparedRow::Ready {
        row_index: row.row_index,
        values: planned.values,
        duplicate,
    })
}

async fn write_group(
    ctx: &Ctx<'_>,
    job: &ImportJob,
    evaluator: &RowEvaluator<'_>,
    group: Vec<PreparedRow>,
) -> Result<(), AppError> {
    let store = ImportStore::of(ctx);
    for prepared in group {
        let outcome = match prepared {
            PreparedRow::Done(outcome) => outcome,
            PreparedRow::Ready {
                row_index,
                values,
                duplicate,
            } => write_row(ctx, job, evaluator, row_index, &values, duplicate).await?,
        };
        store.finish_row(job.id, &outcome).await?;
    }
    Ok(())
}

async fn write_row(
    ctx: &Ctx<'_>,
    job: &ImportJob,
    evaluator: &RowEvaluator<'_>,
    row_index: u32,
    values: &RecordValues,
    duplicate: Option<String>,
) -> Result<RowOutcome, AppError> {
    let options = &job.options;
    let existing = match duplicate {
        Some(value) => {
            RecordLookups::of(ctx)
                .find_duplicate(
                    ctx.scope,
                    &evaluator.fields.record_type,
                    options.duplicate_key,
                    &value,
                )
                .await?
        }
        None => None,
    };
    let writing = |action: RowAction| WritingRow {
        import_id: job.id,
        row_index,
        record_type: job.record_type.clone(),
        action,
    };
    let result = match existing {
        Some(existing) if options.duplicate_strategy == DuplicateStrategy::Skip => {
            return Ok(RowOutcome::done(row_index, RowAction::Skipped, existing));
        }
        Some(existing) => writing_row(
            writing(RowAction::Updated),
            update_record(ctx, job.id, &job.record_type, existing, values),
        )
        .await
        .map(|id| (RowAction::Updated, id)),
        None => {
            let mut values = values.clone();
            if let Some(rule_id) = &options.distribution_rule_id
                && values.text("owner_id").is_none()
                && let Some(owner_id) =
                    distribute_owner(ctx, rule_id, job, row_index, &values).await?
            {
                values
                    .system
                    .insert("owner_id".to_owned(), Value::String(owner_id));
            }
            writing_row(
                writing(RowAction::Created),
                create_record(ctx, job.id, &job.record_type, &values),
            )
            .await
            .map(|id| (RowAction::Created, id))
        }
    };
    match result {
        Ok((action, id)) => Ok(RowOutcome::done(row_index, action, id)),
        Err(error) if is_row_error(&error) => {
            Ok(RowOutcome::failed(row_index, evaluator.errors_from(error)))
        }
        Err(error) => Err(error),
    }
}

pub(super) async fn resolve(
    ctx: &Ctx<'_>,
    job: &ImportJob,
    planned: &mut PlannedRow,
    cache: &mut ResolutionCache,
    mode: ResolveMode,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let lookups = RecordLookups::of(ctx);
    for user in std::mem::take(&mut planned.users) {
        let found = match cache.users.get(&user.email) {
            Some(found) => found.clone(),
            None => {
                let found = lookups
                    .find_member_by_email(&user.email)
                    .await?
                    .map(|id| id.as_str().to_owned());
                cache.users.insert(user.email.clone(), found.clone());
                found
            }
        };
        match found {
            Some(id) => planned.set(&user.key, user.is_system, Value::String(id)),
            None => planned.errors.push(RowError {
                field: Some(user.key.clone()),
                column: user.column.clone(),
                message: format!("no active member has the email {}", user.email),
            }),
        }
    }
    for record in std::mem::take(&mut planned.records) {
        let cache_key = (record.target.clone(), normalize_label(&record.label));
        let found = match cache.records.get(&cache_key) {
            Some(found) => *found,
            None => {
                let found = lookups
                    .find_record_by_label(scope, &record.target, &record.label)
                    .await?;
                if found.is_some() || mode == ResolveMode::Run {
                    cache.records.insert(cache_key.clone(), found);
                }
                found
            }
        };
        if let Some(id) = found {
            planned.set_record(&record, id);
            continue;
        }
        let creation = if job.options.create_missing_references {
            creation_plan(ctx, &record.target, cache).await?
        } else {
            None
        };
        let Some(creation) = creation else {
            planned.errors.push(RowError {
                field: Some(record.key.clone()),
                column: record.column.clone(),
                message: format!("no {} matches {:?}", record.target, record.label),
            });
            continue;
        };
        if mode == ResolveMode::Validate {
            continue;
        }
        match create_missing(ctx, job.id, &record.target, &creation, &record.label).await {
            Ok(id) => {
                cache.records.insert(cache_key, Some(id));
                planned.set_record(&record, id);
            }
            Err(error) if is_row_error(&error) => planned.errors.push(RowError {
                field: Some(record.key.clone()),
                column: record.column.clone(),
                message: format!(
                    "the {} could not be created: {}",
                    record.target,
                    describe(&error)
                ),
            }),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

async fn creation_plan(
    ctx: &Ctx<'_>,
    target: &str,
    cache: &mut ResolutionCache,
) -> Result<Option<CreationPlan>, AppError> {
    if let Some(plan) = cache.creatable.get(target) {
        return Ok(plan.clone());
    }
    let plan = build_creation_plan(ctx, target).await?;
    cache.creatable.insert(target.to_owned(), plan.clone());
    Ok(plan)
}

async fn build_creation_plan(
    ctx: &Ctx<'_>,
    target: &str,
) -> Result<Option<CreationPlan>, AppError> {
    if !is_importable(ctx, target) || require_importable(ctx, target).is_err() {
        return Ok(None);
    }
    let fields = field_set(ctx, target).await?;
    if !fields
        .record_type
        .capabilities
        .import_policy_data()?
        .label_creation
    {
        return Ok(None);
    }
    let writable = |key: &str| {
        fields
            .find_active(key)
            .filter(|field| field.is_system() && field.is_editable())
    };
    let Some(title) = fields.record_type.title_field.as_deref().and_then(writable) else {
        return Ok(None);
    };
    if fields
        .active()
        .any(|field| field.definition.is_required && field.key() != title.key())
    {
        return Ok(None);
    }
    let domain = DuplicateKey::Domain
        .columns(&fields.record_type)
        .into_iter()
        .map(|column| column.field)
        .find(|key| writable(key).is_some());
    Ok(Some(CreationPlan {
        title: title.key().to_owned(),
        domain,
    }))
}

async fn create_missing(
    ctx: &Ctx<'_>,
    import_id: Uuid,
    target: &str,
    creation: &CreationPlan,
    label: &str,
) -> Result<Uuid, AppError> {
    let label = label.trim();
    let mut values = RecordValues::default();
    values
        .system
        .insert(creation.title.clone(), Value::String(label.to_owned()));
    if let Some(domain_field) = &creation.domain
        && !label.contains(char::is_whitespace)
        && let Some(domain) = normalize_domain(label)
    {
        values
            .system
            .insert(domain_field.clone(), Value::String(domain));
    }
    create_record(ctx, import_id, target, &values).await
}

pub(super) async fn distribution_rule_usable(
    ctx: &Ctx<'_>,
    rule_id: &str,
    record_type: &str,
) -> Result<bool, AppError> {
    match assignment_selector(ctx)? {
        Some(selector) => selector.usable(ctx, rule_id, record_type).await,
        None => Ok(false),
    }
}

async fn distribute_owner(
    ctx: &Ctx<'_>,
    rule_id: &str,
    job: &ImportJob,
    row_index: u32,
    values: &RecordValues,
) -> Result<Option<String>, AppError> {
    let mut merged = values.custom.clone();
    merged.extend(values.system.clone());
    let Some(selector) = assignment_selector(ctx)? else {
        return Ok(None);
    };
    Ok(selector
        .select(
            ctx,
            rule_id,
            &job.record_type,
            &merged,
            "import",
            format!("import:{}:{row_index}", job.id),
        )
        .await?
        .map(|user| user.as_str().to_owned()))
}

fn assignment_selector(
    ctx: &Ctx<'_>,
) -> Result<Option<std::sync::Arc<dyn aphel_sdk::assignment::AssignmentSelector>>, AppError> {
    let selectors = ctx
        .app()
        .contributions::<dyn aphel_sdk::assignment::AssignmentSelector>();
    if selectors.len() > 1 {
        return Err(AppError::Configuration(
            "duplicate assignment selector".into(),
        ));
    }
    Ok(selectors.into_iter().next())
}
