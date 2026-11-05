use serde_json::{Value, json};
use std::borrow::Cow;
use uuid::Uuid;

use crate::domain::{FieldSource, PersonalField, PrivacyRequest, PrivacyRequestKind, RunReport};
use crate::service::requests::ExecutionOutcome;
use crate::service::subject::Redaction;
use crate::service::{
    Mutation, definition, personal_fields, publish, remove_stored_files, run_external_sources,
};
use crate::store::{ColumnAssignment, PrivacyStore, free_text_columns};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldValueRepository, CustomFieldWriter,
    EntityDefinition, custom_field_path,
};
use aphel_sdk::platform::application::shared::{Actor, FieldChange, FileStorage, RecordRef};
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::domain::fields::FieldType;

use aphel_sdk::privacy::PrivacyTraits;
fn protected(definition: &EntityDefinition, key: &str) -> bool {
    definition
        .capabilities
        .privacy_policy_data()
        .map(|policy| policy.protected_fields.iter().any(|field| field == key))
        .unwrap_or(true)
        || definition.system_field_by_key(key).is_some_and(|field| {
            !field.writable || definition.owner_column.as_deref() == Some(field.column.as_str())
        })
}

pub(crate) fn anonymized_event(record_type: &str) -> String {
    format!("{record_type}.anonymized")
}

pub(crate) fn erased_event(record_type: &str) -> String {
    format!("{record_type}.purged")
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
        if protected(definition, &field.key) {
            continue;
        }
        let value = if field.required {
            if !field.field_type.is_textual() {
                continue;
            }
            Some(aphel_sdk::platform::application::shared::REDACTED.to_owned())
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
    let mut combined = Redaction::new();
    for record in &request.targets.records {
        let definition = definition(ctx, record.record_type.as_str())?;
        definition.capabilities.privacy_policy_data()?;
        if let Some(snapshot) = PrivacyStore::of(ctx)
            .record_snapshot(&definition, record.record_id)
            .await?
        {
            let fields = personal_fields(ctx, &definition).await?;
            combined.merge(&record_redaction(ctx, &definition, record, &snapshot, &fields).await?);
        }
    }
    for source in super::sources::available(ctx).await? {
        let ids = super::sources::item_ids(&request.targets, source.key());
        let snapshot = source.snapshot(ctx, &request.targets.records, &ids).await?;
        for identifier in snapshot.identifiers {
            combined.add_json(&identifier);
            combined.add_identifiers_json(&identifier);
        }
    }
    super::sources::erase(ctx, storage, request, &combined, &mut report).await?;
    for record in &request.targets.records {
        match erase_record(ctx, request, record).await {
            Ok(Some(erased)) => {
                report.records += 1;
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

fn add_full_names(
    redaction: &mut Redaction,
    definition: &EntityDefinition,
    snapshot: &Value,
    fields: &[PersonalField],
) {
    if let Some(key) = &definition.title_field
        && fields.iter().any(|field| field.key == *key)
        && let Some(name) = snapshot.get(key).and_then(Value::as_str)
    {
        redaction.add_full_name(name);
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
                if protected(definition, &field.key) {
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
    add_full_names(&mut redaction, definition, snapshot, fields);
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
            privacy
                .mark_deleted(&mut tx, &definition, record.record_id, now)
                .await?;
            let removed =
                aphel_sdk::records::purge_entity(&mut tx, &definition, record.record_id).await?;
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
        attachments,
        history,
        storage_keys,
    }))
}
