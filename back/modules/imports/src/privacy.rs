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
        push_text_match(&mut builder, "CAST(r.raw AS CHAR)", matcher);
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
        if request.records.is_empty() && targets.is_empty() {
            return Ok(SourceOutcome::default());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT r.import_job_id, r.row_index FROM import_rows r JOIN import_jobs j ON j.id = r.import_job_id WHERE ",
        );
        selection(&mut builder, request.records, &targets);
        builder.push(" FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        let mut jobs = std::collections::BTreeSet::new();
        for row in &rows {
            let job: String = row.try_get("import_job_id")?;
            let index: u32 = row.try_get("row_index")?;
            sqlx::query("UPDATE import_rows SET raw = ?, errors = NULL, record_id = NULL WHERE import_job_id = ? AND row_index = ?").bind(sqlx::types::Json(json!({"redacted":true}))).bind(&job).bind(index).execute(&mut **tx).await?;
            jobs.insert(job);
        }
        for job in jobs {
            let preview: Option<sqlx::types::Json<Value>> =
                sqlx::query_scalar("SELECT preview FROM import_jobs WHERE id = ? FOR UPDATE")
                    .bind(&job)
                    .fetch_optional(&mut **tx)
                    .await?;
            if let Some(sqlx::types::Json(mut preview)) = preview
                && request.redaction.apply(&mut preview)
            {
                sqlx::query("UPDATE import_jobs SET preview = ? WHERE id = ?")
                    .bind(sqlx::types::Json(preview))
                    .bind(job)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        Ok(SourceOutcome {
            affected: rows.len() as u64,
            storage_keys: Vec::new(),
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
