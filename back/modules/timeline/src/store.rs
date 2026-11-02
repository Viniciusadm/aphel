use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::NaiveDateTime;
use serde_json::{Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::domain::{
    TimelineCursor, TimelineEntry, TimelineEventExtension, TimelineKind, TimelineRequest,
    known_event_types, timeline_kind_for_event,
};
use activities::domain::{ActivityView, LinkedRecord, instant};
use activities::store::{activity_from_row, activity_select, linked_record, record_title_sql};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, CompareOp, FieldTarget, PlanCondition, PlanNode, PlanValue, Predicate,
    QueryPlan,
};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::{push_record_scope, push_where, quoted};
use aphel_kernel::infrastructure::repositories::mapper;
use aphel_kernel::infrastructure::repositories::mapper::user_reference;
use aphel_kernel::records::constrain_query;
use notes::domain::note_preview;
use notes::store::note_from_row;

#[derive(Clone)]
pub(crate) struct TimelineStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

fn push_cursor(
    builder: &mut QueryBuilder<MySql>,
    instant_sql: &str,
    id_sql: &str,
    prefix: &str,
    cursor: Option<&TimelineCursor>,
) {
    let Some(cursor) = cursor else {
        return;
    };
    let at = mapper::naive(cursor.occurred_at);
    builder
        .push(format!(" AND ({instant_sql} < "))
        .push_bind(at)
        .push(format!(" OR ({instant_sql} = "))
        .push_bind(at)
        .push(format!(" AND CONCAT('{prefix}', {id_sql}) < "))
        .push_bind(cursor.item_id.clone())
        .push("))");
}

fn push_list(builder: &mut QueryBuilder<MySql>, values: impl IntoIterator<Item = &'static str>) {
    builder.push("(");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value);
    }
    builder.push(")");
}

fn user_json(entry: &Option<aphel_kernel::application::files::UserReference>) -> Value {
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
        kind: TimelineKind::Activity,
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

fn event_entry(row: &MySqlRow, subject: &RecordRef) -> Result<Option<TimelineEntry>, AppError> {
    let event_type: String = row.try_get("event_type")?;
    let Some(kind) = timeline_kind_for_event(&event_type) else {
        return Ok(None);
    };
    let id: String = row.try_get("id")?;
    let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
    let Json(payload): Json<Value> = row.try_get("payload")?;
    let record: Option<LinkedRecord> =
        linked_record(row, "record_type", "record_id", "record_title")?;
    let automation = match (
        row.try_get::<Option<String>, _>("actor_automation_id")?,
        row.try_get::<Option<String>, _>("actor_automation_name")?,
    ) {
        (Some(id), Some(name)) => json!({ "id": id, "name": name }),
        _ => Value::Null,
    };
    Ok(Some(TimelineEntry {
        id: format!("event:{id}"),
        kind,
        occurred_at: mapper::utc(occurred_at),
        actor: user_reference(row, "actor_user_id", "actor_name")?,
        title: event_type.clone(),
        body: None,
        metadata: json!({
            "event_id": id,
            "event_type": event_type,
            "record_type": record.as_ref().map(|linked| linked.record.record_type.as_str().to_owned()),
            "record_id": record.as_ref().map(|linked| linked.record.record_id.to_string()),
            "payload": payload,
            "automation": automation,
        }),
        record: record.filter(|linked| &linked.record != subject),
    }))
}

const CUSTOM_RECORD_CHANGE_SQL: &str = "e.event_type LIKE 'co!_%.updated' ESCAPE '!' AND e.record_type = SUBSTRING_INDEX(e.event_type, '.', 1)";

impl TimelineStore {
    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    fn definition(
        &self,
        record_type: &RecordType,
    ) -> Result<&aphel_kernel::application::fields::RecordTypeDefinition, AppError> {
        self.registry.get(record_type).ok_or_else(|| {
            AppError::Configuration(format!("record type {record_type} is not registered"))
        })
    }

    async fn activities(
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
            .push(") OR r.id IN (SELECT l.activity_id FROM activity_links l WHERE ")
            .push("l.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND l.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push("))");
        push_record_scope(
            &mut builder,
            scope,
            self.definition(&RecordType::ACTIVITY)?,
            visibility,
        );
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

    async fn notes(
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
                    kind: TimelineKind::Note,
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

    async fn related_plan(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        company_id: &str,
        visibility: &RecordScopeFilter,
    ) -> Result<QueryPlan, AppError> {
        let plan = QueryPlan {
            record_type: self.definition(record_type)?.clone(),
            visibility: visibility.clone(),
            filter: Some(PlanNode::Condition(PlanCondition {
                target: FieldTarget::System {
                    column: "company_id".to_owned(),
                },
                field_type: FieldType::Record,
                predicate: Predicate::Compare(
                    CompareOp::Eq,
                    PlanValue::Text(company_id.to_owned()),
                ),
                negated: false,
            })),
            search: None,
            sort: Vec::new(),
            archived: ArchivedFilter::Include,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        };
        Ok(constrain_query(&self.pool, scope, &plan)
            .await?
            .unwrap_or(plan))
    }

    fn push_related(
        builder: &mut QueryBuilder<MySql>,
        scope: &AccessScope,
        plan: &QueryPlan,
    ) -> Result<(), AppError> {
        builder
            .push(" OR (e.record_type = ")
            .push_bind(plan.record_type.record_type.as_str().to_owned())
            .push(format!(
                " AND e.record_id IN (SELECT r.id FROM {} r",
                quoted(&plan.record_type.table)
            ));
        push_where(builder, scope, plan)?;
        builder.push("))");
        Ok(())
    }

    async fn events(
        &self,
        scope: &AccessScope,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        let subject = &request.subject;
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT e.id, e.event_type, e.record_type, e.record_id, e.payload, e.occurred_at, \
             u.id AS actor_user_id, u.display_name AS actor_name, \
             au.id AS actor_automation_id, au.name AS actor_automation_name, {} AS record_title \
             FROM domain_events e \
             LEFT JOIN users u ON e.actor_type = 'user' AND u.id = e.actor_id \
             LEFT JOIN automations au ON e.actor_type = 'automation' AND au.id = e.actor_id \
             WHERE ",
            record_title_sql("e")
        ));
        builder
            .push("((e.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND e.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push(")");
        builder
            .push(" OR (e.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND e.record_id IN (SELECT m.merged_id FROM record_merges m WHERE ")
            .push("m.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND m.survivor_id = ")
            .push_bind(subject.record_id.to_string())
            .push("))");
        let company_id = subject.record_id.to_string();
        if let Some(visibility) = &request.related_contacts {
            let plan = self
                .related_plan(scope, &RecordType::CONTACT, &company_id, visibility)
                .await?;
            Self::push_related(&mut builder, scope, &plan)?;
        }
        if let Some(visibility) = &request.related_deals {
            let plan = self
                .related_plan(scope, &RecordType::DEAL, &company_id, visibility)
                .await?;
            Self::push_related(&mut builder, scope, &plan)?;
        }
        builder.push(") AND e.event_type NOT LIKE 'note.%' AND e.event_type NOT LIKE 'activity.%'");
        let extensions =
            aphel_kernel::modules::app().all_contributions::<dyn TimelineEventExtension>();
        for (_, extension) in &extensions {
            extension.restrict(&mut builder, scope, subject, request);
        }

        let event_kinds: Vec<TimelineKind> = request
            .kinds
            .iter()
            .copied()
            .filter(TimelineKind::is_event)
            .collect();
        let all_event_kinds = TimelineKind::ALL
            .iter()
            .filter(|kind| kind.is_event())
            .count();
        if event_kinds.len() < all_event_kinds {
            builder.push(" AND (1 = 0");
            for kind in event_kinds {
                if kind == TimelineKind::Event {
                    builder.push(" OR (NOT (");
                    builder.push(CUSTOM_RECORD_CHANGE_SQL);
                    builder.push(") AND e.event_type NOT IN ");
                    push_list(&mut builder, known_event_types());
                    builder.push(")");
                } else {
                    builder.push(" OR e.event_type IN ");
                    push_list(&mut builder, kind.event_types().iter().copied());
                    if kind == TimelineKind::RecordChange {
                        builder.push(" OR (");
                        builder.push(CUSTOM_RECORD_CHANGE_SQL);
                        builder.push(")");
                    }
                }
            }
            builder.push(")");
        }
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
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut entries = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(entry) = event_entry(row, subject)? {
                entries.push(entry);
            }
        }
        for (_, extension) in &extensions {
            extension.enrich(&self.pool, scope, &mut entries).await?;
        }
        Ok(entries)
    }
}

impl TimelineStore {
    pub(crate) async fn entries(
        &self,
        scope: &AccessScope,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        let mut entries = Vec::new();
        if request.wants(TimelineKind::Activity)
            && let Some(visibility) = &request.activities
        {
            entries.extend(self.activities(scope, request, visibility).await?);
        }
        if request.wants(TimelineKind::Note) && request.notes {
            entries.extend(self.notes(scope, request).await?);
        }
        if request.wants_events() {
            entries.extend(self.events(scope, request).await?);
        }
        Ok(entries)
    }
}
