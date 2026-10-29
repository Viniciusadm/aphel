use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    ActivityData, ActivityLocation, AttachmentData, FieldSource, ImportRowData, ImportRowTarget,
    MatchReason, NoteData, PersonalField, PrivacyRequest, PrivacyRequestFilter, PrivacyRequestKind,
    PrivacyRequestStatus, PrivacyTargets, StoredFileRef, SubjectActivity, SubjectImportRow,
    SubjectKind, SubjectNote, SubjectRecord,
};
use crate::mysql::sql::{
    contains_pattern, file_reference_columns, instant, optional_instant, optional_uuid,
    personal_text_columns, phone_digits, push_ids, push_refs, push_text_match, record_title_sql,
    unreferenced_file_predicate, user_reference, uuid_column,
};
use crate::service::subject::{Redaction, SubjectMatcher};
use crate::store::free_text_columns;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{
    RecordTypeDefinition, RecordTypeRegistry, is_sql_identifier,
};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnAssignment {
    pub(crate) column: String,
    pub(crate) value: Option<String>,
}

const EXCERPT_LENGTH: usize = 200;

fn excerpt(body: &str) -> String {
    let mut text: String = body.chars().take(EXCERPT_LENGTH).collect();
    if body.chars().count() > EXCERPT_LENGTH {
        text.push('…');
    }
    text
}

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

fn push_field_match(
    builder: &mut QueryBuilder<MySql>,
    expression: &str,
    field_type: FieldType,
    matcher: &SubjectMatcher,
) -> bool {
    match matcher.kind {
        SubjectKind::Email => {
            let Some(email) = &matcher.email else {
                return false;
            };
            match field_type {
                FieldType::Email => {
                    builder
                        .push(format!("LOWER({expression}) = "))
                        .push_bind(email.clone());
                }
                FieldType::Text | FieldType::LongText | FieldType::Url => {
                    builder
                        .push(format!("LOWER({expression}) LIKE "))
                        .push_bind(contains_pattern(email));
                }
                _ => return false,
            }
            true
        }
        SubjectKind::Phone | SubjectKind::Document => {
            let digits = phone_digits(matcher);
            match field_type {
                FieldType::Phone | FieldType::Text | FieldType::LongText => {
                    builder.push("(");
                    let mut first = true;
                    if field_type == FieldType::Phone
                        && let Some(phone) = &matcher.phone
                    {
                        builder
                            .push(format!("{expression} = "))
                            .push_bind(phone.clone());
                        first = false;
                    }
                    for value in &digits {
                        if !first {
                            builder.push(" OR ");
                        }
                        first = false;
                        if field_type == FieldType::LongText {
                            builder
                                .push(format!("REGEXP_REPLACE({expression}, '[^0-9]', '') LIKE "))
                                .push_bind(contains_pattern(value));
                        } else {
                            builder
                                .push(format!("REGEXP_REPLACE({expression}, '[^0-9]', '') = "))
                                .push_bind(value.clone());
                        }
                    }
                    if first {
                        builder.push("FALSE");
                    }
                    builder.push(")");
                    true
                }
                _ => false,
            }
        }
        SubjectKind::Text => {
            let Some(text) = &matcher.text else {
                return false;
            };
            if !field_type.is_textual() {
                return false;
            }
            builder
                .push(format!("{expression} LIKE "))
                .push_bind(contains_pattern(text));
            true
        }
    }
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
        targets: PrivacyTargets::from_json(&targets),
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

fn note_from_row(row: &MySqlRow) -> Result<Option<NoteData>, AppError> {
    let Some(record) = record_ref(row, "record_type", "record_id")? else {
        return Ok(None);
    };
    Ok(Some(NoteData {
        id: uuid_column(row, "id")?,
        record,
        body: row.try_get("body")?,
        visibility: row.try_get("visibility")?,
        author: row.try_get("author_name")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    }))
}

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

fn push_import_rows(builder: &mut QueryBuilder<MySql>, rows: &[ImportRowTarget]) {
    builder.push("(");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            builder.push(" OR ");
        }
        builder
            .push("(r.import_job_id = ")
            .push_bind(row.import_job_id.to_string())
            .push(" AND r.row_index = ")
            .push_bind(row.row_index)
            .push(")");
    }
    builder.push(")");
}

fn activity_location(row: &MySqlRow) -> Result<Option<ActivityLocation>, AppError> {
    let latitude: Option<f64> = row.try_get("latitude")?;
    let longitude: Option<f64> = row.try_get("longitude")?;
    let (Some(latitude), Some(longitude)) = (latitude, longitude) else {
        return Ok(None);
    };
    Ok(Some(ActivityLocation {
        latitude,
        longitude,
        accuracy_m: row.try_get("location_accuracy_m")?,
        captured_at: optional_instant(row, "location_captured_at")?,
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

    fn personal_definition(&self, record_type: &RecordType) -> Option<&RecordTypeDefinition> {
        self.registry
            .get(record_type)
            .filter(|definition| definition.capabilities.personal_data)
    }

    async fn matching_ids(
        &self,
        definition: &RecordTypeDefinition,
        field: &PersonalField,
        matcher: &SubjectMatcher,
        limit: u32,
    ) -> Result<Vec<Uuid>, AppError> {
        let mut builder = match &field.source {
            FieldSource::Column(column) => {
                if !is_sql_identifier(column) {
                    return Ok(Vec::new());
                }
                let mut builder = QueryBuilder::new(format!(
                    "SELECT r.id FROM {} r WHERE TRUE",
                    definition.table
                ));
                builder.push(" AND ");
                if !push_field_match(
                    &mut builder,
                    &format!("r.{column}"),
                    field.field_type,
                    matcher,
                ) {
                    return Ok(Vec::new());
                }
                builder
            }
            FieldSource::Custom(field_id) => {
                let mut builder = QueryBuilder::new(
                    "SELECT v.record_id AS id FROM custom_field_values v WHERE v.record_type = ",
                );
                builder
                    .push_bind(definition.record_type.as_str().to_owned())
                    .push(" AND v.field_id = ")
                    .push_bind(field_id.to_string())
                    .push(" AND ");
                if !push_field_match(
                    &mut builder,
                    "COALESCE(v.value_text, v.value_long_text)",
                    field.field_type,
                    matcher,
                ) {
                    return Ok(Vec::new());
                }
                builder
            }
        };
        builder.push(" LIMIT ").push_bind(i64::from(limit));
        let ids: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(ids
            .into_iter()
            .filter_map(|id| Uuid::parse_str(&id).ok())
            .collect())
    }

    async fn titles(
        &self,
        record_type: &RecordType,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String, Option<String>, bool, bool)>, AppError> {
        let Some(definition) = self.personal_definition(record_type) else {
            return Ok(Vec::new());
        };
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let table = &definition.table;
        let (title, subtitle) = record_title_sql(definition);
        let is_set = |column: Option<&str>| {
            column.map_or_else(
                || "FALSE".to_owned(),
                |column| format!("r.{column} IS NOT NULL"),
            )
        };
        let archived = is_set(definition.archived_column.as_deref());
        let deleted = is_set(definition.deleted_column.as_deref());
        let mut builder = QueryBuilder::new(format!(
            "SELECT r.id, {title} AS title, {subtitle} AS subtitle, CAST({archived} AS SIGNED) AS archived, \
             CAST({deleted} AS SIGNED) AS deleted FROM {table} r WHERE TRUE"
        ));
        builder.push(" AND r.id");
        push_ids(&mut builder, ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok((
                    uuid_column(row, "id")?,
                    row.try_get::<Option<String>, _>("title")?
                        .unwrap_or_default(),
                    row.try_get("subtitle")?,
                    row.try_get::<i64, _>("archived")? != 0,
                    row.try_get::<i64, _>("deleted")? != 0,
                ))
            })
            .collect()
    }

    async fn linked_activities(
        &self,
        records: &[RecordRef],
    ) -> Result<Vec<(Uuid, RecordRef)>, AppError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let mut direct =
            QueryBuilder::new("SELECT a.id, a.record_type, a.record_id FROM activities a WHERE ");
        push_refs(&mut direct, "a.record_type", "a.record_id", records);
        let mut linked = QueryBuilder::new(
            "SELECT l.activity_id AS id, l.record_type, l.record_id FROM activity_links l WHERE ",
        );
        push_refs(&mut linked, "l.record_type", "l.record_id", records);
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for rows in [
            direct.build().fetch_all(&self.pool).await?,
            linked.build().fetch_all(&self.pool).await?,
        ] {
            for row in &rows {
                let id = uuid_column(row, "id")?;
                if let Some(record) = record_ref(row, "record_type", "record_id")?
                    && seen.insert((id, record.clone()))
                {
                    result.push((id, record));
                }
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
        for target in personal_text_columns(&self.pool).await? {
            let mut after = String::new();
            loop {
                let mut builder = QueryBuilder::new(format!(
                    "SELECT id, CAST({column} AS CHAR) AS content FROM {table} WHERE id > ",
                    column = target.column,
                    table = target.table,
                ));
                builder.push_bind(after.clone()).push(" AND (");
                for (index, pattern) in patterns.iter().enumerate() {
                    if index > 0 {
                        builder.push(" OR ");
                    }
                    builder
                        .push(format!("LOWER(CAST({} AS CHAR)) LIKE ", target.column))
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
                            table = target.table,
                            column = target.column,
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
        definition: &RecordTypeDefinition,
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

    pub(crate) async fn search_notes(
        &self,
        matcher: &SubjectMatcher,
        records: &[RecordRef],
        limit: u32,
    ) -> Result<Vec<SubjectNote>, AppError> {
        let mut notes: Vec<SubjectNote> = Vec::new();
        let mut seen = HashSet::new();
        let mut mentions = QueryBuilder::new(
            "SELECT n.id, n.record_type, n.record_id, n.body, n.created_at FROM notes n WHERE ",
        );
        push_text_match(&mut mentions, "n.body", matcher);
        mentions
            .push(" ORDER BY n.created_at DESC LIMIT ")
            .push_bind(i64::from(limit));
        let mut batches = vec![(
            MatchReason::Mention,
            mentions.build().fetch_all(&self.pool).await?,
        )];
        if !records.is_empty() {
            let mut owned = QueryBuilder::new(
                "SELECT n.id, n.record_type, n.record_id, n.body, n.created_at FROM notes n WHERE ",
            );
            push_refs(&mut owned, "n.record_type", "n.record_id", records);
            owned
                .push(" ORDER BY n.created_at DESC LIMIT ")
                .push_bind(i64::from(limit));
            batches.push((
                MatchReason::Record,
                owned.build().fetch_all(&self.pool).await?,
            ));
        }
        for (reason, rows) in batches {
            for row in &rows {
                let id = uuid_column(row, "id")?;
                let Some(record) = record_ref(row, "record_type", "record_id")? else {
                    continue;
                };
                if !seen.insert(id) {
                    continue;
                }
                let body: String = row.try_get("body")?;
                notes.push(SubjectNote {
                    id,
                    record,
                    excerpt: excerpt(&body),
                    created_at: instant(row, "created_at")?,
                    reason,
                });
            }
        }
        Ok(notes)
    }

    pub(crate) async fn search_activities(
        &self,
        matcher: &SubjectMatcher,
        records: &[RecordRef],
        limit: u32,
    ) -> Result<Vec<SubjectActivity>, AppError> {
        let mut reasons: Vec<(Uuid, Option<RecordRef>, MatchReason)> = self
            .linked_activities(records)
            .await?
            .into_iter()
            .map(|(id, record)| (id, Some(record), MatchReason::Record))
            .collect();
        let mut mentions =
            QueryBuilder::new("SELECT a.id, a.record_type, a.record_id FROM activities a WHERE (");
        push_text_match(&mut mentions, "a.title", matcher);
        mentions.push(" OR ");
        push_text_match(&mut mentions, "COALESCE(a.description, '')", matcher);
        mentions.push(") LIMIT ").push_bind(i64::from(limit));
        for row in &mentions.build().fetch_all(&self.pool).await? {
            let id = uuid_column(row, "id")?;
            if !reasons.iter().any(|(existing, _, _)| *existing == id) {
                reasons.push((
                    id,
                    record_ref(row, "record_type", "record_id")?,
                    MatchReason::Mention,
                ));
            }
        }
        reasons.truncate(limit as usize);
        if reasons.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = reasons.iter().map(|(id, _, _)| *id).collect();
        let mut builder = QueryBuilder::new(
            "SELECT a.id, a.title, a.status, a.due_at FROM activities a WHERE a.id",
        );
        push_ids(&mut builder, &ids);
        let mut details = HashMap::new();
        for row in &builder.build().fetch_all(&self.pool).await? {
            details.insert(
                uuid_column(row, "id")?,
                (
                    row.try_get::<String, _>("title")?,
                    row.try_get::<String, _>("status")?,
                    optional_instant(row, "due_at")?,
                ),
            );
        }
        Ok(reasons
            .into_iter()
            .filter_map(|(id, record, reason)| {
                let (title, status, due_at) = details.get(&id)?.clone();
                Some(SubjectActivity {
                    id,
                    title,
                    status,
                    record,
                    due_at,
                    reason,
                })
            })
            .collect())
    }

    pub(crate) async fn search_import_rows(
        &self,
        matcher: &SubjectMatcher,
        limit: u32,
    ) -> Result<Vec<SubjectImportRow>, AppError> {
        let mut builder = QueryBuilder::new(
            "SELECT r.import_job_id, r.row_index, r.record_id, r.status, j.record_type, j.file_name, j.created_at \
             FROM import_rows r JOIN import_jobs j ON j.id = r.import_job_id WHERE ",
        );
        push_text_match(&mut builder, "CAST(r.raw AS CHAR)", matcher);
        builder
            .push(" ORDER BY j.created_at DESC, r.row_index LIMIT ")
            .push_bind(i64::from(limit));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(SubjectImportRow {
                    target: ImportRowTarget {
                        import_job_id: uuid_column(row, "import_job_id")?,
                        row_index: row.try_get::<u32, _>("row_index")?,
                    },
                    record_type: row.try_get("record_type")?,
                    record_id: optional_uuid(row, "record_id")?,
                    status: row.try_get("status")?,
                    file_name: row.try_get("file_name")?,
                    created_at: instant(row, "created_at")?,
                })
            })
            .collect()
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
        let mut existing = PrivacyTargets::default();
        for (record_type, ids) in group_by_type(&targets.records) {
            let Some(table) = self
                .personal_definition(&record_type)
                .map(|definition| definition.table.clone())
            else {
                continue;
            };
            let mut builder = QueryBuilder::new(format!("SELECT id FROM {table} WHERE TRUE"));
            builder.push(" AND id");
            push_ids(&mut builder, &ids);
            let found: HashSet<String> = builder
                .build_query_scalar::<String>()
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .collect();
            existing.records.extend(
                ids.into_iter()
                    .filter(|id| found.contains(&id.to_string()))
                    .map(|id| RecordRef::new(record_type.clone(), id)),
            );
        }
        if !targets.notes.is_empty() {
            let mut builder = QueryBuilder::new("SELECT id FROM notes WHERE TRUE");
            builder.push(" AND id");
            push_ids(&mut builder, &targets.notes);
            let found: HashSet<String> = builder
                .build_query_scalar::<String>()
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .collect();
            existing.notes = targets
                .notes
                .iter()
                .copied()
                .filter(|id| found.contains(&id.to_string()))
                .collect();
        }
        if !targets.import_rows.is_empty() {
            existing.import_rows = self
                .import_rows(&targets.import_rows)
                .await?
                .into_iter()
                .map(|row| row.target)
                .collect();
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
        definition: &RecordTypeDefinition,
        record_id: Uuid,
    ) -> Result<Option<Value>, AppError> {
        let mut pairs: Vec<(String, String)> = vec![("id".to_owned(), "id".to_owned())];
        for field in &definition.system_fields {
            if let Some(column) = field.stored_column() {
                pairs.push((field.key.as_str().to_owned(), column.to_owned()));
            }
        }
        for column in free_text_columns(definition) {
            if !pairs.iter().any(|(_, existing)| existing == column) {
                pairs.push((column.clone(), column.clone()));
            }
        }
        for column in [&definition.archived_column, &definition.deleted_column]
            .into_iter()
            .flatten()
        {
            if !pairs.iter().any(|(_, existing)| existing == column) {
                pairs.push((column.clone(), column.clone()));
            }
        }
        let arguments: Vec<String> = pairs
            .iter()
            .filter(|(key, column)| is_sql_identifier(key) && is_sql_identifier(column))
            .map(|(key, column)| format!("'{key}', r.{column}"))
            .collect();
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT JSON_OBJECT({}) AS snapshot FROM {} r WHERE r.id = ?",
            arguments.join(", "),
            definition.table
        )))
        .bind(record_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => {
                let Json(snapshot): Json<Value> = row.try_get("snapshot")?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn record_notes(
        &self,
        records: &[RecordRef],
    ) -> Result<Vec<NoteData>, AppError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new(
            "SELECT n.*, u.display_name AS author_name FROM notes n LEFT JOIN users u ON u.id = n.author_id \
             WHERE ",
        );
        push_refs(&mut builder, "n.record_type", "n.record_id", records);
        builder.push(" ORDER BY n.created_at");
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(note_from_row)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    pub(crate) async fn notes_by_id(&self, ids: &[Uuid]) -> Result<Vec<NoteData>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new(
            "SELECT n.*, u.display_name AS author_name FROM notes n LEFT JOIN users u ON u.id = n.author_id \
             WHERE n.id",
        );
        push_ids(&mut builder, ids);
        builder.push(" ORDER BY n.created_at");
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(note_from_row)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    pub(crate) async fn record_activities(
        &self,
        records: &[RecordRef],
    ) -> Result<Vec<ActivityData>, AppError> {
        let links = self.linked_activities(records).await?;
        if links.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = links.iter().map(|(id, _)| *id).collect();
        let mut builder = QueryBuilder::new(
            "SELECT a.*, t.name AS type_name, CAST(a.location_latitude AS DOUBLE) AS latitude, \
             CAST(a.location_longitude AS DOUBLE) AS longitude \
             FROM activities a LEFT JOIN activity_types t ON t.id = a.activity_type_id \
             WHERE a.id",
        );
        push_ids(&mut builder, &ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut by_id = HashMap::new();
        for row in &rows {
            by_id.insert(
                uuid_column(row, "id")?,
                ActivityData {
                    id: uuid_column(row, "id")?,
                    title: row.try_get("title")?,
                    description: row.try_get("description")?,
                    activity_type: row.try_get("type_name")?,
                    status: row.try_get("status")?,
                    priority: row.try_get("priority")?,
                    due_at: optional_instant(row, "due_at")?,
                    completed_at: optional_instant(row, "completed_at")?,
                    record: None,
                    location: activity_location(row)?,
                    created_at: instant(row, "created_at")?,
                },
            );
        }
        Ok(links
            .into_iter()
            .filter_map(|(id, record)| {
                let mut activity = by_id.get(&id)?.clone();
                activity.record = Some(record);
                Some(activity)
            })
            .collect())
    }

    pub(crate) async fn import_rows(
        &self,
        rows: &[ImportRowTarget],
    ) -> Result<Vec<ImportRowData>, AppError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new(
            "SELECT r.import_job_id, r.row_index, r.status, r.record_id, r.raw FROM import_rows r WHERE ",
        );
        push_import_rows(&mut builder, rows);
        let found = builder.build().fetch_all(&self.pool).await?;
        found
            .iter()
            .map(|row| {
                let Json(raw): Json<Value> = row.try_get("raw")?;
                Ok(ImportRowData {
                    target: ImportRowTarget {
                        import_job_id: uuid_column(row, "import_job_id")?,
                        row_index: row.try_get::<u32, _>("row_index")?,
                    },
                    status: row.try_get("status")?,
                    record_id: optional_uuid(row, "record_id")?,
                    raw,
                })
            })
            .collect()
    }

    pub(crate) async fn update_columns(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        record_id: Uuid,
        assignments: &[ColumnAssignment],
    ) -> Result<bool, AppError> {
        let allowed: HashSet<&str> = definition
            .system_fields
            .iter()
            .filter_map(|field| field.stored_column())
            .chain(free_text_columns(definition).iter().map(String::as_str))
            .collect();
        let valid: Vec<&ColumnAssignment> = assignments
            .iter()
            .filter(|assignment| {
                is_sql_identifier(&assignment.column)
                    && allowed.contains(assignment.column.as_str())
            })
            .collect();
        if valid.is_empty() {
            return Ok(false);
        }
        let mut builder = QueryBuilder::new(format!("UPDATE {} SET ", definition.table));
        {
            let mut separated = builder.separated(", ");
            for assignment in &valid {
                separated
                    .push(format!("{} = ", assignment.column))
                    .push_bind_unseparated(assignment.value.clone());
            }
        }
        builder
            .push(" WHERE id = ")
            .push_bind(record_id.to_string());
        let result = builder.build().execute(&mut **tx).await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn delete_record_notes(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM notes WHERE record_type = ? AND record_id = ?")
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected())
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

    pub(crate) async fn delete_notes(
        &self,
        tx: &mut MySqlTx,
        ids: &[Uuid],
    ) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut builder = QueryBuilder::new("DELETE FROM notes WHERE TRUE");
        builder.push(" AND id");
        push_ids(&mut builder, ids);
        let result = builder.build().execute(&mut **tx).await?;
        Ok(result.rows_affected())
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

    pub(crate) async fn delete_duplicate_candidates(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM duplicate_candidates WHERE record_type = ? \
             AND (record_id = ? OR duplicate_id = ?)",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(record.record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
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

    pub(crate) async fn redact_import_rows(
        &self,
        tx: &mut MySqlTx,
        rows: &[ImportRowTarget],
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        if rows.is_empty() {
            return Ok(0);
        }
        let mut builder = QueryBuilder::new(
            "SELECT r.import_job_id, r.row_index, r.raw FROM import_rows r WHERE ",
        );
        push_import_rows(&mut builder, rows);
        builder.push(" FOR UPDATE");
        let found = builder.build().fetch_all(&mut **tx).await?;
        let mut updated = 0;
        for row in &found {
            let job: String = row.try_get("import_job_id")?;
            let index: u32 = row.try_get("row_index")?;
            let Json(mut raw): Json<Value> = row.try_get("raw")?;
            if redaction.apply(&mut raw) {
                sqlx::query(
                    "UPDATE import_rows SET raw = ? WHERE import_job_id = ? AND row_index = ?",
                )
                .bind(Json(raw))
                .bind(job)
                .bind(index)
                .execute(&mut **tx)
                .await?;
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub(crate) async fn mark_deleted(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        record_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let Some(column) = definition
            .deleted_column
            .as_deref()
            .filter(|column| is_sql_identifier(column))
        else {
            return Ok(false);
        };
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {} SET {column} = COALESCE({column}, ?) WHERE id = ?",
            definition.table
        )))
        .bind(mapper::naive(now))
        .bind(record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn delete_activity(
        &self,
        tx: &mut MySqlTx,
        activity_id: Uuid,
    ) -> Result<bool, AppError> {
        let id = activity_id.to_string();
        sqlx::query("DELETE FROM taggings WHERE record_type = 'activity' AND record_id = ?")
            .bind(&id)
            .execute(&mut **tx)
            .await?;
        let result = sqlx::query("DELETE FROM activities WHERE id = ?")
            .bind(&id)
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
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
