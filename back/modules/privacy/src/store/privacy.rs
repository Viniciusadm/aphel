use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    AttachmentData, FieldSource, PersonalField, PrivacyRequest, PrivacyRequestFilter,
    PrivacyRequestKind, PrivacyRequestStatus, PrivacyTargets, StoredFileRef, SubjectKind,
    SubjectRecord,
};
use crate::mysql::sql::{
    contains_pattern, file_reference_columns, instant, optional_instant, optional_uuid,
    phone_digits, push_ids, push_refs, unreferenced_file_predicate, user_reference, uuid_column,
};
use crate::service::subject::{Redaction, SubjectMatcher};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{EntityDefinition, RecordTypeRegistry};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::{RecordRef, RecordType};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::fields::FieldType;
use aphel_sdk::platform::infrastructure::repositories::mapper;

pub(crate) use aphel_sdk::records::TextAssignment as ColumnAssignment;

fn record_ref(
    row: &MySqlRow,
    type_column: &str,
    id_column: &str,
) -> Result<Option<RecordRef>, AppError> {
    let record_type: Option<String> = row.try_get(type_column)?;
    let id = optional_uuid(row, id_column)?;
    Ok(match (record_type, id) {
        (Some(record_type), Some(id)) => RecordType::parse(record_type)
            .ok()
            .map(|record_type| RecordRef::new(record_type, id)),
        _ => None,
    })
}

fn request_from_row(row: &MySqlRow) -> Result<PrivacyRequest, AppError> {
    let kind: String = row.try_get("kind")?;
    let status: String = row.try_get("status")?;
    let subject_kind: String = row.try_get("subject_kind")?;
    let Json(targets): Json<Value> = row.try_get("targets")?;
    let result: Option<Json<Value>> = row.try_get("result")?;
    Ok(PrivacyRequest {
        id: uuid_column(row, "id")?,
        kind: PrivacyRequestKind::parse(&kind)
            .map_err(|_| mapper::invalid_column("kind", &kind))?,
        status: PrivacyRequestStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        subject_kind: SubjectKind::parse(&subject_kind)
            .ok_or_else(|| mapper::invalid_column("subject_kind", &subject_kind))?,
        subject_label: row.try_get("subject_label")?,
        subject_hash: row.try_get("subject_hash")?,
        targets: PrivacyTargets::from_json(&targets)?,
        due_at: instant(row, "due_at")?,
        requested_by: user_reference(row, "requested_by", "requested_by_name")?,
        executed_by: user_reference(row, "executed_by", "executed_by_name")?,
        result: result.map(|Json(value)| value),
        failure_reason: row.try_get("failure_reason")?,
        file_id: optional_uuid(row, "file_id")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
        started_at: optional_instant(row, "started_at")?,
        finished_at: optional_instant(row, "finished_at")?,
    })
}

const REQUEST_SELECT: &str = "SELECT p.*, ur.display_name AS requested_by_name, ue.display_name AS executed_by_name \
     FROM privacy_requests p \
     LEFT JOIN users ur ON ur.id = p.requested_by \
     LEFT JOIN users ue ON ue.id = p.executed_by";

fn attachment_from_row(row: &MySqlRow) -> Result<Option<AttachmentData>, AppError> {
    let Some(record) = record_ref(row, "record_type", "record_id")? else {
        return Ok(None);
    };
    Ok(Some(AttachmentData {
        id: uuid_column(row, "id")?,
        file_id: uuid_column(row, "file_id")?,
        storage_key: row.try_get("storage_key")?,
        file_name: row.try_get("original_name")?,
        mime_type: row.try_get("mime_type")?,
        size: row.try_get::<u64, _>("size")?,
        record,
        created_at: instant(row, "created_at")?,
    }))
}

fn group_by_type(records: &[RecordRef]) -> BTreeMap<RecordType, Vec<Uuid>> {
    let mut grouped: BTreeMap<RecordType, Vec<Uuid>> = BTreeMap::new();
    for record in records {
        grouped
            .entry(record.record_type.clone())
            .or_default()
            .push(record.record_id);
    }
    grouped
}

#[derive(Clone)]
pub(crate) struct PrivacyStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl PrivacyStore {
    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self {
            pool: ctx.pool().clone(),
            registry: ctx.registry().clone(),
        }
    }

    fn personal_definition(&self, record_type: &RecordType) -> Option<&EntityDefinition> {
        self.registry
            .get(record_type)
            .filter(|definition| definition.capabilities.has_trait("privacy.personal_data"))
    }

    async fn matching_ids(
        &self,
        definition: &EntityDefinition,
        field: &PersonalField,
        matcher: &SubjectMatcher,
        limit: u32,
    ) -> Result<Vec<Uuid>, AppError> {
        use aphel_sdk::records::{StoredTextField, TextComparison, matching_entity_text_ids};
        let comparison = match matcher.kind {
            SubjectKind::Email => {
                let Some(email) = &matcher.email else {
                    return Ok(Vec::new());
                };
                if field.field_type.base_type() == FieldType::Email {
                    TextComparison::EqualFolded(email.clone())
                } else if field.field_type.is_textual() {
                    TextComparison::ContainsFolded(vec![email.clone()])
                } else {
                    return Ok(Vec::new());
                }
            }
            SubjectKind::Phone | SubjectKind::Document => {
                if !field.field_type.is_textual() {
                    return Ok(Vec::new());
                }
                TextComparison::DigitsContain(phone_digits(matcher))
            }
            SubjectKind::Text => {
                if !field.field_type.is_textual() {
                    return Ok(Vec::new());
                }
                TextComparison::ContainsFolded(matcher.text.iter().cloned().collect())
            }
        };
        let storage = match &field.source {
            FieldSource::Column(column) => StoredTextField::Column(column),
            FieldSource::Custom(id) => StoredTextField::Custom(*id),
        };
        matching_entity_text_ids(&self.pool, definition, storage, &comparison, limit).await
    }

    async fn titles(
        &self,
        record_type: &RecordType,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String, Option<String>, bool, bool)>, AppError> {
        use aphel_sdk::party::PartyTraits;
        let Some(definition) = self.personal_definition(record_type) else {
            return Ok(Vec::new());
        };
        let mut result = Vec::new();
        for id in ids {
            let record = RecordRef::new(record_type.clone(), *id);
            if let Some(snapshot) =
                aphel_sdk::records::inspect_entity(&self.pool, &self.registry, &record).await?
            {
                let value = |key: &str| {
                    snapshot
                        .get("values")
                        .and_then(|values| values.get(key))
                        .or_else(|| {
                            snapshot
                                .get("custom_fields")
                                .and_then(|values| values.get(key))
                        })
                };
                let title = definition
                    .title_field
                    .as_deref()
                    .and_then(value)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let subtitle = definition
                    .capabilities
                    .contact_point_columns()
                    .iter()
                    .find_map(|column| value(column).and_then(Value::as_str).map(str::to_owned));
                result.push((
                    *id,
                    title,
                    subtitle,
                    snapshot
                        .get("archived")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    snapshot
                        .get("deleted")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                ));
            }
        }
        Ok(result)
    }

    async fn redact_table(
        &self,
        tx: &mut MySqlTx,
        table: &str,
        column: &str,
        records: &[RecordRef],
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        let mut builder = QueryBuilder::new(format!(
            "SELECT id, {column} AS content FROM {table} WHERE TRUE"
        ));
        builder.push(" AND ");
        push_refs(&mut builder, "record_type", "record_id", records);
        builder.push(" FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        let mut updated = 0;
        for row in &rows {
            let id: String = row.try_get("id")?;
            let Json(mut content): Json<Value> = row.try_get("content")?;
            if redaction.apply(&mut content) {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {column} = ? WHERE id = ?"
                )))
                .bind(Json(content))
                .bind(id)
                .execute(&mut **tx)
                .await?;
                updated += 1;
            }
        }
        Ok(updated)
    }
}

const MENTION_BATCH: i64 = 500;

impl PrivacyStore {
    pub(crate) async fn redact_mentions(
        &self,
        tx: &mut MySqlTx,
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        let redaction = redaction.mentions_only();
        let patterns: Vec<String> = redaction.values().map(contains_pattern).collect();
        if patterns.is_empty() {
            return Ok(0);
        }
        let mut updated = 0;
        for (table, column) in [("domain_events", "payload"), ("audit_logs", "changes")] {
            let mut after = String::new();
            loop {
                let mut builder = QueryBuilder::new(format!(
                    "SELECT id, CAST({column} AS CHAR) AS content FROM {table} WHERE id > ",
                    column = column,
                    table = table,
                ));
                builder.push_bind(after.clone()).push(" AND (");
                for (index, pattern) in patterns.iter().enumerate() {
                    if index > 0 {
                        builder.push(" OR ");
                    }
                    builder
                        .push(format!("LOWER(CAST({} AS CHAR)) LIKE ", column))
                        .push_bind(pattern.clone());
                }
                builder
                    .push(") ORDER BY id LIMIT ")
                    .push_bind(MENTION_BATCH)
                    .push(" FOR UPDATE");
                let rows = builder.build().fetch_all(&mut **tx).await?;
                for row in &rows {
                    let id: String = row.try_get("id")?;
                    let content: Option<String> = row.try_get("content")?;
                    if let Some(redacted) = content
                        .as_deref()
                        .and_then(|text| redaction.apply_text(text))
                    {
                        sqlx::query(sqlx::AssertSqlSafe(format!(
                            "UPDATE {table} SET {column} = ? WHERE id = ?",
                            table = table,
                            column = column,
                        )))
                        .bind(redacted)
                        .bind(&id)
                        .execute(&mut **tx)
                        .await?;
                        updated += 1;
                    }
                    after = id;
                }
                if (rows.len() as i64) < MENTION_BATCH {
                    break;
                }
            }
        }
        Ok(updated)
    }
}

impl PrivacyStore {
    pub(crate) async fn search_records(
        &self,
        definition: &EntityDefinition,
        fields: &[PersonalField],
        matcher: &SubjectMatcher,
        limit: u32,
    ) -> Result<Vec<SubjectRecord>, AppError> {
        let mut matches: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
        for field in fields {
            for id in self.matching_ids(definition, field, matcher, limit).await? {
                let labels = matches.entry(id).or_default();
                if !labels.contains(&field.label) {
                    labels.push(field.label.clone());
                }
            }
        }
        let ids: Vec<Uuid> = matches.keys().copied().take(limit as usize).collect();
        let titles = self.titles(&definition.record_type, &ids).await?;
        Ok(titles
            .into_iter()
            .map(|(id, title, subtitle, archived, deleted)| SubjectRecord {
                record: RecordRef::new(definition.record_type.clone(), id),
                title,
                subtitle,
                matched_fields: matches.remove(&id).unwrap_or_default(),
                archived,
                deleted,
            })
            .collect())
    }

    pub(crate) async fn attachments(
        &self,
        records: &[RecordRef],
    ) -> Result<Vec<AttachmentData>, AppError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new(
            "SELECT a.id, a.file_id, a.record_type, a.record_id, a.created_at, f.storage_key, f.original_name, \
             f.mime_type, f.size FROM attachments a JOIN files f ON f.id = a.file_id \
             WHERE ",
        );
        push_refs(&mut builder, "a.record_type", "a.record_id", records);
        builder.push(" ORDER BY a.created_at");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut attachments = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(attachment) = attachment_from_row(row)? {
                attachments.push(attachment);
            }
        }
        Ok(attachments)
    }

    pub(crate) async fn existing_targets(
        &self,
        targets: &PrivacyTargets,
    ) -> Result<PrivacyTargets, AppError> {
        let mut existing = PrivacyTargets {
            records: Vec::new(),
            items: targets.items.clone(),
        };
        for (record_type, ids) in group_by_type(&targets.records) {
            let Some(definition) = self.personal_definition(&record_type) else {
                continue;
            };
            for id in aphel_sdk::records::existing_entity_ids(&self.pool, definition, &ids).await? {
                existing
                    .records
                    .push(RecordRef::new(record_type.clone(), id));
            }
        }
        Ok(existing)
    }

    pub(crate) async fn record_titles(
        &self,
        records: &[RecordRef],
    ) -> Result<HashMap<RecordRef, String>, AppError> {
        let mut titles = HashMap::new();
        for (record_type, ids) in group_by_type(records) {
            for (id, title, _, _, _) in self.titles(&record_type, &ids).await? {
                titles.insert(RecordRef::new(record_type.clone(), id), title);
            }
        }
        Ok(titles)
    }

    pub(crate) async fn insert_request(
        &self,
        tx: &mut MySqlTx,
        request: &PrivacyRequest,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO privacy_requests (id, kind, status, subject_kind, subject_label, \
             subject_hash, targets, due_at, requested_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(request.id.to_string())
        .bind(request.kind.as_str())
        .bind(request.status.as_str())
        .bind(request.subject_kind.as_str())
        .bind(&request.subject_label)
        .bind(&request.subject_hash)
        .bind(Json(request.targets.to_json()))
        .bind(mapper::naive(request.due_at))
        .bind(
            request
                .requested_by
                .as_ref()
                .map(|user| user.id.as_str().to_owned()),
        )
        .bind(mapper::naive(request.created_at))
        .bind(mapper::naive(request.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_request(
        &self,
        tx: &mut MySqlTx,
        request: &PrivacyRequest,
        from: PrivacyRequestStatus,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE privacy_requests SET status = ?, executed_by = ?, result = ?, failure_reason = ?, \
             file_id = ?, updated_at = ?, started_at = ?, finished_at = ? \
             WHERE id = ? AND status = ?",
        )
        .bind(request.status.as_str())
        .bind(request.executed_by.as_ref().map(|user| user.id.as_str().to_owned()))
        .bind(request.result.clone().map(Json))
        .bind(&request.failure_reason)
        .bind(request.file_id.map(|id| id.to_string()))
        .bind(mapper::naive(request.updated_at))
        .bind(request.started_at.map(mapper::naive))
        .bind(request.finished_at.map(mapper::naive))
        .bind(request.id.to_string())
        .bind(from.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn find_request(
        &self,
        request_id: Uuid,
    ) -> Result<Option<PrivacyRequest>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{REQUEST_SELECT} WHERE p.id = ?"
        )))
        .bind(request_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(request_from_row).transpose()
    }

    pub(crate) async fn list_requests(
        &self,
        filter: &PrivacyRequestFilter,
        window: ListWindow,
    ) -> Result<Listing<PrivacyRequest>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<MySql>| {
            builder.push(" WHERE TRUE");
            if let Some(status) = filter.status {
                builder.push(" AND p.status = ").push_bind(status.as_str());
            }
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM privacy_requests p");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::new(REQUEST_SELECT);
        push_filter(&mut builder);
        builder
            .push(" ORDER BY p.created_at DESC, p.id DESC LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(request_from_row)
                .collect::<Result<_, _>>()?,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn expired_exports(
        &self,
        finished_before: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<Uuid>, AppError> {
        let rows = sqlx::query(
            "SELECT id FROM privacy_requests WHERE kind = 'export' AND file_id IS NOT NULL \
             AND finished_at < ? ORDER BY finished_at LIMIT ?",
        )
        .bind(mapper::naive(finished_before))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(|row| uuid_column(row, "id")).collect()
    }

    pub(crate) async fn record_snapshot(
        &self,
        definition: &EntityDefinition,
        id: Uuid,
    ) -> Result<Option<Value>, AppError> {
        Ok(aphel_sdk::records::inspect_entity(
            &self.pool,
            &self.registry,
            &RecordRef::new(definition.record_type.clone(), id),
        )
        .await?
        .and_then(|mut value| value.get_mut("values").map(Value::take)))
    }

    pub(crate) async fn update_columns(
        &self,
        tx: &mut MySqlTx,
        definition: &EntityDefinition,
        id: Uuid,
        assignments: &[ColumnAssignment],
    ) -> Result<bool, AppError> {
        aphel_sdk::records::update_entity_text(tx, definition, id, assignments).await
    }

    pub(crate) async fn claim_item(
        &self,
        tx: &mut MySqlTx,
        request_id: Uuid,
        record: &RecordRef,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO privacy_request_items (request_id, record_type, record_id) VALUES (?, ?, ?)",
        )
        .bind(request_id.to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn count_record_attachments(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM attachments WHERE record_type = ? AND record_id = ?",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_one(&mut **tx)
        .await?;
        Ok(u64::try_from(count).unwrap_or_default())
    }

    pub(crate) async fn delete_record_attachments(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<(u64, Vec<StoredFileRef>), AppError> {
        let file_ids: Vec<String> = sqlx::query_scalar(
            "SELECT file_id FROM attachments WHERE record_type = ? AND record_id = ? FOR UPDATE",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_all(&mut **tx)
        .await?;
        let removed =
            sqlx::query("DELETE FROM attachments WHERE record_type = ? AND record_id = ?")
                .bind(record.record_type.as_str())
                .bind(record.record_id.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
        let ids: Vec<Uuid> = file_ids
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();
        if ids.is_empty() {
            return Ok((removed, Vec::new()));
        }
        let mut builder = QueryBuilder::new("SELECT f.id, f.storage_key FROM files f WHERE TRUE");
        builder.push(" AND f.id");
        push_ids(&mut builder, &ids);
        builder.push(format!(
            " AND {}",
            unreferenced_file_predicate(&file_reference_columns(&self.pool).await?)
        ));
        let rows = builder.build().fetch_all(&mut **tx).await?;
        let files = rows
            .iter()
            .map(|row| {
                Ok(StoredFileRef {
                    file_id: uuid_column(row, "id")?,
                    storage_key: row.try_get("storage_key")?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok((removed, files))
    }

    pub(crate) async fn redact_history(
        &self,
        tx: &mut MySqlTx,
        records: &[RecordRef],
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        if records.is_empty() || redaction.is_empty() {
            return Ok(0);
        }
        let audit = self
            .redact_table(tx, "audit_logs", "changes", records, redaction)
            .await?;
        let events = self
            .redact_table(tx, "domain_events", "payload", records, redaction)
            .await?;
        Ok(audit + events)
    }

    pub(crate) async fn mark_deleted(
        &self,
        tx: &mut MySqlTx,
        definition: &EntityDefinition,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        aphel_sdk::records::mark_entity_deleted(tx, definition, id, now).await
    }

    pub(crate) async fn delete_file_rows(
        &self,
        tx: &mut MySqlTx,
        file_ids: &[Uuid],
    ) -> Result<(), AppError> {
        if file_ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::new("DELETE FROM files WHERE TRUE");
        builder.push(" AND id");
        push_ids(&mut builder, file_ids);
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn file_ref(&self, file_id: Uuid) -> Result<Option<StoredFileRef>, AppError> {
        let key: Option<String> = sqlx::query_scalar("SELECT storage_key FROM files WHERE id = ?")
            .bind(file_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(key.map(|storage_key| StoredFileRef {
            file_id,
            storage_key,
        }))
    }
}
