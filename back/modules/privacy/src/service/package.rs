use std::collections::HashMap;
use std::io::{Cursor, Write};

use bytes::Bytes;
use serde_json::{Value, json};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::domain::{MAX_EXPORT_ATTACHMENT_BYTES, PrivacyRequest, RunReport};
use crate::service::requests::ExecutionOutcome;
use crate::service::{definition, run_external_sources};
use crate::store::PrivacyStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::CustomFieldReader;
use aphel_kernel::application::files::sanitize_file_name;
use aphel_kernel::application::shared::{FileStorage, RecordRef};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use exports::{GeneratedFile, read_file_content};

pub(crate) const EXPORT_FORMAT: &str = "aphel.privacy_export.v1";
pub(crate) const EXPORT_DATA_FILE: &str = "dados.json";
pub(crate) const EXPORT_ATTACHMENTS_DIR: &str = "anexos";
pub(crate) const ZIP_MIME_TYPE: &str = "application/zip";

fn zip_error(error: impl std::fmt::Display) -> AppError {
    AppError::Internal(format!("cannot build the privacy export archive: {error}"))
}

struct Archive {
    writer: ZipWriter<Cursor<Vec<u8>>>,
}

impl Archive {
    fn new() -> Self {
        Self {
            writer: ZipWriter::new(Cursor::new(Vec::new())),
        }
    }

    fn add(&mut self, path: &str, content: &[u8]) -> Result<(), AppError> {
        self.writer
            .start_file(
                path,
                SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(0o600),
            )
            .map_err(zip_error)?;
        self.writer.write_all(content).map_err(zip_error)
    }

    fn finish(self) -> Result<Bytes, AppError> {
        Ok(Bytes::from(
            self.writer.finish().map_err(zip_error)?.into_inner(),
        ))
    }
}

pub(crate) async fn build_export(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    request: &PrivacyRequest,
) -> Result<ExecutionOutcome, AppError> {
    let scope = AccessScope::system();
    let mut report = RunReport::default();
    let mut archive = Archive::new();
    let records = &request.targets.records;
    let privacy = PrivacyStore::of(ctx);

    let mut source_data = serde_json::Map::new();
    for source in super::sources::available(ctx).await? {
        let ids = super::sources::item_ids(&request.targets, source.key());
        let snapshot = source.snapshot(ctx, records, &ids).await?;
        report
            .source_items
            .insert(source.key().to_owned(), snapshot.data.len() as u64);
        source_data.insert(source.key().to_owned(), json!(snapshot.data));
    }
    let mut attachments_by_record: HashMap<RecordRef, Vec<Value>> = HashMap::new();
    let mut archived_bytes = 0u64;
    for attachment in privacy.attachments(records).await? {
        report.attachments += 1;
        let mut path = None;
        let mut omitted = false;
        if archived_bytes + attachment.size <= MAX_EXPORT_ATTACHMENT_BYTES {
            match read_file_content(storage, &attachment.storage_key).await {
                Ok(content) => {
                    let entry = format!(
                        "{EXPORT_ATTACHMENTS_DIR}/{}-{}",
                        attachment.id,
                        sanitize_file_name(&attachment.file_name)
                    );
                    archive.add(&entry, &content)?;
                    archived_bytes += content.len() as u64;
                    report.files += 1;
                    path = Some(entry);
                }
                Err(error) => {
                    tracing::warn!(error = %error, "an attachment could not be read for a privacy export");
                }
            }
        } else {
            omitted = true;
        }
        attachments_by_record
            .entry(attachment.record.clone())
            .or_default()
            .push(json!({
                "id": attachment.id.to_string(),
                "file_name": attachment.file_name,
                "mime_type": attachment.mime_type,
                "size": attachment.size,
                "created_at": attachment.created_at.to_rfc3339(),
                "path": path,
                "omitted_by_size_limit": omitted,
            }));
    }

    let fields_ports = ctx.fields();
    let reader = CustomFieldReader::new(&fields_ports);
    let mut exported = Vec::with_capacity(records.len());
    for record in records {
        let definition = definition(ctx, record.record_type.as_str())?;
        let Some(fields) = privacy
            .record_snapshot(&definition, record.record_id)
            .await?
        else {
            report.skipped += 1;
            continue;
        };
        let custom = reader
            .load(&scope, &record.record_type, &[record.record_id])
            .await?
            .remove(&record.record_id)
            .unwrap_or_default();
        report.records += 1;
        exported.push(json!({
            "record_type": record.record_type.as_str(),
            "id": record.record_id.to_string(),
            "fields": fields,
            "custom_fields": custom,
            "attachments": attachments_by_record.remove(record).unwrap_or_default(),
        }));
    }

    let external: serde_json::Map<String, Value> = run_external_sources(ctx, request, &mut report)
        .await
        .into_iter()
        .collect();

    let now = ctx.now();
    let document = json!({
        "format": EXPORT_FORMAT,
        "request_id": request.id.to_string(),
        "generated_at": now.to_rfc3339(),
        "subject": {
            "kind": request.subject_kind.as_str(),
            "label": request.subject_label,
        },
        "records": exported,
        "sources": source_data,
        "external": external,
    });
    let data = serde_json::to_vec_pretty(&document)
        .map_err(|error| AppError::Internal(format!("cannot encode the export: {error}")))?;
    archive.add(EXPORT_DATA_FILE, &data)?;
    let content = archive.finish()?;
    Ok(ExecutionOutcome {
        report,
        file: Some(GeneratedFile {
            name: format!("dados-titular-{}.zip", now.format("%Y%m%d-%H%M%S")),
            mime_type: ZIP_MIME_TYPE.to_owned(),
            content,
        }),
    })
}
