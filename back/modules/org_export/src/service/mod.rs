pub(crate) mod config;
pub(crate) mod restore;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::path::Path;

use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::store::{OrgDataStore, OrgExportStore, RowSource};
use crate::{
    ArchiveWriter, CONFIG_KEYS_FILE, CONFIG_PACKAGE_FILE, ConfigKind, DATASETS,
    DOMAIN_EVENTS_DATASET, DataRow, ExportSummary, FILE_BLOBS_DIR, FILES_FILE, MEMBERS_FILE,
    Manifest, ManifestOrganization, ORG_EXPORT_COMPLETED, ORG_EXPORT_FAILED, ORG_EXPORT_FORMAT,
    ORG_EXPORT_FORMAT_VERSION, ORG_EXPORT_IN_PROGRESS, ORG_EXPORT_NOT_READY,
    ORG_EXPORT_RECORD_TYPE, ORG_EXPORT_REQUESTED, ORG_EXPORT_RUN_JOB, OrgExport, OrgExportStatus,
    TempFile, ZIP_MIME_TYPE, file_stream, org_export_not_found, record_path, record_priority,
    sha256_file,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, Authorizer, build_access_scope};
use aphel_kernel::application::fields::{CustomFieldReader, EntityDefinition, RecordTypeRegistry};
use aphel_kernel::application::files::{FileRepository, NewFile, sanitize_file_name, storage_key};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, ByteStream, DomainEvent, EventPublisher, FieldChange, FileStorage,
    Job, JobQueue, RecordRef, RecordType,
};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::infrastructure::storage::MySqlFileRepository;
use aphel_sdk::configuration::APHEL_VERSION;

pub(crate) use config::{export_config_package, install_config_package};
pub(crate) use restore::{IdStrategy, RestoreCommand, RestoreEnv, inspect, restore_installation};

pub(crate) const EXPORT_PAGE_SIZE: u64 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestOrgExportCommand {
    pub(crate) include_files: bool,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunOrgExportRequest {
    pub(crate) export_id: Uuid,
    pub(crate) requested_by: UserId,
}

impl RunOrgExportRequest {
    pub(crate) fn payload(&self) -> Value {
        json!({
            "export_id": self.export_id.to_string(),
            "requested_by": self.requested_by.as_str(),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| {
            payload.get(key).and_then(Value::as_str).ok_or_else(|| {
                AppError::Internal(format!("organization export job payload misses {key}"))
            })
        };
        Ok(Self {
            export_id: Uuid::parse_str(text("export_id")?).map_err(|_| {
                AppError::Internal("invalid organization export id in job payload".to_owned())
            })?,
            requested_by: UserId::new(text("requested_by")?)?,
        })
    }
}

pub(crate) struct OrgExportDownload {
    pub(crate) file_name: String,
    pub(crate) size: u64,
    pub(crate) content: ByteStream,
}

pub(crate) async fn request(
    ctx: &Ctx<'_>,
    command: RequestOrgExportCommand,
) -> Result<OrgExport, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::ORGANIZATION_EXPORT)?;
    let exports = OrgExportStore::of(ctx);
    let now = ctx.now();
    let export = OrgExport {
        id: Uuid::now_v7(),
        status: OrgExportStatus::Queued,
        include_files: command.include_files,
        file_id: None,
        summary: None,
        failure_reason: None,
        requested_by: scope.user_id().clone(),
        requested_by_name: None,
        created_at: now,
        updated_at: now,
        started_at: None,
        finished_at: None,
    };
    let request = RunOrgExportRequest {
        export_id: export.id,
        requested_by: export.requested_by.clone(),
    };
    let record = RecordRef::new(ORG_EXPORT_RECORD_TYPE, export.id);
    let mut tx = ctx.begin().await?;
    if exports.has_pending(&mut tx).await? {
        return Err(AppError::conflict_code(
            ORG_EXPORT_IN_PROGRESS,
            "an organization export is already queued or running",
        ));
    }
    exports.insert(&mut tx, &export).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(Actor::from_scope(scope), ORG_EXPORT_REQUESTED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "export_id": export.id.to_string(),
                    "requested_by": export.requested_by.as_str(),
                    "include_files": export.include_files,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::from_scope(scope), ORG_EXPORT_REQUESTED, now)
                .with_record(record)
                .with_changes(vec![FieldChange::new(
                    "include_files",
                    Value::Null,
                    json!(export.include_files),
                )])
                .with_ip(command.ip),
        )
        .await?;
    ctx.jobs()
        .enqueue(
            &mut tx,
            Job::new(ORG_EXPORT_RUN_JOB, request.payload())
                .with_max_attempts(2)
                .with_unique_key(format!("organization_export:{}", export.id)),
        )
        .await?;
    tx.commit().await?;
    Ok(export)
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    window: ListWindow,
) -> Result<Listing<OrgExport>, AppError> {
    ctx.scope.require(&Permission::ORGANIZATION_EXPORT)?;
    OrgExportStore::of(ctx).list(window).await
}

pub(crate) async fn get(ctx: &Ctx<'_>, export_id: &str) -> Result<OrgExport, AppError> {
    ctx.scope.require(&Permission::ORGANIZATION_EXPORT)?;
    let id = Uuid::parse_str(export_id.trim()).map_err(|_| org_export_not_found())?;
    OrgExportStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(org_export_not_found)
}

pub(crate) async fn download(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    export_id: &str,
) -> Result<OrgExportDownload, AppError> {
    let export = get(ctx, export_id).await?;
    let file_id = match (export.status, export.file_id) {
        (OrgExportStatus::Completed, Some(file_id)) => file_id,
        _ => {
            return Err(AppError::conflict_code(
                ORG_EXPORT_NOT_READY,
                "the organization export has no file to download",
            ));
        }
    };
    let file = MySqlFileRepository::new(ctx.pool().clone())
        .find_file(file_id)
        .await?
        .ok_or_else(org_export_not_found)?;
    let content = storage.get_stream(&file.storage_key).await?;
    Ok(OrgExportDownload {
        file_name: file.original_name,
        size: file.size,
        content,
    })
}

pub(crate) async fn run(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    request: &RunOrgExportRequest,
) -> Result<(), AppError> {
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
    let exports = OrgExportStore::of(&ctx);
    let Some(mut export) = exports.find(request.export_id).await? else {
        return Ok(());
    };
    if export.status.is_finished() {
        return Ok(());
    }
    if scope.require(&Permission::ORGANIZATION_EXPORT).is_err() {
        return fail(
            &ctx,
            request,
            "the requester can no longer export the organization",
        )
        .await;
    }
    let started = ctx.now();
    export.status = OrgExportStatus::Running;
    export.started_at = Some(started);
    export.updated_at = started;
    let mut tx = ctx.begin().await?;
    exports.save(&mut tx, &export).await?;
    tx.commit().await?;

    let temp = TempFile::new("aphel-org-export", "zip");
    let summary =
        build_archive(&ctx, storage, export.id, export.include_files, temp.path()).await?;
    let now = ctx.now();
    let name = format!("aphel-backup-{}.zip", now.format("%Y%m%d-%H%M%S"));
    let stored = upload_archive(storage, &scope, &temp, &name, now).await?;
    drop(temp);
    let record = RecordRef::new(ORG_EXPORT_RECORD_TYPE, export.id);
    let mut tx = ctx.begin().await?;
    if let Err(error) = MySqlFileRepository::new(ctx.pool().clone())
        .insert_file(&mut tx, &stored)
        .await
    {
        discard_file(storage, &stored.storage_key).await;
        return Err(error);
    }
    let file_id = stored.id;
    let storage_key = stored.storage_key;
    export.status = OrgExportStatus::Completed;
    export.file_id = Some(file_id);
    export.summary = Some(summary.clone());
    export.finished_at = Some(now);
    export.updated_at = now;
    let result = async {
        exports.save(&mut tx, &export).await?;
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(Actor::System, ORG_EXPORT_COMPLETED, now)
                    .with_record(record.clone())
                    .with_payload(json!({
                        "export_id": export.id.to_string(),
                        "requested_by": export.requested_by.as_str(),
                        "file_id": file_id.to_string(),
                        "summary": summary,
                    })),
            )
            .await?;
        ctx.audit()
            .record(
                &mut tx,
                AuditEntry::new(Actor::System, ORG_EXPORT_COMPLETED, now)
                    .with_record(record)
                    .with_changes(vec![
                        FieldChange::new("file_id", Value::Null, json!(file_id.to_string())),
                        FieldChange::new("summary", Value::Null, json!(summary)),
                    ]),
            )
            .await?;
        tx.commit().await.map_err(AppError::from)
    }
    .await;
    if let Err(error) = result {
        discard_file(storage, &storage_key).await;
        return Err(error);
    }
    Ok(())
}

async fn upload_archive(
    storage: &dyn FileStorage,
    scope: &AccessScope,
    archive: &TempFile,
    name: &str,
    now: DateTime<Utc>,
) -> Result<NewFile, AppError> {
    let (size, checksum) = sha256_file(archive.path())?;
    let id = Uuid::now_v7();
    let file = NewFile {
        id,
        storage_key: storage_key(id, now),
        original_name: sanitize_file_name(name),
        mime_type: ZIP_MIME_TYPE.to_owned(),
        size,
        checksum,
        uploaded_by: scope.user_id().clone(),
        created_at: now,
    };
    if let Err(error) = storage
        .put_stream(&file.storage_key, file_stream(archive.path())?)
        .await
    {
        discard_file(storage, &file.storage_key).await;
        return Err(error);
    }
    Ok(file)
}

async fn discard_file(storage: &dyn FileStorage, storage_key: &str) {
    if let Err(error) = storage.delete(storage_key).await {
        tracing::warn!(error = %error, "failed to remove an orphan generated file");
    }
}

pub(crate) async fn fail(
    ctx: &Ctx<'_>,
    request: &RunOrgExportRequest,
    reason: &str,
) -> Result<(), AppError> {
    let exports = OrgExportStore::of(ctx);
    let Some(mut export) = exports.find(request.export_id).await? else {
        return Ok(());
    };
    if export.status.is_finished() {
        return Ok(());
    }
    let now = ctx.now();
    export.status = OrgExportStatus::Failed;
    export.failure_reason = Some(reason.chars().take(500).collect());
    export.finished_at = Some(now);
    export.updated_at = now;
    let record = RecordRef::new(ORG_EXPORT_RECORD_TYPE, export.id);
    let mut tx = ctx.begin().await?;
    exports.save(&mut tx, &export).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(Actor::System, ORG_EXPORT_FAILED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "export_id": export.id.to_string(),
                    "requested_by": export.requested_by.as_str(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(Actor::System, ORG_EXPORT_FAILED, now)
                .with_record(record)
                .with_changes(vec![FieldChange::new(
                    "failure_reason",
                    Value::Null,
                    json!(export.failure_reason),
                )]),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn all_rows(data: &OrgDataStore, source: RowSource<'_>) -> Result<Vec<DataRow>, AppError> {
    let mut rows = Vec::new();
    let mut offset = 0;
    loop {
        let page = data
            .rows(
                source,
                ListWindow {
                    offset,
                    limit: EXPORT_PAGE_SIZE,
                },
            )
            .await?;
        let size = page.len() as u64;
        rows.extend(page);
        if size < EXPORT_PAGE_SIZE {
            return Ok(rows);
        }
        offset += size;
    }
}

async fn write_rows(
    archive: &mut ArchiveWriter,
    data: &OrgDataStore,
    source: RowSource<'_>,
) -> Result<(), AppError> {
    let mut offset = 0;
    loop {
        let page = data
            .rows(
                source,
                ListWindow {
                    offset,
                    limit: EXPORT_PAGE_SIZE,
                },
            )
            .await?;
        let size = page.len() as u64;
        for row in page {
            archive.push_line(&Value::Object(row))?;
        }
        if size < EXPORT_PAGE_SIZE {
            return Ok(());
        }
        offset += size;
    }
}

fn row_id(row: &DataRow) -> Option<Uuid> {
    row.get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn sorted_definitions(registry: &RecordTypeRegistry) -> Vec<&EntityDefinition> {
    let mut definitions: Vec<&EntityDefinition> = registry.definitions().collect();
    definitions.sort_by_key(|definition| {
        (
            record_priority(definition),
            definition.record_type.as_str().to_owned(),
        )
    });
    definitions
}

enum BlobOutcome {
    Written(u64),
    Mismatch,
    Unreadable(AppError),
}

async fn write_blob(
    archive: &mut ArchiveWriter,
    storage: &dyn FileStorage,
    entry: &str,
    storage_key: &str,
    checksum: Option<&str>,
) -> Result<BlobOutcome, AppError> {
    let mut content = match storage.get_stream(storage_key).await {
        Ok(content) => content,
        Err(error) => return Ok(BlobOutcome::Unreadable(error)),
    };
    archive.start(entry, "blob", None)?;
    while let Some(chunk) = content.next().await {
        match chunk {
            Ok(chunk) => archive.write(&chunk)?,
            Err(error) => {
                archive.abort()?;
                return Ok(BlobOutcome::Unreadable(error));
            }
        }
    }
    let matches = archive
        .current_sha256()
        .is_some_and(|sum| checksum.is_none_or(|expected| expected == sum));
    if !matches {
        archive.abort()?;
        return Ok(BlobOutcome::Mismatch);
    }
    Ok(BlobOutcome::Written(archive.end_blob()?))
}

pub(crate) async fn build_archive(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    export_id: Uuid,
    include_files: bool,
    destination: &Path,
) -> Result<ExportSummary, AppError> {
    let data = &OrgDataStore::of(ctx);
    let mut archive = ArchiveWriter::create(destination)?;
    let mut summary = ExportSummary::default();

    let package = export_config_package(ctx).await?;
    archive.add(
        CONFIG_PACKAGE_FILE,
        "config_package",
        None,
        1,
        &serde_json::to_vec_pretty(&package)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    )?;

    let keys = data.config_keys().await?;
    let keys_json: Map<String, Value> = keys
        .iter()
        .map(|(kind, items)| {
            (
                kind.as_str().to_owned(),
                Value::Object(
                    items
                        .iter()
                        .map(|(id, key)| (id.to_string(), Value::String(key.clone())))
                        .collect(),
                ),
            )
        })
        .collect();
    archive.add(
        CONFIG_KEYS_FILE,
        "config_keys",
        None,
        keys.values().map(|items| items.len() as u64).sum(),
        &serde_json::to_vec_pretty(&Value::Object(keys_json))
            .map_err(|error| AppError::Internal(error.to_string()))?,
    )?;
    for kind in ConfigKind::FALLBACK {
        archive.start(&kind.items_path(), "config_items", Some(kind.as_str()))?;
        write_rows(
            &mut archive,
            data,
            RowSource::Table {
                table: kind.table(),
                partition: None,
            },
        )
        .await?;
        archive.end()?;
    }

    let members = data.members().await?;
    archive.start(MEMBERS_FILE, "members", None)?;
    for member in &members {
        archive.push_line(
            &serde_json::to_value(member).map_err(|error| AppError::Internal(error.to_string()))?,
        )?;
    }
    summary.members = archive.end()?;

    let registry = ctx.registry();
    let fields = ctx.fields();
    let reader = CustomFieldReader::new(&fields);
    let mut record_types = Vec::new();
    for definition in sorted_definitions(registry) {
        let record_type = definition.record_type.clone();
        record_types.push(record_type.as_str().to_owned());
        let partition = definition
            .partition
            .as_ref()
            .map(|partition| (partition.column.as_str(), partition.value));
        let source = RowSource::Table {
            table: &definition.table,
            partition,
        };
        archive.start(
            &record_path(record_type.as_str()),
            "records",
            Some(record_type.as_str()),
        )?;
        let mut offset = 0;
        loop {
            let page = data
                .rows(
                    source,
                    ListWindow {
                        offset,
                        limit: EXPORT_PAGE_SIZE,
                    },
                )
                .await?;
            let size = page.len() as u64;
            let ids: Vec<Uuid> = page.iter().filter_map(row_id).collect();
            let mut custom = reader.load_unrestricted(&record_type, &ids).await?;
            for row in page {
                let custom_fields = row_id(&row)
                    .and_then(|id| custom.remove(&id))
                    .map(|values| {
                        values
                            .into_iter()
                            .filter(|(_, value)| !value.is_null())
                            .collect::<Map<String, Value>>()
                    })
                    .unwrap_or_default();
                archive.push_line(&json!({
                    "id": row.get("id").cloned().unwrap_or(Value::Null),
                    "row": row,
                    "custom_fields": custom_fields,
                }))?;
            }
            if size < EXPORT_PAGE_SIZE {
                break;
            }
            offset += size;
        }
        let count = archive.end()?;
        summary
            .records
            .insert(record_type.as_str().to_owned(), count);
    }

    let mut event_types = record_types.clone();
    event_types.push(RecordType::NOTE.as_str().to_owned());
    for dataset in DATASETS {
        if !data.table_exists(dataset.table).await? {
            continue;
        }
        let source = if dataset.key == DOMAIN_EVENTS_DATASET {
            RowSource::RecordEvents {
                record_types: &event_types,
            }
        } else {
            RowSource::Table {
                table: dataset.table,
                partition: None,
            }
        };
        archive.start(&dataset.path(), "dataset", Some(dataset.key))?;
        write_rows(&mut archive, data, source).await?;
        let count = archive.end()?;
        summary.datasets.insert(dataset.key.to_owned(), count);
    }

    let mut file_rows = Vec::new();
    for mut row in all_rows(data, RowSource::AttachmentFiles).await? {
        let storage_key = row
            .remove("storage_key")
            .and_then(|value| value.as_str().map(str::to_owned));
        let id = row_id(&row);
        let mut path = Value::Null;
        if include_files && let (Some(id), Some(storage_key)) = (id, storage_key) {
            let entry = format!("{FILE_BLOBS_DIR}/{id}");
            let checksum = row.get("checksum").and_then(Value::as_str);
            match write_blob(&mut archive, storage, &entry, &storage_key, checksum).await? {
                BlobOutcome::Written(bytes) => {
                    summary.file_bytes += bytes;
                    path = Value::String(entry);
                }
                BlobOutcome::Mismatch => {
                    tracing::warn!(file_id = %id, "a file does not match its checksum and was left out of the organization export");
                    summary.missing_files += 1;
                }
                BlobOutcome::Unreadable(error) => {
                    tracing::warn!(file_id = %id, error = %error, "a file could not be read for the organization export");
                    summary.missing_files += 1;
                }
            }
        }
        row.insert("path".to_owned(), path);
        file_rows.push(row);
    }
    archive.start(FILES_FILE, "files", None)?;
    for row in file_rows {
        archive.push_line(&Value::Object(row))?;
    }
    summary.files = archive.end()?;

    let profile = data.profile().await?;
    let manifest = Manifest {
        format: ORG_EXPORT_FORMAT.to_owned(),
        format_version: ORG_EXPORT_FORMAT_VERSION,
        aphel_version: APHEL_VERSION.to_owned(),
        exported_at: ctx.now().to_rfc3339(),
        export_id: export_id.to_string(),
        organization: ManifestOrganization {
            name: profile.name,
            locale: profile.locale,
            timezone: profile.timezone,
            currency: profile.currency,
        },
        include_files,
        counts: summary.clone(),
        entries: archive.entries().to_vec(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    archive.finish(&manifest_bytes)?;
    summary.archive_bytes = std::fs::metadata(destination)
        .map_err(|error| AppError::Internal(format!("cannot inspect the archive: {error}")))?
        .len();
    Ok(summary)
}
