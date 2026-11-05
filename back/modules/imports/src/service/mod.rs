mod evaluate;
mod run;

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use bytes::Bytes;
use chrono_tz::Tz;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::store::ImportStore;
use crate::{
    DuplicateKey, DuplicateStrategy, IMPORT_RECORD_TYPE, IMPORT_RETRIED, IMPORT_RUN_JOB,
    IMPORT_STARTED, ImportJob, ImportJobFilter, ImportOptions, ImportRow, ImportStatus,
    ImportTarget, ImportTotals, PREVIEW_ROWS, RowReport, RowStatus, ValidationReport,
    normalize_label, parse_source, source_mime_type, suggest_mapping,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::exports::{CSV_MIME_TYPE, discard_generated_file, read_file_content};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::fields::permissions::{
    NOT_ACCESSIBLE_MESSAGE, READ_ONLY_MESSAGE, query_error,
};
use aphel_sdk::platform::application::files::{
    FileRepository, NewFile, sanitize_file_name, sha256_hex, storage_key,
};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::query::{RecordQueryExecutor, RecordQueryPorts};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, FieldChange, Job, JobQueue, RecordRef,
};
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::infrastructure::storage::MySqlFileRepository;

use evaluate::{ResolutionCache, ResolveMode, RowEvaluator};

pub use evaluate::{RELATION_TARGET_PREFIX, import_targets};
pub(crate) use run::{continue_later, fail, run};

pub const IMPORT_NOT_FOUND: &str = "import_not_found";
pub const IMPORT_NOT_SUPPORTED: &str = "import_not_supported";
pub const IMPORT_NOT_EDITABLE: &str = "import_not_editable";
pub const IMPORT_NOT_RETRYABLE: &str = "import_not_retryable";
pub const MAX_REPORTED_ROWS: usize = 200;
pub const RUN_TIME_BUDGET: Duration = Duration::from_secs(120);
pub const IMPORT_WRITE_CONCURRENCY: usize = 4;
pub(crate) const GENERIC_FAILURE: &str = "the import could not be completed";

pub fn import_not_found() -> AppError {
    AppError::not_found_code(IMPORT_NOT_FOUND, "import was not found")
}

fn import_not_editable() -> AppError {
    AppError::conflict_code(
        IMPORT_NOT_EDITABLE,
        "the import was already started and can no longer be changed",
    )
}

fn import_permission(ctx: &Ctx<'_>, record_type: &str, action: &str) -> Option<Permission> {
    let definition = ctx.registry().require(record_type).ok()?;
    if !is_importable(ctx, record_type) {
        return None;
    }
    definition.action_permission(action)
}
pub fn create_permission(ctx: &Ctx<'_>, record_type: &str) -> Option<Permission> {
    import_permission(ctx, record_type, "create")
}
pub fn update_permission(ctx: &Ctx<'_>, record_type: &str) -> Option<Permission> {
    import_permission(ctx, record_type, "update")
}
pub fn is_importable(ctx: &Ctx<'_>, record_type: &str) -> bool {
    ctx.registry().require(record_type).is_ok_and(|definition| {
        definition.capabilities.has_trait("imports.importable")
            && aphel_sdk::records::enabled(ctx, &definition.record_type)
    })
}
pub(super) fn require_importable(ctx: &Ctx<'_>, record_type: &str) -> Result<(), AppError> {
    let permission = create_permission(ctx, record_type).ok_or_else(|| {
        AppError::validation_code(
            IMPORT_NOT_SUPPORTED,
            format!("{record_type} records cannot be imported"),
        )
    })?;
    ctx.scope.require(&permission).map(|_| ())
}

pub fn is_row_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Validation(_)
            | AppError::Coded { .. }
            | AppError::Forbidden(_)
            | AppError::NotFound(_)
            | AppError::Conflict(_)
    )
}

pub use aphel_sdk::imports::CreateImportCommand;

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigureImportCommand {
    pub mapping: Vec<Option<String>>,
    pub options: ImportOptions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDetails {
    pub job: ImportJob,
    pub targets: Vec<ImportTarget>,
    pub(crate) duplicate_keys: Vec<DuplicateKey>,
}

impl ImportDetails {
    fn new(job: ImportJob, targets: Vec<ImportTarget>, fields: &FieldSet) -> Self {
        Self {
            job,
            targets,
            duplicate_keys: DuplicateKey::supported(Some(&fields.record_type)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunImportRequest {
    pub(crate) import_id: Uuid,
    pub(crate) requested_by: UserId,
    pub(crate) api_token_id: Option<Uuid>,
}

impl RunImportRequest {
    pub(crate) fn for_scope(import_id: Uuid, scope: &AccessScope) -> Self {
        Self {
            import_id,
            requested_by: scope.user_id().clone(),
            api_token_id: scope.api_token_id().and_then(|id| Uuid::parse_str(id).ok()),
        }
    }

    pub(crate) fn payload(&self) -> Value {
        json!({
            "import_job_id": self.import_id.to_string(),
            "requested_by": self.requested_by.as_str(),
            "api_token_id": self.api_token_id.map(|id| id.to_string()),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::Internal(format!("import job payload misses {key}")))
        };
        Ok(Self {
            import_id: Uuid::parse_str(text("import_job_id")?)
                .map_err(|_| AppError::Internal("invalid import id in job payload".to_owned()))?,
            requested_by: UserId::new(text("requested_by")?)?,
            api_token_id: match payload.get("api_token_id").and_then(Value::as_str) {
                Some(id) => Some(Uuid::parse_str(id).map_err(|_| {
                    AppError::Internal("invalid API token id in job payload".to_owned())
                })?),
                None => None,
            },
        })
    }

    pub(super) fn job(&self, processed: u64) -> Job {
        Job::new(IMPORT_RUN_JOB, self.payload())
            .with_unique_key(format!("import:{}:{processed}", self.import_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunProgress {
    Continue { processed: u64 },
    Finished,
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: CreateImportCommand,
) -> Result<ImportDetails, AppError> {
    let scope = ctx.scope;
    require_importable(ctx, &command.record_type)?;
    let source = parse_source(&command.content)?;
    let fields = field_set(ctx, &command.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    let previous = match command.retry_of.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(id) => {
            let previous = visible(ctx, id).await?;
            if previous.record_type != command.record_type {
                return Err(AppError::invalid_fields(
                    "invalid import",
                    vec![FieldViolation::new(
                        "retry_of",
                        "must be an import of the same record type",
                    )],
                ));
            }
            Some(previous)
        }
    };
    let suggested = suggest_mapping(&source.headers, &targets);
    let (mapping, options) = match &previous {
        Some(previous) => {
            let known: HashMap<String, String> = previous
                .headers
                .iter()
                .zip(&previous.mapping)
                .filter_map(|(header, target)| {
                    target
                        .as_ref()
                        .map(|target| (normalize_label(header), target.clone()))
                })
                .collect();
            let mut used = BTreeSet::new();
            let mapping = source
                .headers
                .iter()
                .zip(&suggested)
                .map(|(header, suggestion)| {
                    known
                        .get(&normalize_label(header))
                        .filter(|key| targets.iter().any(|target| &target.key == *key))
                        .cloned()
                        .or_else(|| suggestion.clone())
                        .filter(|key| used.insert(key.clone()))
                })
                .collect();
            (mapping, previous.options.clone())
        }
        None => (
            suggested,
            ImportOptions::default_for_definition(Some(&fields.record_type)),
        ),
    };

    let now = ctx.now();
    let file_id = Uuid::now_v7();
    let file_name = sanitize_file_name(&command.file_name);
    let file = NewFile {
        id: file_id,
        storage_key: storage_key(file_id, now),
        original_name: file_name.clone(),
        mime_type: source_mime_type(&command.content)
            .unwrap_or(CSV_MIME_TYPE)
            .to_owned(),
        size: command.content.len() as u64,
        checksum: sha256_hex(&command.content),
        uploaded_by: scope.user_id().clone(),
        created_at: now,
    };
    let row_count = source.rows.len() as u32;
    let job = ImportJob {
        id: Uuid::now_v7(),
        record_type: command.record_type.clone(),
        file_id: Some(file_id),
        privacy_redacted_at: None,
        file_name,
        status: ImportStatus::Draft,
        encoding: source.encoding,
        delimiter: source.delimiter,
        headers: source.headers,
        preview: source.rows.into_iter().take(PREVIEW_ROWS).collect(),
        row_count,
        mapping,
        options,
        totals: ImportTotals {
            total: u64::from(row_count),
            pending: u64::from(row_count),
            ..ImportTotals::default()
        },
        error_file_id: None,
        retry_of: previous.map(|previous| previous.id),
        failure_reason: None,
        created_by: scope.user_id().clone(),
        created_at: now,
        updated_at: now,
        started_at: None,
        finished_at: None,
    };

    let storage = ctx.storage()?;
    storage.put(&file.storage_key, command.content).await?;
    let saved = async {
        let mut tx = ctx.begin().await?;
        MySqlFileRepository::new(ctx.pool().clone())
            .insert_file(&mut tx, &file)
            .await?;
        ImportStore::of(ctx).insert_job(&mut tx, &job).await?;
        tx.commit().await.map_err(AppError::from)
    }
    .await;
    if let Err(error) = saved {
        discard_generated_file(storage.as_ref(), &file.storage_key).await;
        return Err(error);
    }
    Ok(ImportDetails::new(job, targets, &fields))
}

pub(crate) async fn get(ctx: &Ctx<'_>, import_id: &str) -> Result<ImportDetails, AppError> {
    let job = visible(ctx, import_id).await?;
    let fields = field_set(ctx, &job.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    Ok(ImportDetails::new(job, targets, &fields))
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    record_type: Option<String>,
    window: ListWindow,
) -> Result<Listing<ImportJob>, AppError> {
    let scope = ctx.scope;
    let filter = ImportJobFilter {
        created_by: if scope.require(&Permission::SETTINGS_MANAGE).is_ok() {
            None
        } else {
            Some(scope.user_id().clone())
        },
        record_type: record_type.filter(|value| !value.trim().is_empty()),
    };
    ImportStore::of(ctx).list_jobs(&filter, window).await
}

pub async fn configure(
    ctx: &Ctx<'_>,
    import_id: &str,
    command: ConfigureImportCommand,
) -> Result<ImportDetails, AppError> {
    let mut job = owned(ctx, import_id).await?;
    if job.status != ImportStatus::Draft {
        return Err(import_not_editable());
    }
    let fields = field_set(ctx, &job.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    let mut violations = Vec::new();
    if command.mapping.len() != job.headers.len() {
        violations.push(FieldViolation::new(
            "mapping",
            format!("must have one entry per column ({})", job.headers.len()),
        ));
    }
    let mut used = BTreeSet::new();
    let mapping: Vec<Option<String>> = command
        .mapping
        .into_iter()
        .map(|target| {
            target
                .map(|key| key.trim().to_owned())
                .filter(|key| !key.is_empty())
        })
        .collect();
    for (index, target) in mapping.iter().enumerate() {
        let Some(key) = target else { continue };
        if fields.is_hidden(key) {
            violations.push(FieldViolation::new(
                format!("mapping.{index}"),
                NOT_ACCESSIBLE_MESSAGE,
            ));
        } else if fields
            .find_active(key)
            .is_some_and(|field| !field.is_editable())
        {
            violations.push(FieldViolation::new(
                format!("mapping.{index}"),
                READ_ONLY_MESSAGE,
            ));
        } else if !targets.iter().any(|candidate| &candidate.key == key) {
            violations.push(FieldViolation::new(
                format!("mapping.{index}"),
                "is not an importable field",
            ));
        } else if !used.insert(key.clone()) {
            violations.push(FieldViolation::new(
                format!("mapping.{index}"),
                "is mapped to more than one column",
            ));
        }
    }
    if !command
        .options
        .duplicate_key
        .supported_by(&fields.record_type)
    {
        violations.push(FieldViolation::new(
            "options.duplicate_key",
            format!(
                "{} is not a duplicate key for {} records",
                command.options.duplicate_key.as_str(),
                job.record_type
            ),
        ));
    }
    if let Some(rule_id) = &command.options.distribution_rule_id
        && !run::distribution_rule_usable(ctx, rule_id, &job.record_type).await?
    {
        violations.push(FieldViolation::new(
            "options.distribution_rule_id",
            "must be an active lead distribution rule for this record type",
        ));
    }
    if !violations.is_empty() {
        return Err(query_error("invalid import mapping", violations));
    }
    job.mapping = mapping;
    job.options = command.options;
    job.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    ImportStore::of(ctx).save_job(&mut tx, &job).await?;
    tx.commit().await?;
    Ok(ImportDetails::new(job, targets, &fields))
}

pub(crate) async fn validate(ctx: &Ctx<'_>, import_id: &str) -> Result<ValidationReport, AppError> {
    let job = owned(ctx, import_id).await?;
    require_importable(ctx, &job.record_type)?;
    let source = parse_source(&file_content(ctx, &job).await?)?;
    let fields = field_set(ctx, &job.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    let evaluator = RowEvaluator {
        fields: &fields,
        targets: &targets,
        headers: &job.headers,
        mapping: &job.mapping,
        timezone: timezone(ctx).await?,
    };
    let mut cache = ResolutionCache::default();
    let mut report = ValidationReport {
        total_rows: source.rows.len() as u64,
        valid_rows: 0,
        invalid_rows: 0,
        rows: Vec::new(),
    };
    for row in &source.rows {
        let mut planned = evaluator.plan(&row.cells);
        if planned.errors.is_empty() {
            run::resolve(ctx, &job, &mut planned, &mut cache, ResolveMode::Validate).await?;
        }
        if planned.errors.is_empty() {
            report.valid_rows += 1;
        } else {
            report.invalid_rows += 1;
            if report.rows.len() < MAX_REPORTED_ROWS {
                report.rows.push(RowReport {
                    row_index: row.row_index,
                    errors: planned.errors,
                });
            }
        }
    }
    Ok(report)
}

pub(crate) async fn start(
    ctx: &Ctx<'_>,
    import_id: &str,
    ip: Option<String>,
) -> Result<ImportDetails, AppError> {
    let scope = ctx.scope;
    let mut job = owned(ctx, import_id).await?;
    require_importable(ctx, &job.record_type)?;
    if job.status != ImportStatus::Draft {
        return Err(import_not_editable());
    }
    if job.options.duplicate_strategy == DuplicateStrategy::Update
        && job.options.duplicate_key != DuplicateKey::None
        && let Some(permission) = update_permission(ctx, &job.record_type)
    {
        scope.require(&permission)?;
    }
    let fields = field_set(ctx, &job.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    check_mapping_complete(&job, &targets)?;
    let source = parse_source(&file_content(ctx, &job).await?)?;

    let now = ctx.now();
    let request = RunImportRequest::for_scope(job.id, scope);
    job.status = ImportStatus::Queued;
    job.row_count = source.rows.len() as u32;
    job.totals = ImportTotals {
        total: u64::from(job.row_count),
        pending: u64::from(job.row_count),
        ..ImportTotals::default()
    };
    job.updated_at = now;
    let record = RecordRef::new(IMPORT_RECORD_TYPE, job.id);
    let store = ImportStore::of(ctx);
    let mut tx = ctx.begin().await?;
    store.insert_rows(&mut tx, job.id, &source.rows).await?;
    store.save_job(&mut tx, &job).await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::from_scope(scope), IMPORT_STARTED, now)
                .with_record(record)
                .with_changes(vec![
                    FieldChange::new("record_type", Value::Null, json!(job.record_type)),
                    FieldChange::new("file_name", Value::Null, json!(job.file_name)),
                    FieldChange::new("rows", Value::Null, json!(job.row_count)),
                    FieldChange::new("mapping", Value::Null, json!(job.mapping)),
                    FieldChange::new("options", Value::Null, json!(job.options)),
                ])
                .with_ip(ip),
        )
        .await?;
    ctx.jobs().enqueue(&mut tx, request.job(0)).await?;
    tx.commit().await?;
    Ok(ImportDetails::new(job, targets, &fields))
}

pub(crate) async fn rows(
    ctx: &Ctx<'_>,
    import_id: &str,
    status: Option<RowStatus>,
    window: ListWindow,
) -> Result<Listing<ImportRow>, AppError> {
    let job = visible(ctx, import_id).await?;
    ImportStore::of(ctx).list_rows(job.id, status, window).await
}

pub(crate) async fn retry_failed(
    ctx: &Ctx<'_>,
    import_id: &str,
    ip: Option<String>,
) -> Result<ImportDetails, AppError> {
    let scope = ctx.scope;
    let mut job = owned(ctx, import_id).await?;
    require_importable(ctx, &job.record_type)?;
    if job.privacy_redacted_at.is_some() || !job.status.is_finished() || job.totals.failed == 0 {
        return Err(AppError::conflict_code(
            IMPORT_NOT_RETRYABLE,
            "only finished imports with failed rows can be retried",
        ));
    }
    let now = ctx.now();
    let store = ImportStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let reset = store.reset_failed_rows(&mut tx, job.id).await?;
    job.totals.failed = job.totals.failed.saturating_sub(reset);
    job.totals.pending += reset;
    job.status = ImportStatus::Queued;
    job.error_file_id = None;
    job.failure_reason = None;
    job.finished_at = None;
    job.updated_at = now;
    store.save_job(&mut tx, &job).await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::from_scope(scope), IMPORT_RETRIED, now)
                .with_record(RecordRef::new(IMPORT_RECORD_TYPE, job.id))
                .with_changes(vec![FieldChange::new("rows", Value::Null, json!(reset))])
                .with_ip(ip),
        )
        .await?;
    let request = RunImportRequest::for_scope(job.id, scope);
    ctx.jobs()
        .enqueue(&mut tx, request.job(job.totals.processed()))
        .await?;
    tx.commit().await?;
    let fields = field_set(ctx, &job.record_type).await?;
    let targets = import_targets(&fields, ctx.registry())?;
    Ok(ImportDetails::new(job, targets, &fields))
}

fn check_mapping_complete(job: &ImportJob, targets: &[ImportTarget]) -> Result<(), AppError> {
    let mapped: BTreeSet<&str> = job.mapping.iter().flatten().map(String::as_str).collect();
    let mut violations = Vec::new();
    if mapped.is_empty() {
        violations.push(FieldViolation::new("mapping", "map at least one column"));
    }
    for target in targets.iter().filter(|target| target.required) {
        let satisfied = mapped.contains(target.key.as_str())
            || target
                .satisfied_by
                .as_deref()
                .is_some_and(|key| mapped.contains(key));
        if !satisfied {
            violations.push(FieldViolation::new(
                "mapping",
                format!("the required field {} is not mapped", target.label),
            ));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(AppError::invalid_fields(
            "incomplete import mapping",
            violations,
        ))
    }
}

async fn file_content(ctx: &Ctx<'_>, job: &ImportJob) -> Result<Bytes, AppError> {
    let file = MySqlFileRepository::new(ctx.pool().clone())
        .find_file(job.file_id.ok_or_else(|| {
            AppError::conflict_code(
                IMPORT_NOT_RETRYABLE,
                "the original import file was removed by a privacy request",
            )
        })?)
        .await?
        .ok_or_else(|| AppError::not_found_code(IMPORT_NOT_FOUND, "the import file was removed"))?;
    read_file_content(ctx.storage()?.as_ref(), &file.storage_key).await
}

pub(super) async fn field_set(ctx: &Ctx<'_>, record_type: &str) -> Result<FieldSet, AppError> {
    let query = ctx.query();
    let definition = query.registry().require(record_type)?;
    FieldSet::for_scope(query.definitions(), definition, ctx.scope).await
}

pub(super) async fn timezone(ctx: &Ctx<'_>) -> Result<Tz, AppError> {
    ctx.query().executor().timezone().await
}

async fn visible(ctx: &Ctx<'_>, import_id: &str) -> Result<ImportJob, AppError> {
    let scope = ctx.scope;
    let id = Uuid::parse_str(import_id.trim()).map_err(|_| import_not_found())?;
    let job = ImportStore::of(ctx)
        .find_job(id)
        .await?
        .ok_or_else(import_not_found)?;
    if &job.created_by == scope.user_id() || scope.require(&Permission::SETTINGS_MANAGE).is_ok() {
        return Ok(job);
    }
    Err(import_not_found())
}

async fn owned(ctx: &Ctx<'_>, import_id: &str) -> Result<ImportJob, AppError> {
    let job = visible(ctx, import_id).await?;
    if &job.created_by != ctx.scope.user_id() {
        return Err(AppError::forbidden_code(
            aphel_sdk::platform::application::access::PERMISSION_DENIED,
            "only the user who created the import can change it",
        ));
    }
    Ok(job)
}
