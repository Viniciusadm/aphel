use crate::domain::{ActivityView, instant};
use crate::store::{activity_from_row, activity_select};
use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::{
    Ctx,
    application::{
        fields::{RecordTypeRegistry, permissions::FieldAccessPolicy},
        query::RecordQueryPorts,
        shared::{RecordRef, RecordType},
    },
    domain::access::{AccessScope, Permission, RecordScopeFilter},
    errors::AppError,
    infrastructure::{query::compiler::push_record_scope, repositories::mapper},
};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::timeline::{
    KindDefinition, TimelineEntry, TimelineRequest, TimelineSource, push_cursor,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use std::sync::Arc;
fn user_json(entry: &Option<aphel_sdk::platform::application::files::UserReference>) -> Value {
    entry
        .as_ref()
        .map(|user| json!({ "id": user.id.as_str(), "display_name": user.display_name }))
        .unwrap_or(Value::Null)
}

fn activity_entry(
    activity: ActivityView,
    occurred_at: NaiveDateTime,
    subject: &RecordRef,
) -> TimelineEntry {
    let record = activity
        .record
        .clone()
        .filter(|linked| &linked.record != subject);
    TimelineEntry {
        id: format!("activity:{}", activity.id),
        kind: "activity".to_owned(),
        occurred_at: mapper::utc(occurred_at),
        actor: activity
            .created_by
            .clone()
            .or_else(|| activity.owner.clone()),
        title: activity.title.clone(),
        body: activity.description.clone(),
        record,
        metadata: json!({
            "activity_id": activity.id.to_string(),
            "status": activity.status.as_str(),
            "priority": activity.priority.as_str(),
            "due_at": activity.due_at.map(instant),
            "completed_at": activity.completed_at.map(instant),
            "duration_minutes": activity.duration_minutes,
            "activity_type": {
                "id": activity.activity_type.id.to_string(),
                "key": activity.activity_type.key,
                "name": activity.activity_type.name,
                "icon": activity.activity_type.icon,
                "color": activity.activity_type.color,
            },
            "owner": user_json(&activity.owner),
            "location": activity.location.as_ref().map(|location| json!({
                "latitude": location.latitude.to_string(),
                "longitude": location.longitude.to_string(),
                "accuracy_m": location.accuracy_m,
                "captured_at": instant(location.captured_at),
                "map_url": location.map_url(),
            })),
        }),
    }
}

pub(crate) struct TasksTimeline;
struct SourceStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}
impl SourceStore {
    async fn load(
        &self,
        scope: &AccessScope,
        request: &TimelineRequest,
        visibility: &RecordScopeFilter,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        let subject = &request.subject;
        let mut builder = QueryBuilder::<MySql>::new(activity_select("r"));
        builder
            .push("WHERE ")
            .push("r.deleted_at IS NULL AND ((r.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push(") OR r.id IN (SELECT l.activity_id FROM task_links l WHERE ")
            .push("l.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND l.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push("))");
        push_record_scope(
            &mut builder,
            scope,
            self.registry.require(RecordType::ACTIVITY.as_str())?,
            visibility,
        );
        let plan = aphel_sdk::platform::application::query::QueryPlan {
            record_type: self
                .registry
                .require(RecordType::ACTIVITY.as_str())?
                .clone(),
            visibility: visibility.clone(),
            filter: None,
            search: None,
            sort: Vec::new(),
            archived: aphel_sdk::platform::application::query::ArchivedFilter::Include,
            window: aphel_sdk::platform::application::organization_admin::ListWindow {
                offset: 0,
                limit: 1,
            },
        };
        if let Some(constrained) =
            aphel_sdk::platform::records::constrain_query(&self.pool, scope, &plan).await?
        {
            builder.push(" AND r.id IN (SELECT r.id FROM tasks r");
            aphel_sdk::platform::infrastructure::query::compiler::push_where(
                &mut builder,
                scope,
                &constrained,
            )?;
            builder.push(")");
        }
        push_cursor(
            &mut builder,
            "COALESCE(r.completed_at, r.created_at)",
            "r.id",
            "activity:",
            request.cursor.as_ref(),
        );
        builder
            .push(" ORDER BY occurred_at DESC, r.id DESC LIMIT ")
            .push_bind(request.limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
                Ok(activity_entry(
                    activity_from_row(row)?,
                    occurred_at,
                    subject,
                ))
            })
            .collect()
    }
}
#[async_trait]
impl TimelineSource for TasksTimeline {
    fn key(&self) -> &'static str {
        "activity"
    }
    fn kinds(&self) -> Vec<KindDefinition> {
        vec![KindDefinition::new("activity", "Tasks", "Tarefas").hide(&["activity."])]
    }
    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        if !request.wants("activity") {
            return Ok(Vec::new());
        }
        let Ok(visibility) = ctx.scope.record_filter(&Permission::TASKS_READ) else {
            return Ok(Vec::new());
        };
        let mut entries = SourceStore {
            pool: ctx.pool().clone(),
            registry: ctx.registry().clone(),
        }
        .load(ctx.scope, request, &visibility)
        .await?;
        crate::store::hydrate_link_titles(
            ctx,
            entries
                .iter_mut()
                .filter_map(|entry| entry.record.as_mut())
                .collect(),
        )
        .await?;
        attach_contributions(ctx, &mut entries).await?;
        let policy =
            FieldAccessPolicy::load(ctx.query().definitions(), ctx.scope, &RecordType::ACTIVITY)
                .await?;
        let hidden = policy.hidden_keys();
        let checker = aphel_sdk::platform::infrastructure::records::MySqlRecordAccessChecker::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
        );
        use aphel_sdk::platform::application::files::RecordAccessChecker;
        for entry in &mut entries {
            if let Some(linked) = &entry.record
                && (!aphel_sdk::records::enabled(ctx, &linked.record.record_type)
                    || !checker.can_read(ctx.scope, &linked.record).await?)
            {
                entry.record = None;
            }
            if hidden.contains("record_id") || hidden.contains("record_type") {
                entry.record = None;
            }
            if hidden.contains("description") {
                entry.body = None;
            }
            if hidden.contains("title") {
                entry.title.clear();
            }
            if let Some(metadata) = entry.metadata.as_object_mut() {
                for key in &hidden {
                    metadata.remove(key);
                }
                if hidden.contains("owner_id") {
                    metadata.remove("owner");
                }
                if hidden.contains("activity_type_id") {
                    metadata.remove("activity_type");
                }
                if hidden.contains("location_latitude") || hidden.contains("location_longitude") {
                    metadata.remove("location");
                }
                if let Some(location) = metadata.get_mut("location").and_then(Value::as_object_mut)
                {
                    for (field, key) in [
                        ("location_accuracy_m", "accuracy_m"),
                        ("location_captured_at", "captured_at"),
                    ] {
                        if hidden.contains(field) {
                            location.remove(key);
                        }
                    }
                }
            }
        }
        Ok(entries)
    }
}

use aphel_sdk::platform::records::serialize_records;
use serde_json::Map;
use std::collections::HashMap;
async fn attach_contributions(
    ctx: &Ctx<'_>,
    entries: &mut [TimelineEntry],
) -> Result<(), AppError> {
    let ids: Vec<Value> = entries
        .iter()
        .filter(|entry| entry.kind == "activity")
        .filter_map(|entry| entry.metadata.get("activity_id").cloned())
        .map(|id| json!({ "id": id }))
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let mut contributions: HashMap<String, Map<String, Value>> =
        serialize_records(ctx.pool(), ctx.scope, &RecordType::ACTIVITY, ids)
            .await?
            .into_iter()
            .filter_map(|item| match item {
                Value::Object(mut object) => {
                    let id = object.remove("id")?.as_str()?.to_owned();
                    Some((id, object))
                }
                _ => None,
            })
            .collect();
    for entry in entries.iter_mut().filter(|entry| entry.kind == "activity") {
        let Some(extra) = entry
            .metadata
            .get("activity_id")
            .and_then(Value::as_str)
            .and_then(|id| contributions.remove(id))
        else {
            continue;
        };
        if let Some(metadata) = entry.metadata.as_object_mut() {
            for (key, value) in extra {
                metadata.entry(key).or_insert(value);
            }
        }
    }
    Ok(())
}
