pub(crate) mod merges;

pub(crate) use merges::MergeStore;

use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::detection::{ScanKey, ScanPass};
use crate::domain::{
    CandidateStatus, DuplicateFamily, DuplicateRule, normalize_company_name, normalize_name,
    ordered_pair,
};
use crate::{
    CandidateResolution, CandidateView, DuplicateType, NewCandidate, RecordFingerprint,
    RecordSummary, RuleSetting, ScanStatus, identifier_key, normalize_identifier, rules_json,
    split_identifier,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::LabelNormalization;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::query::compiler::{
    aliased, push_partition, push_record_scope,
};
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::repositories::mapper::{
    instant, optional_instant, optional_user, push_ids,
};

pub(crate) const MAX_PROBABLE_MATCHES: i64 = 200;

const NULL_TEXT: &str = "CAST(NULL AS CHAR)";

fn fingerprint_columns(kind: &DuplicateType) -> String {
    match kind.family {
        DuplicateFamily::Contact => "r.id, r.full_name AS name, r.email, r.phone, r.mobile, \
             r.company_id, CAST(NULL AS CHAR) AS domain"
            .to_owned(),
        DuplicateFamily::Company => "r.id, r.name AS name, r.email, r.phone, \
             CAST(NULL AS CHAR) AS mobile, CAST(NULL AS CHAR) AS company_id, r.domain"
            .to_owned(),
        DuplicateFamily::Generic => {
            let mut columns = format!(
                "r.id, {} AS name, {NULL_TEXT} AS email, {NULL_TEXT} AS phone, \
                 {NULL_TEXT} AS mobile, {NULL_TEXT} AS company_id, {NULL_TEXT} AS domain",
                aliased("r", kind.title_column()),
            );
            for (index, alias) in kind.identifiers().iter().enumerate() {
                columns.push_str(&format!(
                    ", CAST({} AS CHAR) AS ident_{index}",
                    aliased("r", &alias.column)
                ));
            }
            columns
        }
    }
}

fn push_base(builder: &mut QueryBuilder<MySql>, kind: &DuplicateType) {
    builder.push(format!(
        " FROM {} r WHERE {} IS NULL",
        kind.table(),
        aliased("r", kind.deleted_column())
    ));
    push_partition(builder, &kind.definition, "r");
}

fn fingerprint_query(kind: &DuplicateType) -> QueryBuilder<MySql> {
    let mut builder = QueryBuilder::new(format!("SELECT {}", fingerprint_columns(kind)));
    push_base(&mut builder, kind);
    builder
}

fn fingerprint_from_row(
    kind: &DuplicateType,
    row: &MySqlRow,
) -> Result<RecordFingerprint, AppError> {
    let phone: Option<String> = row.try_get("phone")?;
    let mobile: Option<String> = row.try_get("mobile")?;
    let company_id: Option<String> = row.try_get("company_id")?;
    let mut identifiers = Vec::new();
    for (index, alias) in kind.identifiers().iter().enumerate() {
        let value: Option<String> = row.try_get(format!("ident_{index}").as_str())?;
        if let Some(value) =
            value.and_then(|value| normalize_identifier(&value, alias.normalization))
        {
            identifiers.push(identifier_key(&alias.column, &value));
        }
    }
    Ok(RecordFingerprint {
        id: uuid_column(row, "id")?,
        name: row
            .try_get::<Option<String>, _>("name")?
            .unwrap_or_default(),
        email: row.try_get("email")?,
        phones: [phone, mobile].into_iter().flatten().collect(),
        company_id: company_id.and_then(|id| Uuid::parse_str(&id).ok()),
        domain: row.try_get("domain")?,
        identifiers,
    })
}

fn key_sources(kind: &DuplicateType, key: &ScanKey) -> Vec<(String, String)> {
    let present = |column: &str| {
        let column = aliased("r", column);
        format!("{column} IS NOT NULL AND TRIM({column}) <> ''")
    };
    match key {
        ScanKey::Columns(columns) => columns
            .iter()
            .map(|column| (format!("TRIM({})", aliased("r", column)), present(column)))
            .collect(),
        ScanKey::Lowercase(columns) => columns
            .iter()
            .map(|column| {
                (
                    format!("LOWER(TRIM({}))", aliased("r", column)),
                    present(column),
                )
            })
            .collect(),
        ScanKey::CompanyAndName => vec![(
            format!(
                "CONCAT(r.company_id, '|', LOWER(TRIM(COALESCE({}, ''))))",
                aliased("r", kind.title_column())
            ),
            "r.company_id IS NOT NULL".to_owned(),
        )],
        ScanKey::Title => vec![(
            format!("LOWER(TRIM({}))", aliased("r", kind.title_column())),
            present(kind.title_column()),
        )],
    }
}

fn rules_from_json(value: &Value) -> Vec<DuplicateRule> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter_map(DuplicateRule::parse)
                .collect()
        })
        .unwrap_or_default()
}

fn summary_from_row(row: &MySqlRow, prefix: &str) -> Result<RecordSummary, AppError> {
    Ok(RecordSummary {
        id: uuid_column(row, &format!("{prefix}_id"))?,
        title: row
            .try_get::<Option<String>, _>(format!("{prefix}_title").as_str())?
            .unwrap_or_default(),
        subtitle: row.try_get(format!("{prefix}_subtitle").as_str())?,
        owner_id: optional_user(row, &format!("{prefix}_owner_id"))?,
        created_at: instant(row, &format!("{prefix}_created_at"))?,
    })
}

fn candidate_from_row(row: &MySqlRow) -> Result<Option<CandidateView>, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let status: String = row.try_get("status")?;
    let (Ok(record_type), Some(status)) = (
        RecordType::parse(&record_type),
        CandidateStatus::parse(&status),
    ) else {
        return Ok(None);
    };
    let Json(rules): Json<Value> = row.try_get("rules")?;
    Ok(Some(CandidateView {
        id: uuid_column(row, "id")?,
        record_type,
        record: summary_from_row(row, "record")?,
        duplicate: summary_from_row(row, "duplicate")?,
        rules: rules_from_json(&rules),
        status,
        detected_at: instant(row, "detected_at")?,
        resolved_at: optional_instant(row, "resolved_at")?,
    }))
}

fn subtitle_sql(alias: &str, columns: &[String]) -> String {
    match columns {
        [] => NULL_TEXT.to_owned(),
        columns => format!(
            "COALESCE({})",
            columns
                .iter()
                .map(|column| format!("CAST({} AS CHAR)", aliased(alias, column)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn candidate_select(kind: &DuplicateType, subtitle_columns: &[String]) -> String {
    let side = |alias: &str, prefix: &str| {
        let owner = kind
            .definition
            .owner_column
            .as_deref()
            .map_or_else(|| NULL_TEXT.to_owned(), |column| aliased(alias, column));
        format!(
            "{alias}.id AS {prefix}_id, {title} AS {prefix}_title, {subtitle} AS {prefix}_subtitle, \
             {owner} AS {prefix}_owner_id, {alias}.created_at AS {prefix}_created_at",
            title = aliased(alias, kind.title_column()),
            subtitle = subtitle_sql(alias, subtitle_columns),
        )
    };
    format!(
        "SELECT c.id, c.record_type, c.rules, c.status, c.detected_at, c.resolved_at, {}, {} \
         FROM duplicate_candidates c \
         JOIN {table} a ON a.id = c.record_id \
         JOIN {table} b ON b.id = c.duplicate_id \
         WHERE TRUE",
        side("a", "record"),
        side("b", "duplicate"),
        table = kind.table(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingCandidate {
    pub(crate) id: Uuid,
    pub(crate) record_id: Uuid,
    pub(crate) duplicate_id: Uuid,
    pub(crate) rules: Vec<DuplicateRule>,
}

#[derive(Clone)]
pub(crate) struct DuplicateStore {
    pool: MySqlPool,
}

impl DuplicateStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    fn push_visible(
        &self,
        builder: &mut QueryBuilder<MySql>,
        scope: &AccessScope,
        kind: &DuplicateType,
        column: &str,
        visibility: &RecordScopeFilter,
        include_deleted: bool,
    ) {
        builder.push(format!(
            " AND {column} IN (SELECT r.id FROM {} r WHERE TRUE",
            kind.table()
        ));
        if !include_deleted {
            builder.push(format!(
                " AND {} IS NULL",
                aliased("r", kind.deleted_column())
            ));
        }
        push_record_scope(builder, scope, &kind.definition, visibility);
        builder.push(")");
    }

    pub(crate) async fn rule_settings(
        &self,
        record_type: &str,
    ) -> Result<Vec<RuleSetting>, AppError> {
        let rows =
            sqlx::query("SELECT rule_key, is_enabled FROM duplicate_rules WHERE record_type = ?")
                .bind(record_type)
                .fetch_all(&self.pool)
                .await?;
        let mut settings = Vec::with_capacity(rows.len());
        for row in rows {
            let key: String = row.try_get("rule_key")?;
            let enabled: bool = row.try_get("is_enabled")?;
            if let Some(rule) = DuplicateRule::parse(&key) {
                settings.push(RuleSetting { rule, enabled });
            }
        }
        Ok(settings)
    }

    pub(crate) async fn save_rule(
        &self,
        tx: &mut MySqlTx,
        record_type: &str,
        setting: &RuleSetting,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO duplicate_rules (record_type, rule_key, is_enabled) \
             VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE is_enabled = VALUES(is_enabled)",
        )
        .bind(record_type)
        .bind(setting.rule.as_str())
        .bind(setting.enabled)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn scan_status(&self) -> Result<Option<ScanStatus>, AppError> {
        let row = sqlx::query(
            "SELECT status, requested_at, started_at, finished_at, records, candidates \
             FROM duplicate_scan_status WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let requested_at: NaiveDateTime = row.try_get("requested_at")?;
            let started_at: Option<NaiveDateTime> = row.try_get("started_at")?;
            let finished_at: Option<NaiveDateTime> = row.try_get("finished_at")?;
            let records: u32 = row.try_get("records")?;
            let candidates: u32 = row.try_get("candidates")?;
            Ok(ScanStatus {
                status: row.try_get("status")?,
                requested_at: mapper::utc(requested_at),
                started_at: started_at.map(mapper::utc),
                finished_at: finished_at.map(mapper::utc),
                records: u64::from(records),
                candidates: u64::from(candidates),
            })
        })
        .transpose()
    }

    pub(crate) async fn mark_scan_queued(
        &self,
        requested_by: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO duplicate_scan_status (id, status, requested_by, requested_at) \
             VALUES (1, 'queued', ?, CURRENT_TIMESTAMP(6)) \
             ON DUPLICATE KEY UPDATE status = IF(status IN ('queued', 'running'), status, 'queued'), \
               requested_by = IF(status IN ('queued', 'running'), requested_by, VALUES(requested_by)), \
               requested_at = IF(status IN ('queued', 'running'), requested_at, CURRENT_TIMESTAMP(6))",
        )
        .bind(requested_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_scan_running(&self) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO duplicate_scan_status (id, status, started_at) \
             VALUES (1, 'running', CURRENT_TIMESTAMP(6)) \
             ON DUPLICATE KEY UPDATE status = 'running', started_at = CURRENT_TIMESTAMP(6), finished_at = NULL",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_scan_finished(
        &self,
        status: &str,
        records: u64,
        candidates: u64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE duplicate_scan_status SET status = ?, finished_at = CURRENT_TIMESTAMP(6), \
             records = ?, candidates = ? WHERE id = 1",
        )
        .bind(status)
        .bind(records.min(u64::from(u32::MAX)) as u32)
        .bind(candidates.min(u64::from(u32::MAX)) as u32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn count_records(&self, kind: &DuplicateType) -> Result<u64, AppError> {
        let mut builder = QueryBuilder::new("SELECT COUNT(*)");
        push_base(&mut builder, kind);
        let total: i64 = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(total.max(0) as u64)
    }

    pub(crate) async fn scan_page(
        &self,
        kind: &DuplicateType,
        pass: &ScanPass,
        after: Option<&(Vec<u8>, Uuid)>,
        limit: u32,
    ) -> Result<Vec<(Vec<u8>, RecordFingerprint)>, AppError> {
        let columns = fingerprint_columns(kind);
        let mut builder = QueryBuilder::<MySql>::new("SELECT * FROM (");
        for (index, (expression, condition)) in key_sources(kind, &pass.key).iter().enumerate() {
            if index > 0 {
                builder.push(" UNION ALL ");
            }
            builder.push(format!(
                "SELECT {columns}, CAST({expression} AS BINARY) AS scan_key"
            ));
            push_base(&mut builder, kind);
            builder.push(format!(" AND {condition}"));
        }
        builder.push(") s WHERE TRUE");
        if let Some((key, id)) = after {
            builder
                .push(" AND (s.scan_key > ")
                .push_bind(key.clone())
                .push(" OR (s.scan_key = ")
                .push_bind(key.clone())
                .push(" AND s.id > ")
                .push_bind(id.to_string())
                .push("))");
        }
        builder
            .push(" ORDER BY s.scan_key, s.id LIMIT ")
            .push_bind(i64::from(limit));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let key: Vec<u8> = row.try_get("scan_key")?;
                Ok((key, fingerprint_from_row(kind, row)?))
            })
            .collect()
    }

    pub(crate) async fn fingerprints_by_ids(
        &self,
        kind: &DuplicateType,
        ids: &[Uuid],
    ) -> Result<Vec<RecordFingerprint>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = fingerprint_query(kind);
        builder.push(" AND r.id");
        push_ids(&mut builder, ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| fingerprint_from_row(kind, row))
            .collect()
    }

    pub(crate) async fn probable_matches(
        &self,
        kind: &DuplicateType,
        probe: &RecordFingerprint,
        rules: &[DuplicateRule],
        visibility: Option<(&AccessScope, &RecordScopeFilter)>,
    ) -> Result<Vec<RecordFingerprint>, AppError> {
        let mut builder = fingerprint_query(kind);
        builder
            .push(" AND r.id <> ")
            .push_bind(probe.id.to_string())
            .push(" AND (1 = 0");
        let mut conditions = 0;
        for rule in rules {
            match rule {
                DuplicateRule::Email => {
                    if let Some(email) = probe.email.as_deref().filter(|email| !email.is_empty()) {
                        builder
                            .push(" OR r.email = ")
                            .push_bind(email.to_lowercase());
                        conditions += 1;
                    }
                }
                DuplicateRule::Phone => {
                    if !probe.phones.is_empty() {
                        let columns: &[&str] = match kind.family {
                            DuplicateFamily::Contact => &["r.phone", "r.mobile"],
                            _ => &["r.phone"],
                        };
                        for column in columns {
                            builder.push(format!(" OR {column} IN ("));
                            let mut separated = builder.separated(", ");
                            for phone in &probe.phones {
                                separated.push_bind(phone.clone());
                            }
                            builder.push(")");
                        }
                        conditions += 1;
                    }
                }
                DuplicateRule::NameAndCompany => {
                    if let Some(company_id) = probe.company_id {
                        builder
                            .push(" OR r.company_id = ")
                            .push_bind(company_id.to_string());
                        conditions += 1;
                    }
                }
                DuplicateRule::Domain => {
                    if let Some(domain) =
                        probe.domain.as_deref().filter(|domain| !domain.is_empty())
                    {
                        builder
                            .push(" OR r.domain = ")
                            .push_bind(domain.to_lowercase());
                        conditions += 1;
                    }
                }
                DuplicateRule::Name => {
                    let normalized = match kind.family {
                        DuplicateFamily::Company => normalize_company_name(&probe.name),
                        _ => normalize_name(&probe.name),
                    };
                    let prefix: String = normalized
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .chars()
                        .take(3)
                        .collect();
                    if prefix.chars().count() >= 2 {
                        builder
                            .push(format!(" OR {} LIKE ", aliased("r", kind.title_column())))
                            .push_bind(format!("{prefix}%"));
                        conditions += 1;
                    }
                }
                DuplicateRule::Identifier => {
                    for identifier in &probe.identifiers {
                        let Some((column, value)) = split_identifier(identifier) else {
                            continue;
                        };
                        let Some(alias) = kind
                            .identifiers()
                            .iter()
                            .find(|alias| alias.column == column)
                        else {
                            continue;
                        };
                        let column = aliased("r", &alias.column);
                        let expression = match alias.normalization {
                            LabelNormalization::Exact => format!("TRIM({column})"),
                            _ => format!("LOWER(TRIM({column}))"),
                        };
                        builder
                            .push(format!(" OR {expression} = "))
                            .push_bind(value.to_owned());
                        conditions += 1;
                    }
                }
            }
        }
        if conditions == 0 {
            return Ok(Vec::new());
        }
        builder.push(")");
        if let Some((scope, filter)) = visibility {
            push_record_scope(&mut builder, scope, &kind.definition, filter);
        }
        builder
            .push(" ORDER BY r.id LIMIT ")
            .push_bind(MAX_PROBABLE_MATCHES);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| fingerprint_from_row(kind, row))
            .collect()
    }

    pub(crate) async fn upsert_candidates(
        &self,
        tx: &mut MySqlTx,
        record_type: &str,
        candidates: &[NewCandidate],
    ) -> Result<u64, AppError> {
        if candidates.is_empty() {
            return Ok(0);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO duplicate_candidates (id, record_type, record_id, duplicate_id, \
             rules, status, detected_at) ",
        );
        builder.push_values(candidates, |mut row, candidate| {
            let (record_id, duplicate_id) =
                ordered_pair(candidate.record_id, candidate.duplicate_id);
            row.push_bind(candidate.id.to_string())
                .push_bind(record_type.to_owned())
                .push_bind(record_id.to_string())
                .push_bind(duplicate_id.to_string())
                .push_bind(Json(rules_json(&candidate.rules)))
                .push_bind(CandidateStatus::Pending.as_str())
                .push_bind(mapper::naive(candidate.detected_at));
        });
        builder
            .push(" ON DUPLICATE KEY UPDATE rules = IF(status = 'pending', VALUES(rules), rules)");
        let result = builder.build().execute(&mut **tx).await?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn list_candidates(
        &self,
        scope: &AccessScope,
        kind: &DuplicateType,
        status: CandidateStatus,
        visibility: &RecordScopeFilter,
        subtitle_columns: &[String],
        window: ListWindow,
    ) -> Result<Listing<CandidateView>, AppError> {
        let include_deleted = status == CandidateStatus::Merged;
        let push_filters = |builder: &mut QueryBuilder<MySql>| {
            builder
                .push(" AND c.record_type = ")
                .push_bind(kind.as_str().to_owned())
                .push(" AND c.status = ")
                .push_bind(status.as_str());
            for column in ["c.record_id", "c.duplicate_id"] {
                self.push_visible(builder, scope, kind, column, visibility, include_deleted);
            }
        };

        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM duplicate_candidates c WHERE TRUE");
        push_filters(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut builder = QueryBuilder::new(candidate_select(kind, subtitle_columns));
        push_filters(&mut builder);
        builder
            .push(" ORDER BY c.detected_at DESC, c.id DESC LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(candidate) = candidate_from_row(row)? {
                items.push(candidate);
            }
        }
        Ok(Listing {
            items,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn candidate_type(
        &self,
        candidate_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let record_type: Option<String> =
            sqlx::query_scalar("SELECT record_type FROM duplicate_candidates WHERE id = ?")
                .bind(candidate_id.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(record_type)
    }

    pub(crate) async fn find_candidate(
        &self,
        kind: &DuplicateType,
        candidate_id: Uuid,
        subtitle_columns: &[String],
    ) -> Result<Option<CandidateView>, AppError> {
        let mut builder = QueryBuilder::new(candidate_select(kind, subtitle_columns));
        builder
            .push(" AND c.id = ")
            .push_bind(candidate_id.to_string());
        let row = builder.build().fetch_optional(&self.pool).await?;
        match row {
            Some(row) => candidate_from_row(&row),
            None => Ok(None),
        }
    }

    pub(crate) async fn set_candidate_status(
        &self,
        tx: &mut MySqlTx,
        candidate_id: Uuid,
        status: CandidateStatus,
        resolution: &CandidateResolution,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE duplicate_candidates SET status = ?, resolved_by = ?, resolved_at = ? \
             WHERE id = ? AND status = 'pending'",
        )
        .bind(status.as_str())
        .bind(resolution.resolved_by.as_str())
        .bind(mapper::naive(resolution.resolved_at))
        .bind(candidate_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn resolve_merged(
        &self,
        tx: &mut MySqlTx,
        record_type: &str,
        merged_id: Uuid,
        survivor_id: Uuid,
        resolution: &CandidateResolution,
    ) -> Result<(), AppError> {
        let (record_id, duplicate_id) = ordered_pair(merged_id, survivor_id);
        sqlx::query(
            "UPDATE duplicate_candidates SET status = 'merged', resolved_by = ?, resolved_at = ? \
             WHERE record_type = ? AND record_id = ? AND duplicate_id = ?",
        )
        .bind(resolution.resolved_by.as_str())
        .bind(mapper::naive(resolution.resolved_at))
        .bind(record_type)
        .bind(record_id.to_string())
        .bind(duplicate_id.to_string())
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "DELETE FROM duplicate_candidates WHERE record_type = ? \
             AND status = 'pending' AND (record_id = ? OR duplicate_id = ?)",
        )
        .bind(record_type)
        .bind(merged_id.to_string())
        .bind(merged_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn visible_records(
        &self,
        scope: &AccessScope,
        kind: &DuplicateType,
        visibility: &RecordScopeFilter,
        ids: &[Uuid],
        include_deleted: bool,
    ) -> Result<Vec<Uuid>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder =
            QueryBuilder::<MySql>::new(format!("SELECT r.id FROM {} r WHERE r.id", kind.table()));
        push_ids(&mut builder, ids);
        if !include_deleted {
            builder.push(format!(
                " AND {} IS NULL",
                aliased("r", kind.deleted_column())
            ));
        }
        push_record_scope(&mut builder, scope, &kind.definition, visibility);
        let found: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(found
            .into_iter()
            .filter_map(|id| Uuid::parse_str(&id).ok())
            .collect())
    }

    pub(crate) async fn pending_page(
        &self,
        record_type: &str,
        after: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<PendingCandidate>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT id, record_id, duplicate_id, rules FROM duplicate_candidates \
             WHERE status = 'pending' AND record_type = ",
        );
        builder.push_bind(record_type.to_owned());
        if let Some(after) = after {
            builder.push(" AND id > ").push_bind(after.to_string());
        }
        builder
            .push(" ORDER BY id LIMIT ")
            .push_bind(i64::from(limit));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let Json(rules): Json<Value> = row.try_get("rules")?;
                Ok(PendingCandidate {
                    id: uuid_column(row, "id")?,
                    record_id: uuid_column(row, "record_id")?,
                    duplicate_id: uuid_column(row, "duplicate_id")?,
                    rules: rules_from_json(&rules),
                })
            })
            .collect()
    }

    pub(crate) async fn delete_pending(
        &self,
        tx: &mut MySqlTx,
        ids: &[Uuid],
    ) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "DELETE FROM duplicate_candidates WHERE status = 'pending' AND id",
        );
        push_ids(&mut builder, ids);
        Ok(builder.build().execute(&mut **tx).await?.rows_affected())
    }

    pub(crate) async fn update_pending_rules(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        rules: &[DuplicateRule],
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE duplicate_candidates SET rules = ? WHERE id = ? AND status = 'pending'",
        )
        .bind(Json(rules_json(rules)))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn forget_record(
        tx: &mut MySqlTx,
        record_type: &str,
        record_id: Uuid,
    ) -> Result<(), AppError> {
        let id = record_id.to_string();
        sqlx::query(
            "DELETE FROM duplicate_candidates WHERE record_type = ? \
             AND (record_id = ? OR duplicate_id = ?)",
        )
        .bind(record_type)
        .bind(&id)
        .bind(&id)
        .execute(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM record_merges WHERE record_type = ? AND survivor_id = ?")
            .bind(record_type)
            .bind(&id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
