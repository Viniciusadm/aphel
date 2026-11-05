use crate::{
    domain::{NoteView, note_preview},
    store::note_from_row,
};
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
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::RecordPipeline;
use aphel_sdk::privacy::sql::{push_ids, push_refs, push_text_match, redact_text, table_exists};
use aphel_sdk::privacy::{
    PrivacyItem, PrivacySource, Redaction, SourceOutcome, SourceRequest, SourceSnapshot,
    SourceTarget, SubjectMatcher,
};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;
pub(crate) struct NotesPrivacy;
fn ids(values: &[String]) -> Result<Vec<Uuid>, AppError> {
    values
        .iter()
        .map(|value| {
            Uuid::parse_str(value)
                .map_err(|_| AppError::Validation("invalid note privacy target".to_owned()))
        })
        .collect()
}
fn selection(builder: &mut QueryBuilder<MySql>, records: &[RecordRef], ids: &[Uuid]) {
    builder.push("(FALSE");
    if !ids.is_empty() {
        builder.push(" OR n.id");
        push_ids(builder, ids);
    }
    if !records.is_empty() {
        builder.push(" OR ");
        push_refs(builder, "n.record_type", "n.record_id", records);
    }
    builder.push(")");
}
fn snapshot(note: &NoteView) -> Value {
    json!({"id":note.id.to_string(),"record":{"record_type":note.record.record_type.as_str(),"id":note.record.record_id.to_string()},"body":note.body,"visibility":note.visibility.as_str(),"author":note.author.as_ref().map(|author| &author.display_name),"created_at":note.created_at,"updated_at":note.updated_at})
}
#[async_trait::async_trait]
impl PrivacySource for NotesPrivacy {
    fn key(&self) -> &'static str {
        "notes"
    }
    async fn available(&self, ctx: &Ctx<'_>) -> Result<bool, AppError> {
        table_exists(ctx, "notes").await
    }
    fn labels(&self) -> LocalizedLabels {
        LocalizedLabels(std::collections::BTreeMap::from([
            ("en".into(), "Notes".into()),
            ("pt-BR".into(), "Notas".into()),
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
            "SELECT n.*, u.display_name AS author_name FROM notes n LEFT JOIN users u ON u.id = n.author_id WHERE (",
        );
        push_text_match(&mut builder, "n.body", matcher);
        if !records.is_empty() {
            builder.push(" OR ");
            push_refs(&mut builder, "n.record_type", "n.record_id", records);
        }
        builder
            .push(") ORDER BY n.created_at DESC, n.id LIMIT ")
            .push_bind(limit);
        builder.build().fetch_all(ctx.pool()).await?.iter().map(|row| {
            let note = note_from_row(row)?;
            Ok(PrivacyItem { target: SourceTarget { source:self.key().into(), id:note.id.to_string() }, record:Some(note.record.clone()), title:note_preview(&note.body), excerpt:Some(note.body.chars().take(200).collect()), reason:if records.contains(&note.record) {"record"} else {"mention"}.into(), metadata:json!({"created_at":note.created_at,"visibility":note.visibility.as_str()}) })
        }).collect()
    }
    async fn existing(&self, ctx: &Ctx<'_>, values: &[String]) -> Result<Vec<String>, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let ids = ids(values)?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new("SELECT id FROM notes WHERE id");
        push_ids(&mut builder, &ids);
        Ok(builder.build_query_scalar().fetch_all(ctx.pool()).await?)
    }
    async fn snapshot(
        &self,
        ctx: &Ctx<'_>,
        records: &[RecordRef],
        values: &[String],
    ) -> Result<SourceSnapshot, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let ids = ids(values)?;
        if records.is_empty() && ids.is_empty() {
            return Ok(SourceSnapshot::default());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT n.*, u.display_name AS author_name FROM notes n LEFT JOIN users u ON u.id = n.author_id WHERE ",
        );
        selection(&mut builder, records, &ids);
        builder.push(" ORDER BY n.created_at, n.id");
        let rows = builder.build().fetch_all(ctx.pool()).await?;
        let data = rows
            .iter()
            .map(|row| note_from_row(row).map(|note| snapshot(&note)))
            .collect::<Result<Vec<_>, _>>()?;
        let identifiers = data
            .iter()
            .filter_map(|value| value.get("body").cloned())
            .collect();
        Ok(SourceSnapshot { data, identifiers })
    }
    async fn erase(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        request: &SourceRequest<'_>,
    ) -> Result<SourceOutcome, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let ids = ids(request.items)?;
        if request.records.is_empty() && ids.is_empty() {
            return Ok(SourceOutcome::default());
        }
        let mut builder = QueryBuilder::<MySql>::new("SELECT n.id FROM notes n WHERE ");
        selection(&mut builder, request.records, &ids);
        builder.push(" FOR UPDATE");
        let found: Vec<String> = builder.build_query_scalar().fetch_all(&mut **tx).await?;
        let mut storage_keys = Vec::new();
        for id in &found {
            sqlx::query("DELETE FROM notes WHERE id = ?")
                .bind(id)
                .execute(&mut **tx)
                .await?;
            let record = RecordRef::new(
                RecordType::from_static("notes.note"),
                Uuid::parse_str(id)
                    .map_err(|_| AppError::Internal("invalid note id".to_owned()))?,
            );
            storage_keys.extend(
                ctx.pipeline()
                    .purged(tx, ctx.scope, &record)
                    .await?
                    .orphan_storage_keys,
            );
        }
        Ok(SourceOutcome {
            affected: found.len() as u64,
            storage_keys,
        })
    }
    async fn redact(
        &self,
        _ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        redact_text(tx, "notes", "body", redaction).await
    }
    async fn restrict_inactive(
        &self,
        _ctx: &Ctx<'_>,
        builder: &mut QueryBuilder<MySql>,
        record_type: &RecordType,
        threshold: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), AppError> {
        builder
            .push(" AND NOT EXISTS(SELECT 1 FROM notes n WHERE n.record_type = ")
            .push_bind(record_type.as_str().to_owned())
            .push(" AND n.record_id = r.id AND n.updated_at >= ")
            .push_bind(aphel_sdk::platform::infrastructure::repositories::mapper::naive(threshold))
            .push(")");
        Ok(())
    }
}
