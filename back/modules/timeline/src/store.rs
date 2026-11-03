use aphel_kernel::{
    Ctx,
    application::{files::UserReference, shared::RecordRef},
    domain::UserId,
    errors::AppError,
    infrastructure::repositories::mapper,
};
use aphel_sdk::timeline::{
    KindDefinition, LinkedRecord, TimelineEntry, TimelineEventExtension, TimelineEventScope,
    TimelineRequest, TimelineSource, push_cursor,
};
use chrono::NaiveDateTime;
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder, Row};
pub(crate) struct Events;
fn matches(builder: &mut QueryBuilder<MySql>, kind: &KindDefinition) {
    builder.push("(1 = 0");
    for event in &kind.event_types {
        builder.push(" OR e.event_type = ").push_bind(event.clone());
    }
    for prefix in &kind.event_prefixes {
        builder
            .push(" OR LEFT(e.event_type, ")
            .push_bind(prefix.len() as u32)
            .push(") = ")
            .push_bind(prefix.clone());
    }
    builder.push(")");
}
#[async_trait::async_trait]
impl TimelineSource for Events {
    fn key(&self) -> &'static str {
        "event"
    }
    fn kinds(&self) -> Vec<KindDefinition> {
        vec![
            KindDefinition::new("attachment", "Files", "Arquivos").events(&[
                aphel_kernel::application::files::ATTACHMENT_ADDED,
                aphel_kernel::application::files::ATTACHMENT_REMOVED,
            ]),
            KindDefinition::new("record_change", "Changes", "Alterações"),
            KindDefinition::new("event", "Other events", "Outros eventos"),
        ]
    }
    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        let kinds = crate::domain::kinds(ctx)?;
        if !request.kinds.iter().any(|key| {
            kinds.iter().any(|kind| {
                &kind.key == key
                    && (key == "event"
                        || !kind.event_types.is_empty()
                        || !kind.event_prefixes.is_empty())
            })
        }) {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT e.id, e.event_type, e.record_type, e.record_id, e.payload, e.occurred_at, e.actor_type, e.actor_id, u.display_name AS actor_name FROM domain_events e LEFT JOIN users u ON e.actor_type = 'user' AND u.id = e.actor_id WHERE ((e.record_type = ",
        );
        builder
            .push_bind(request.subject.record_type.as_str().to_owned())
            .push(" AND e.record_id = ")
            .push_bind(request.subject.record_id.to_string())
            .push(")");
        for scope in ctx.app().contributions::<dyn TimelineEventScope>() {
            scope.extend(ctx, &mut builder, &request.subject).await?;
        }
        builder.push(")");
        for prefix in crate::domain::hidden_prefixes(ctx) {
            builder
                .push(" AND LEFT(e.event_type, ")
                .push_bind(prefix.len() as u32)
                .push(") <> ")
                .push_bind(prefix);
        }
        for (_, extension) in ctx.app().all_contributions::<dyn TimelineEventExtension>() {
            extension.restrict(&mut builder, ctx.scope, &request.subject, request);
        }
        builder.push(" AND (1 = 0");
        for kind in kinds.iter().filter(|kind| request.wants(&kind.key)) {
            if kind.key == "event" {
                builder.push(" OR NOT (1 = 0");
                for known in kinds.iter().filter(|known| known.key != "event") {
                    builder.push(" OR ");
                    matches(&mut builder, known);
                }
                builder.push(")");
            } else if !kind.event_types.is_empty() || !kind.event_prefixes.is_empty() {
                builder.push(" OR ");
                matches(&mut builder, kind);
            }
        }
        builder.push(")");
        push_cursor(
            &mut builder,
            "e.occurred_at",
            "e.id",
            "event:",
            request.cursor.as_ref(),
        );
        builder
            .push(" ORDER BY e.occurred_at DESC, e.id DESC LIMIT ")
            .push_bind(request.limit);
        let rows = builder.build().fetch_all(ctx.pool()).await?;
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let event_type: String = row.try_get("event_type")?;
            let id: String = row.try_get("id")?;
            let record_type: String = row.try_get("record_type")?;
            let record_id: String = row.try_get("record_id")?;
            let reference = RecordRef::parse(&record_type, &record_id)?;
            let actor_type: String = row.try_get("actor_type")?;
            let actor_id: Option<String> = row.try_get("actor_id")?;
            let actor_name: Option<String> = row.try_get("actor_name")?;
            let actor = if actor_type == "user" {
                actor_id
                    .as_ref()
                    .zip(actor_name)
                    .map(|(id, display_name)| {
                        Ok::<UserReference, AppError>(UserReference {
                            id: UserId::new(id.clone())?,
                            display_name,
                        })
                    })
                    .transpose()?
            } else {
                None
            };
            let sqlx::types::Json(payload): sqlx::types::Json<Value> = row.try_get("payload")?;
            let record = if reference == request.subject {
                None
            } else {
                let title = match ctx
                    .load_record(reference.record_type.as_str(), reference.record_id)
                    .await?
                {
                    Some(value) => ctx
                        .registry()
                        .require(reference.record_type.as_str())?
                        .title_column()
                        .and_then(|key| value.get(key))
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    None => None,
                };
                Some(LinkedRecord {
                    record: reference,
                    title,
                })
            };
            entries.push(TimelineEntry { id: format!("event:{id}"), kind: crate::domain::event_kind(&event_type, &kinds), occurred_at: mapper::utc(row.try_get::<NaiveDateTime, _>("occurred_at")?), actor, title: event_type.clone(), body: None, record,
                metadata: json!({"event_id":id, "event_type":event_type, "record_type":record_type, "record_id":record_id, "payload":payload, "service_actor":{"kind":actor_type,"id":actor_id}}) });
        }
        for (_, extension) in ctx.app().all_contributions::<dyn TimelineEventExtension>() {
            extension
                .enrich(ctx.pool(), ctx.scope, &mut entries)
                .await?;
        }
        Ok(entries)
    }
}
