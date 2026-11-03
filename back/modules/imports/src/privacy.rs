use aphel_kernel::application::access::Authorizer;
use aphel_kernel::{
    Ctx,
    application::{fields::LocalizedLabels, shared::RecordRef},
    database::transaction::MySqlTx,
    domain::access::Permission,
    errors::AppError,
};
use aphel_sdk::privacy::sql::{push_refs, push_text_match, table_exists};
use aphel_sdk::privacy::{
    PrivacyItem, PrivacySource, Redaction, SourceOutcome, SourceRequest, SourceSnapshot,
    SourceTarget, SubjectMatcher,
};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;
pub(crate) struct ImportPrivacy;
fn targets(ids: &[String]) -> Result<Vec<(Uuid, u32)>, AppError> {
    ids.iter()
        .map(|id| {
            let (job, row) = id
                .split_once(':')
                .ok_or_else(|| AppError::Validation("invalid import privacy target".to_owned()))?;
            Ok((
                Uuid::parse_str(job).map_err(|_| {
                    AppError::Validation("invalid import privacy target".to_owned())
                })?,
                row.parse().map_err(|_| {
                    AppError::Validation("invalid import privacy target".to_owned())
                })?,
            ))
        })
        .collect()
}
fn selection(builder: &mut QueryBuilder<MySql>, records: &[RecordRef], ids: &[(Uuid, u32)]) {
    builder.push("(FALSE");
    for (job, row) in ids {
        builder
            .push(" OR (r.import_job_id = ")
            .push_bind(job.to_string())
            .push(" AND r.row_index = ")
            .push_bind(*row)
            .push(")");
    }
    if !records.is_empty() {
        builder.push(" OR ");
        push_refs(builder, "j.record_type", "r.record_id", records);
    }
    builder.push(")");
}
#[async_trait::async_trait]
impl PrivacySource for ImportPrivacy {
    fn key(&self) -> &'static str {
        "imports"
    }
    async fn available(&self, ctx: &Ctx<'_>) -> Result<bool, AppError> {
        table_exists(ctx, "import_rows").await
    }
    fn labels(&self) -> LocalizedLabels {
        LocalizedLabels(std::collections::BTreeMap::from([
            ("en".into(), "Import rows".into()),
            ("pt-BR".into(), "Linhas de importação".into()),
        ]))
    }
    async fn search(
        &self,
        ctx: &Ctx<'_>,
        matcher: &SubjectMatcher,
        _records: &[RecordRef],
        limit: u32,
    ) -> Result<Vec<PrivacyItem>, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT r.import_job_id, r.row_index, r.record_id, r.status, j.record_type, j.file_name, j.created_at FROM import_rows r JOIN import_jobs j ON j.id = r.import_job_id WHERE ",
        );
        push_text_match(
            &mut builder,
            "CONCAT_WS(' ', CAST(r.raw AS CHAR),j.file_name,CAST(j.headers AS CHAR),CAST(r.errors AS CHAR))",
            matcher,
        );
        builder
            .push(" ORDER BY j.created_at DESC, r.import_job_id, r.row_index LIMIT ")
            .push_bind(limit);
        builder.build().fetch_all(ctx.pool()).await?.iter().map(|row| {
            let job:String=row.try_get("import_job_id")?; let index:u32=row.try_get("row_index")?; let kind:String=row.try_get("record_type")?; let id:Option<String>=row.try_get("record_id")?;
            let file:Option<String>=row.try_get("file_name")?;
            let record=id.as_deref().map(|id|RecordRef::parse(&kind,id)).transpose()?;
            let created:chrono::NaiveDateTime=row.try_get("created_at")?;
            Ok(PrivacyItem { target:SourceTarget{source:self.key().into(),id:format!("{job}:{index}")},record,title:format!("{}:{index}",file.as_deref().unwrap_or(&job)),excerpt:None,reason:"mention".into(),metadata:json!({"status":row.try_get::<String,_>("status")?,"created_at":created.and_utc()}) })
        }).collect()
    }
    async fn existing(&self, ctx: &Ctx<'_>, ids: &[String]) -> Result<Vec<String>, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let targets = targets(ids)?;
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT r.import_job_id, r.row_index FROM import_rows r JOIN import_jobs j ON j.id = r.import_job_id WHERE ",
        );
        selection(&mut builder, &[], &targets);
        builder
            .build()
            .fetch_all(ctx.pool())
            .await?
            .iter()
            .map(|row| {
                Ok(format!(
                    "{}:{}",
                    row.try_get::<String, _>("import_job_id")?,
                    row.try_get::<u32, _>("row_index")?
                ))
            })
            .collect()
    }
    async fn snapshot(
        &self,
        ctx: &Ctx<'_>,
        records: &[RecordRef],
        ids: &[String],
    ) -> Result<SourceSnapshot, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let targets = targets(ids)?;
        if records.is_empty() && targets.is_empty() {
            return Ok(SourceSnapshot::default());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT r.import_job_id, r.row_index, r.record_id, r.status, r.raw, r.errors, j.record_type FROM import_rows r JOIN import_jobs j ON j.id = r.import_job_id WHERE ",
        );
        selection(&mut builder, records, &targets);
        builder.push(" ORDER BY r.import_job_id, r.row_index");
        let mut data = Vec::new();
        let mut identifiers = Vec::new();
        for row in builder.build().fetch_all(ctx.pool()).await? {
            let sqlx::types::Json(raw): sqlx::types::Json<Value> = row.try_get("raw")?;
            identifiers.push(raw.clone());
            data.push(json!({"import_job_id":row.try_get::<String,_>("import_job_id")?,"row_index":row.try_get::<u32,_>("row_index")?,"status":row.try_get::<String,_>("status")?,"record_type":row.try_get::<String,_>("record_type")?,"record_id":row.try_get::<Option<String>,_>("record_id")?,"raw":raw,"errors":row.try_get::<Option<sqlx::types::Json<Value>>,_>("errors")?.map(|value|value.0)}));
        }
        Ok(SourceSnapshot { data, identifiers })
    }
    async fn erase(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        request: &SourceRequest<'_>,
    ) -> Result<SourceOutcome, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let targets = targets(request.items)?;
        let mentions = request.redaction.mentions_only();
        if request.records.is_empty() && targets.is_empty() && mentions.is_empty() {
            return Ok(SourceOutcome::default());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT r.import_job_id, r.row_index, r.record_id, j.record_type FROM import_rows r JOIN import_jobs j ON j.id = r.import_job_id WHERE (",
        );
        selection(&mut builder, request.records, &targets);
        for term in mentions.values() {
            builder.push(" OR LOWER(CONCAT_WS(' ',CAST(r.raw AS CHAR),CAST(r.errors AS CHAR),j.file_name,CAST(j.headers AS CHAR),CAST(j.preview AS CHAR))) LIKE ")
                .push_bind(aphel_sdk::privacy::sql::contains_pattern(term));
        }
        builder.push(") ORDER BY r.import_job_id,r.row_index");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        let mut jobs = std::collections::BTreeSet::new();
        for row in &rows {
            jobs.insert(row.try_get::<String, _>("import_job_id")?);
        }
        let mut file_ids = Vec::new();
        for job in &jobs {
            let row = sqlx::query("SELECT status, file_id, error_file_id, headers FROM import_jobs WHERE id = ? FOR UPDATE").bind(job).fetch_one(&mut **tx).await?;
            let status: String = row.try_get("status")?;
            if matches!(status.as_str(), "queued" | "running") {
                return Err(AppError::conflict_code(
                    "privacy_import_busy",
                    "an affected import is still queued or running",
                ));
            }
            for column in ["file_id", "error_file_id"] {
                if let Some(id) = row.try_get::<Option<String>, _>(column)? {
                    file_ids.push(Uuid::parse_str(&id).map_err(|_| {
                        AppError::Internal("invalid stored import file identity".into())
                    })?);
                }
            }
            let sqlx::types::Json(headers): sqlx::types::Json<Vec<String>> =
                row.try_get("headers")?;
            let headers: Vec<_> = (1..=headers.len()).map(|index| index.to_string()).collect();
            sqlx::query("UPDATE import_jobs SET file_id = NULL, error_file_id = NULL, file_name = 'redacted-import.csv', preview = JSON_ARRAY(), headers = ?, privacy_redacted_at = ?, updated_at = ?, failure_reason = 'privacy_redacted', status = IF(status = 'draft','failed',status), finished_at = COALESCE(finished_at,?) WHERE id = ?")
                .bind(sqlx::types::Json(headers)).bind(request.at.naive_utc()).bind(request.at.naive_utc()).bind(request.at.naive_utc()).bind(job).execute(&mut **tx).await?;
        }
        let mut affected = 0;
        for row in &rows {
            let job: String = row.try_get("import_job_id")?;
            let index: u32 = row.try_get("row_index")?;
            let kind: String = row.try_get("record_type")?;
            let record_id: Option<String> = row.try_get("record_id")?;
            let selected = targets
                .iter()
                .any(|(id, number)| id.to_string() == job && *number == index)
                || request.records.iter().any(|record| {
                    record.record_type.as_str() == kind
                        && record_id.as_deref() == Some(record.record_id.to_string().as_str())
                });
            if selected {
                sqlx::query("UPDATE import_rows SET raw = JSON_ARRAY(), errors = NULL, record_id = NULL, status = 'skipped', action = NULL WHERE import_job_id = ? AND row_index = ?").bind(&job).bind(index).execute(&mut **tx).await?;
                affected += 1;
            } else {
                let row = sqlx::query("SELECT raw,errors FROM import_rows WHERE import_job_id = ? AND row_index = ? FOR UPDATE").bind(&job).bind(index).fetch_one(&mut **tx).await?;
                let sqlx::types::Json(mut raw): sqlx::types::Json<Value> = row.try_get("raw")?;
                let errors: Option<sqlx::types::Json<Value>> = row.try_get("errors")?;
                let mut errors = errors.map(|value| value.0).unwrap_or(Value::Null);
                let changed = mentions.apply(&mut raw) | mentions.apply(&mut errors);
                if changed {
                    sqlx::query("UPDATE import_rows SET raw = ?, errors = ? WHERE import_job_id = ? AND row_index = ?").bind(sqlx::types::Json(raw)).bind((!errors.is_null()).then_some(sqlx::types::Json(errors))).bind(&job).bind(index).execute(&mut **tx).await?;
                    affected += 1;
                }
            }
        }
        for job in jobs {
            let rows = sqlx::query("SELECT status,action,COUNT(*) total FROM import_rows WHERE import_job_id = ? GROUP BY status,action").bind(&job).fetch_all(&mut **tx).await?;
            let mut totals = crate::ImportTotals::default();
            for row in rows {
                let count: i64 = row.try_get("total")?;
                let count = u64::try_from(count)
                    .map_err(|_| AppError::Internal("invalid import row count".into()))?;
                totals.total += count;
                let status: String = row.try_get("status")?;
                let action: Option<String> = row.try_get("action")?;
                match (status.as_str(), action.as_deref()) {
                    ("pending", _) => totals.pending += count,
                    ("imported", Some("updated")) => totals.updated += count,
                    ("imported", _) => totals.imported += count,
                    ("skipped", _) => totals.skipped += count,
                    ("failed", _) => totals.failed += count,
                    _ => return Err(AppError::Internal("invalid stored import row state".into())),
                }
            }
            sqlx::query("UPDATE import_jobs SET totals = ? WHERE id = ?")
                .bind(sqlx::types::Json(totals))
                .bind(job)
                .execute(&mut **tx)
                .await?;
        }
        file_ids.sort();
        file_ids.dedup();
        let storage_keys = aphel_sdk::files::delete_unreferenced_files(tx, &file_ids).await?;
        Ok(SourceOutcome {
            affected,
            storage_keys,
        })
    }
    async fn redact(
        &self,
        _ctx: &Ctx<'_>,
        _tx: &mut MySqlTx,
        _redaction: &Redaction,
    ) -> Result<u64, AppError> {
        Ok(0)
    }
}
