use aphel_contract_crm::CrmRecordTypes;
use serde_json::{Value, json};
use std::borrow::Cow;
use uuid::Uuid;

use crate::domain::{
    ANONYMIZED_CONTACT_NAME, ANONYMIZED_TEXT, FieldSource, PRIVACY_REQUEST_RECORD_TYPE,
    PersonalField, PrivacyRequest, PrivacyRequestKind, RunReport,
};
use crate::service::requests::ExecutionOutcome;
use crate::service::subject::Redaction;
use crate::service::{
    Mutation, definition, personal_fields, publish, remove_stored_files, run_external_sources,
};
use crate::store::{ColumnAssignment, PrivacyStore, free_text_columns};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldValueRepository, CustomFieldWriter,
    EntityDefinition, custom_field_path,
};
use aphel_kernel::application::shared::{Actor, FieldChange, FileStorage, RecordRef, RecordType};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use trash::{TrashStore, is_trashable, purged_event};

pub(crate) const PRIVACY_REQUEST_ITEMS_ERASED: &str = "privacy_request.items_erased";

const PROTECTED_KEYS: [&str; 11] = [
    "created_at",
    "updated_at",
    "currency",
    "status",
    "priority",
    "pipeline_id",
    "stage_id",
    "activity_type_id",
    "stage_entered_at",
    "owner_id",
    "amount",
];
const GENERATED_COLUMNS: [&str; 1] = ["full_name"];
const NAME_KEYS: [&str; 2] = ["name", "full_name"];

pub(crate) fn anonymized_event(record_type: &str) -> String {
    format!("{record_type}.anonymized")
}

pub(crate) fn erased_event(record_type: &str) -> String {
    purged_event(record_type)
}

fn neutral_text(record_type: &str) -> &'static str {
    if record_type == RecordType::CONTACT.as_str() {
        ANONYMIZED_CONTACT_NAME
    } else {
        ANONYMIZED_TEXT
    }
}

pub(crate) fn anonymize_assignments(
    definition: &EntityDefinition,
    fields: &[PersonalField],
) -> Vec<(String, ColumnAssignment)> {
    let mut assignments: Vec<(String, ColumnAssignment)> = Vec::new();
    for field in fields {
        let FieldSource::Column(column) = &field.source else {
            continue;
        };
        if PROTECTED_KEYS.contains(&field.key.as_str())
            || GENERATED_COLUMNS.contains(&column.as_str())
        {
            continue;
        }
        let value = if field.required {
            if !field.field_type.is_textual() {
                continue;
            }
            Some(neutral_text(definition.record_type.as_str()).to_owned())
        } else {
            None
        };
        if assignments
            .iter()
            .any(|(_, existing)| existing.column == *column)
        {
            continue;
        }
        assignments.push((
            field.key.clone(),
            ColumnAssignment {
                column: column.clone(),
                value,
            },
        ));
    }
    for column in free_text_columns(definition) {
        if !assignments
            .iter()
            .any(|(_, existing)| existing.column == *column)
        {
            assignments.push((
                (*column).to_owned(),
                ColumnAssignment {
                    column: (*column).to_owned(),
                    value: None,
                },
            ));
        }
    }
    assignments
}

#[derive(Default)]
struct ErasedRecord {
    notes: u64,
    attachments: u64,
    history: u64,
    storage_keys: Vec<String>,
}

pub(crate) async fn erase(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    request: &PrivacyRequest,
) -> Result<ExecutionOutcome, AppError> {
    let mut report = RunReport::default();
    run_external_sources(ctx, request, &mut report).await;
    for record in &request.targets.records {
        match erase_record(ctx, request, record).await {
            Ok(Some(erased)) => {
                report.records += 1;
                if record.record_type == RecordType::ACTIVITY {
                    report.activities += 1;
                }
                report.notes += erased.notes;
                report.attachments += erased.attachments;
                report.history_entries += erased.history;
                report.files += remove_stored_files(storage, &erased.storage_keys).await;
            }
            Ok(None) => report.skipped += 1,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    record_type = %record.record_type,
                    "a record of a privacy request could not be processed"
                );
                report.errors += 1;
            }
        }
    }

    let notes = &request.targets.notes;
    let rows = &request.targets.import_rows;
    if !notes.is_empty() || !rows.is_empty() {
        let mut redaction = Redaction::new();
        let privacy = PrivacyStore::of(ctx);
        for row in privacy.import_rows(rows).await? {
            redaction.add_json(&row.raw);
            redaction.add_identifiers_json(&row.raw);
        }
        let now = ctx.now();
        let mut tx = ctx.begin().await?;
        if !privacy
            .claim_item(
                &mut tx,
                request.id,
                &RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
            )
            .await?
        {
            return Ok(ExecutionOutcome { report, file: None });
        }
        let removed_notes = privacy.delete_notes(&mut tx, notes).await?;
        let note_refs: Vec<RecordRef> = notes
            .iter()
            .map(|id| RecordRef::new(RecordType::NOTE, *id))
            .collect();
        let history = if redaction.is_empty() {
            0
        } else {
            privacy
                .redact_history(&mut tx, &note_refs, &redaction)
                .await?
        };
        let redacted_rows = privacy
            .redact_import_rows(&mut tx, rows, &redaction)
            .await?;
        let mentions = privacy.redact_mentions(&mut tx, &redaction).await?;
        publish(
            ctx,
            &mut tx,
            Mutation {
                actor: Actor::System,
                event_type: PRIVACY_REQUEST_ITEMS_ERASED.into(),
                record: RecordRef::new(PRIVACY_REQUEST_RECORD_TYPE, request.id),
                payload: json!({
                    "request_id": request.id.to_string(),
                    "notes": removed_notes,
                    "import_rows": redacted_rows,
                }),
                changes: vec![
                    FieldChange::new("notes_removed", Value::Null, json!(removed_notes)),
                    FieldChange::new("import_rows_redacted", Value::Null, json!(redacted_rows)),
                ],
                now,
                ip: None,
            },
        )
        .await?;
        tx.commit().await?;
        report.notes += removed_notes;
        report.import_rows += redacted_rows;
        report.history_entries += history + mentions;
    }
    Ok(ExecutionOutcome { report, file: None })
}

fn add_mentions(redaction: &mut Redaction, field: &PersonalField, value: &Value) {
    if field.sensitive
        || matches!(
            field.field_type.base_type(),
            FieldType::Email | FieldType::Phone
        )
    {
        redaction.add_strong_json(value);
    } else {
        redaction.add_identifiers_json(value);
    }
}

fn add_full_names(redaction: &mut Redaction, snapshot: &Value, fields: &[PersonalField]) {
    let personal_text = |key: &str| {
        fields
            .iter()
            .any(|field| field.key == key)
            .then(|| snapshot.get(key).and_then(Value::as_str))
            .flatten()
            .map(str::trim)
            .filter(|text| !text.is_empty())
    };
    for key in NAME_KEYS {
        if let Some(name) = personal_text(key) {
            redaction.add_full_name(name);
        }
    }
    if let (Some(first), Some(last)) = (personal_text("first_name"), personal_text("last_name")) {
        redaction.add_full_name(&format!("{first} {last}"));
    }
}

async fn record_redaction(
    ctx: &Ctx<'_>,
    definition: &EntityDefinition,
    record: &RecordRef,
    snapshot: &Value,
    fields: &[PersonalField],
) -> Result<Redaction, AppError> {
    let mut redaction = Redaction::new();
    let fields_ports = ctx.fields();
    let custom = CustomFieldReader::new(&fields_ports)
        .load(
            &AccessScope::system(),
            &record.record_type,
            &[record.record_id],
        )
        .await?
        .remove(&record.record_id)
        .unwrap_or_default();
    for field in fields {
        let value = match &field.source {
            FieldSource::Column(_) => {
                if PROTECTED_KEYS.contains(&field.key.as_str()) {
                    continue;
                }
                redaction.add_field(field.key.clone());
                snapshot.get(&field.key)
            }
            FieldSource::Custom(_) => {
                redaction.add_field(field.key.clone());
                redaction.add_field(custom_field_path(&field.key));
                custom.get(&field.key)
            }
        };
        if let Some(value) = value {
            redaction.add_json(value);
            add_mentions(&mut redaction, field, value);
        }
    }
    add_full_names(&mut redaction, snapshot, fields);
    for column in free_text_columns(definition) {
        redaction.add_field(column.as_str());
        if let Some(value) = snapshot.get(column.as_str()) {
            redaction.add_json(value);
        }
    }
    Ok(redaction)
}

async fn erase_record(
    ctx: &Ctx<'_>,
    request: &PrivacyRequest,
    record: &RecordRef,
) -> Result<Option<ErasedRecord>, AppError> {
    let scope = ctx.scope;
    let definition = definition(ctx, record.record_type.as_str())?;
    let privacy = PrivacyStore::of(ctx);
    let Some(snapshot) = privacy
        .record_snapshot(&definition, record.record_id)
        .await?
    else {
        return Ok(None);
    };
    let fields = personal_fields(ctx, &definition).await?;
    let redaction = record_redaction(ctx, &definition, record, &snapshot, &fields).await?;
    let now = ctx.now();
    let fields_ports = ctx.fields();
    let mut tx = ctx.begin().await?;
    if !privacy.claim_item(&mut tx, request.id, record).await? {
        return Ok(Some(ErasedRecord::default()));
    }
    let notes = privacy.delete_record_notes(&mut tx, record).await?;
    privacy.delete_duplicate_candidates(&mut tx, record).await?;
    let history = privacy
        .redact_history(&mut tx, std::slice::from_ref(record), &redaction)
        .await?
        + privacy.redact_mentions(&mut tx, &redaction).await?;
    let payload = json!({ "privacy_request_id": request.id.to_string() });
    let (event_type, changes, attachments, storage_keys): (
        Cow<'static, str>,
        Vec<FieldChange>,
        u64,
        Vec<String>,
    ) = match request.kind {
        PrivacyRequestKind::Anonymize => {
            let (attachments, files) = privacy.delete_record_attachments(&mut tx, record).await?;
            let file_ids: Vec<Uuid> = files.iter().map(|file| file.file_id).collect();
            privacy.delete_file_rows(&mut tx, &file_ids).await?;
            let assignments = anonymize_assignments(&definition, &fields);
            let columns: Vec<ColumnAssignment> = assignments
                .iter()
                .map(|(_, assignment)| assignment.clone())
                .collect();
            privacy
                .update_columns(&mut tx, &definition, record.record_id, &columns)
                .await?;
            let mut changes: Vec<FieldChange> = assignments
                .iter()
                .map(|(key, _)| FieldChange::redacted(key.clone()))
                .collect();
            for field in &fields {
                if let FieldSource::Custom(field_id) = field.source {
                    fields_ports
                        .values()
                        .delete(&mut tx, record, field_id)
                        .await?;
                    changes.push(FieldChange::redacted(custom_field_path(&field.key)));
                }
            }
            (
                anonymized_event(definition.record_type.as_str()).into(),
                changes,
                attachments,
                files.into_iter().map(|file| file.storage_key).collect(),
            )
        }
        PrivacyRequestKind::Delete => {
            let attachments = privacy.count_record_attachments(&mut tx, record).await?;
            let purged = CustomFieldWriter::new(&fields_ports)
                .purge_record(&mut tx, scope, record)
                .await?;
            let removed = if is_trashable(&definition) {
                privacy
                    .mark_deleted(&mut tx, &definition, record.record_id, now)
                    .await?;
                TrashStore::of(ctx)
                    .purge(&mut tx, &definition, record.record_id)
                    .await?
            } else {
                privacy.delete_activity(&mut tx, record.record_id).await?
            };
            if !removed {
                return Ok(None);
            }
            (
                erased_event(record.record_type.as_str()).into(),
                vec![FieldChange::redacted("record")],
                attachments,
                purged.orphan_storage_keys,
            )
        }
        PrivacyRequestKind::Export => return Ok(None),
    };
    publish(
        ctx,
        &mut tx,
        Mutation {
            actor: Actor::System,
            event_type,
            record: record.clone(),
            payload,
            changes,
            now,
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Some(ErasedRecord {
        notes,
        attachments,
        history,
        storage_keys,
    }))
}
