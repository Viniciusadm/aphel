use aphel_kernel::application::access::Authorizer;
use aphel_kernel::{
    Ctx,
    application::{
        fields::LocalizedLabels,
        shared::{RecordRef, RecordType},
    },
    database::transaction::MySqlTx,
    domain::access::Permission,
    errors::AppError,
};
use aphel_sdk::privacy::sql::{push_ids, push_refs, push_text_match, redact_text, table_exists};
use aphel_sdk::privacy::{
    PrivacyItem, PrivacySource, Redaction, SourceOutcome, SourceRequest, SourceSnapshot,
    SourceTarget, SubjectMatcher,
};
use serde_json::json;
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;
pub(crate) struct TasksPrivacy;
fn selection(builder: &mut QueryBuilder<MySql>, records: &[RecordRef]) {
    builder.push("(FALSE");
    if !records.is_empty() {
        builder.push(" OR ");
        push_refs(builder, "a.record_type", "a.record_id", records);
        builder.push(" OR a.id IN (SELECT l.activity_id FROM task_links l WHERE ");
        push_refs(builder, "l.record_type", "l.record_id", records);
        builder.push(")");
    }
    builder.push(")");
}
#[async_trait::async_trait]
impl PrivacySource for TasksPrivacy {
    fn key(&self) -> &'static str {
        "tasks"
    }
    fn entity_type(&self) -> Option<RecordType> {
        Some(RecordType::from_static("tasks.task"))
    }
    async fn available(&self, ctx: &Ctx<'_>) -> Result<bool, AppError> {
        table_exists(ctx, "tasks").await
    }
    fn labels(&self) -> LocalizedLabels {
        LocalizedLabels(std::collections::BTreeMap::from([
            ("en".into(), "Tasks".into()),
            ("pt-BR".into(), "Tarefas".into()),
        ]))
    }
    async fn search(
        &self,
        ctx: &Ctx<'_>,
        matcher: &SubjectMatcher,
        records: &[RecordRef],
        limit: u32,
    ) -> Result<Vec<PrivacyItem>, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT a.id, a.title, a.description, a.status, a.due_at FROM tasks a WHERE (",
        );
        push_text_match(&mut builder, "a.title", matcher);
        builder.push(" OR ");
        push_text_match(&mut builder, "COALESCE(a.description, '')", matcher);
        builder.push(" OR ");
        selection(&mut builder, records);
        builder
            .push(") ORDER BY a.created_at DESC, a.id LIMIT ")
            .push_bind(limit);
        builder.build().fetch_all(ctx.pool()).await?.iter().map(|row|{
            let id:String=row.try_get("id")?;let due:Option<chrono::NaiveDateTime>=row.try_get("due_at")?;
            Ok(PrivacyItem{target:SourceTarget{source:self.key().into(),id:id.clone()},record:Some(RecordRef::parse("tasks.task",&id)?),title:row.try_get("title")?,excerpt:row.try_get::<Option<String>,_>("description")?.map(|text|text.chars().take(200).collect()),reason:"mention".into(),metadata:json!({"status":row.try_get::<String,_>("status")?,"due_at":due.map(|value|value.and_utc())})})
        }).collect()
    }
    async fn existing(&self, ctx: &Ctx<'_>, ids: &[String]) -> Result<Vec<String>, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let ids = ids
            .iter()
            .map(|id| {
                Uuid::parse_str(id)
                    .map_err(|_| AppError::Validation("invalid task privacy target".to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new("SELECT id FROM tasks WHERE id");
        push_ids(&mut builder, &ids);
        Ok(builder.build_query_scalar().fetch_all(ctx.pool()).await?)
    }
    async fn snapshot(
        &self,
        ctx: &Ctx<'_>,
        records: &[RecordRef],
        _ids: &[String],
    ) -> Result<SourceSnapshot, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        if records.is_empty() {
            return Ok(SourceSnapshot::default());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT a.id, a.title, a.description, a.status, a.priority, a.due_at, a.completed_at, a.created_at, a.record_type, a.record_id, a.location_latitude, a.location_longitude, a.location_accuracy_m, a.location_captured_at, t.name AS type_name FROM tasks a LEFT JOIN task_types t ON t.id = a.activity_type_id WHERE ",
        );
        selection(&mut builder, records);
        builder.push(" ORDER BY a.created_at, a.id");
        let mut data = Vec::new();
        let mut identifiers = Vec::new();
        for row in builder.build().fetch_all(ctx.pool()).await? {
            let title: String = row.try_get("title")?;
            let description: Option<String> = row.try_get("description")?;
            identifiers.push(json!({"title":title,"description":description}));
            let instant =
                |key: &str| -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
                    Ok(row
                        .try_get::<Option<chrono::NaiveDateTime>, _>(key)?
                        .map(|value| value.and_utc()))
                };
            data.push(json!({"id":row.try_get::<String,_>("id")?,"title":title,"description":description,"activity_type":row.try_get::<Option<String>,_>("type_name")?,"status":row.try_get::<String,_>("status")?,"priority":row.try_get::<String,_>("priority")?,"record_type":row.try_get::<Option<String>,_>("record_type")?,"record_id":row.try_get::<Option<String>,_>("record_id")?,"due_at":instant("due_at")?,"completed_at":instant("completed_at")?,"created_at":instant("created_at")?,"location":{"latitude":row.try_get::<Option<rust_decimal::Decimal>,_>("location_latitude")?.map(|value|value.to_string()),"longitude":row.try_get::<Option<rust_decimal::Decimal>,_>("location_longitude")?.map(|value|value.to_string()),"accuracy_m":row.try_get::<Option<u32>,_>("location_accuracy_m")?,"captured_at":instant("location_captured_at")?}}));
        }
        Ok(SourceSnapshot { data, identifiers })
    }
    async fn erase(
        &self,
        _ctx: &Ctx<'_>,
        _tx: &mut MySqlTx,
        request: &SourceRequest<'_>,
    ) -> Result<SourceOutcome, AppError> {
        if !request.items.is_empty() {
            return Err(AppError::Validation(
                "task privacy targets must be canonical records".to_owned(),
            ));
        }
        Ok(SourceOutcome::default())
    }
    async fn redact(
        &self,
        _ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        Ok(redact_text(tx, "tasks", "title", redaction).await?
            + redact_text(tx, "tasks", "description", redaction).await?)
    }
    async fn restrict_inactive(
        &self,
        _ctx: &Ctx<'_>,
        builder: &mut QueryBuilder<MySql>,
        record_type: &RecordType,
        threshold: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), AppError> {
        let at = aphel_kernel::infrastructure::repositories::mapper::naive(threshold);
        builder.push(" AND NOT EXISTS(SELECT 1 FROM tasks a WHERE a.record_type = ").push_bind(record_type.as_str().to_owned()).push(" AND a.record_id = r.id AND a.updated_at >= ").push_bind(at).push(") AND NOT EXISTS(SELECT 1 FROM task_links l JOIN tasks a ON a.id = l.activity_id WHERE l.record_type = ").push_bind(record_type.as_str().to_owned()).push(" AND l.record_id = r.id AND a.updated_at >= ").push_bind(at).push(")");
        Ok(())
    }
}
