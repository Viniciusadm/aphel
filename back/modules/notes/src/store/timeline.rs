use crate::{domain::note_preview, store::note_from_row};
use aphel_contract_crm::CrmPermissions;
use aphel_kernel::{
    Ctx,
    domain::access::{AccessScope, Permission},
    errors::AppError,
};
use aphel_sdk::platform::api::dto::common::timestamp as instant;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::database::Db;
use aphel_sdk::timeline::{
    KindDefinition, TimelineEntry, TimelineRequest, TimelineSource, push_cursor,
};
use async_trait::async_trait;
use serde_json::json;
use sqlx::{MySql, QueryBuilder};
pub(crate) struct NotesTimeline;
struct SourceStore {
    pool: Db,
}
impl SourceStore {
    async fn load(
        &self,
        scope: &AccessScope,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        let subject = &request.subject;
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT n.id, n.author_id, au.display_name AS author_name, n.record_type, n.record_id, \
             n.body, n.visibility, n.pinned, n.created_at, n.updated_at \
             FROM notes n LEFT JOIN users au ON au.id = n.author_id WHERE ",
        );
        builder
            .push("n.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND n.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push(" AND n.deleted_at IS NULL AND (n.visibility = 'team' OR n.author_id = ")
            .push_bind(scope.user_id().as_str().to_owned())
            .push(")");
        push_cursor(
            &mut builder,
            "n.created_at",
            "n.id",
            "note:",
            request.cursor.as_ref(),
        );
        builder
            .push(" ORDER BY n.created_at DESC, n.id DESC LIMIT ")
            .push_bind(request.limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let note = note_from_row(row)?;
                Ok(TimelineEntry {
                    id: format!("note:{}", note.id),
                    kind: "note".to_owned(),
                    occurred_at: note.created_at,
                    actor: note.author.clone(),
                    title: note_preview(&note.body),
                    metadata: json!({
                        "note_id": note.id.to_string(),
                        "visibility": note.visibility.as_str(),
                        "pinned": note.pinned,
                        "author_id": note.author_id().map(|author| author.as_str().to_owned()),
                        "updated_at": instant(note.updated_at),
                    }),
                    body: Some(note.body),
                    record: None,
                })
            })
            .collect()
    }
}
#[async_trait]
impl TimelineSource for NotesTimeline {
    fn key(&self) -> &'static str {
        "note"
    }
    fn kinds(&self) -> Vec<KindDefinition> {
        vec![KindDefinition::new("note", "Notes", "Notas").hide(&["note."])]
    }
    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        if !request.wants("note") || ctx.scope.require(&Permission::NOTES_READ).is_err() {
            return Ok(Vec::new());
        }
        SourceStore {
            pool: ctx.pool().clone(),
        }
        .load(ctx.scope, request)
        .await
    }
}
