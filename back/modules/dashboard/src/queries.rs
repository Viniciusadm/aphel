use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::repository::{RecentEventExclusion, link_column, user_column, uuid_column};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::EntityDefinition;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{ArchivedFilter, QueryPlan};
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::repositories::mapper::utc;
use aphel_sdk::platform::records::constrain_query;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordLink {
    pub(crate) record_type: String,
    pub(crate) record_id: Uuid,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecentEvent {
    pub(crate) id: Uuid,
    pub(crate) event_type: String,
    pub(crate) occurred_at: DateTime<Utc>,
    pub(crate) actor: Option<UserReference>,
    pub(crate) record: Option<RecordLink>,
    pub(crate) payload: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReadableRecords {
    pub(crate) definition: EntityDefinition,
    pub(crate) visibility: RecordScopeFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecentEventsRequest {
    pub(crate) records: Vec<ReadableRecords>,
    pub(crate) actor_ids: Option<Vec<String>>,
    pub(crate) limit: u32,
    pub(crate) since: Option<DateTime<Utc>>,
    pub(crate) record_types: Option<Vec<String>>,
    pub(crate) before: Option<EventCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventCursor {
    pub(crate) occurred_at: DateTime<Utc>,
    pub(crate) id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecentEventsPage {
    pub(crate) events: Vec<RecentEvent>,
    pub(crate) next: Option<EventCursor>,
    pub(crate) exhausted: bool,
}

async fn readable_plans(
    pool: &MySqlPool,
    scope: &AccessScope,
    readable: &[ReadableRecords],
) -> Result<Vec<QueryPlan>, AppError> {
    let mut plans = Vec::with_capacity(readable.len());
    for records in readable {
        let plan = QueryPlan {
            record_type: records.definition.clone(),
            visibility: records.visibility.clone(),
            filter: None,
            search: None,
            sort: Vec::new(),
            archived: ArchivedFilter::Include,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        };
        plans.push(constrain_query(pool, scope, &plan).await?.unwrap_or(plan));
    }
    Ok(plans)
}

pub(crate) async fn recent_events<P: aphel_sdk::platform::application::query::RecordQueryPorts>(
    pool: &MySqlPool,
    scope: &AccessScope,
    query: &P,
    request: &RecentEventsRequest,
) -> Result<Vec<RecentEvent>, AppError> {
    Ok(recent_events_page(pool, scope, query, request).await?.events)
}

pub(crate) async fn recent_events_page<
    P: aphel_sdk::platform::application::query::RecordQueryPorts,
>(
    pool: &MySqlPool,
    scope: &AccessScope,
    query: &P,
    request: &RecentEventsRequest,
) -> Result<RecentEventsPage, AppError> {
    let plans = readable_plans(pool, scope, &request.records).await?;
    let sources = aphel_sdk::platform::modules::app()
        .contributions::<dyn aphel_sdk::dashboard::DashboardEventScope>();
    let mut source_types = std::collections::HashSet::new();
    for source in &sources {
        if !source_types.insert(source.record_type()) {
            return Err(AppError::Configuration(
                "duplicate dashboard event scope".into(),
            ));
        }
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT e.id, e.event_type, e.record_type, e.record_id, e.payload, e.occurred_at, u.id AS actor_user_id, u.display_name AS actor_name FROM domain_events e LEFT JOIN users u ON e.actor_type = 'user' AND u.id = e.actor_id WHERE (FALSE",
    );
    for plan in &plans {
        if source_types.contains(&plan.record_type.record_type) {
            continue;
        }
        aphel_sdk::platform::infrastructure::query::compiler::push_reference_subquery(
            &mut builder,
            scope,
            plan,
            "e",
            "record_type",
            "record_id",
        )?;
    }
    for source in &sources {
        source.restrict(pool, scope, &plans, &mut builder).await?;
    }
    builder.push(")");
    for exclusion in aphel_sdk::platform::modules::app().contributions::<dyn RecentEventExclusion>()
    {
        exclusion.exclude(&mut builder);
    }
    if let Some(actors) = &request.actor_ids {
        if actors.is_empty() {
            builder.push(" AND FALSE");
        } else {
            builder.push(" AND e.actor_type = 'user' AND e.actor_id IN (");
            let mut values = builder.separated(", ");
            for actor in actors {
                values.push_bind(actor.clone());
            }
            builder.push(")");
        }
    }
    if let Some(since) = request.since {
        builder
            .push(" AND e.occurred_at >= ")
            .push_bind(aphel_sdk::platform::infrastructure::repositories::mapper::naive(since));
    }
    if let Some(types) = &request.record_types {
        builder.push(" AND e.record_type IN (");
        let mut values = builder.separated(", ");
        for record_type in types {
            values.push_bind(record_type.clone());
        }
        builder.push(")");
    }
    if let Some(cursor) = &request.before {
        let at = aphel_sdk::platform::infrastructure::repositories::mapper::naive(cursor.occurred_at);
        builder
            .push(" AND (e.occurred_at < ")
            .push_bind(at)
            .push(" OR (e.occurred_at = ")
            .push_bind(at)
            .push(" AND e.id < ")
            .push_bind(cursor.id.clone())
            .push("))");
    }
    let limit = request.limit.min(100);
    builder
        .push(" ORDER BY e.occurred_at DESC, e.id DESC LIMIT ")
        .push_bind(limit);
    let rows = builder.build().fetch_all(pool).await?;
    let exhausted = rows.len() < limit as usize;
    let next = match rows.last() {
        Some(row) => Some(EventCursor {
            occurred_at: utc(row.try_get::<NaiveDateTime, _>("occurred_at")?),
            id: row.try_get("id")?,
        }),
        None => None,
    };
    let mut raw = Vec::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let event_type: String = row.try_get("event_type")?;
        let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
        let Json(mut payload): Json<Value> = row.try_get("payload")?;
        let link = link_column(row, "record_type", "record_id")?;
        if let Some((record_type, _)) = &link {
            if query.registry().require(record_type).is_ok() {
                let policy =
                    aphel_sdk::platform::application::fields::permissions::FieldAccessPolicy::load(
                        query.definitions(),
                        scope,
                        &aphel_sdk::platform::application::shared::RecordType::parse(record_type)?,
                    )
                    .await?;
                policy.mask_event_payload(&event_type, &mut payload);
            } else {
                payload = Value::Null;
            }
        } else {
            payload = Value::Null;
        }
        raw.push((
            RecentEvent {
                id: uuid_column(&id, "domain_events.id")?,
                event_type,
                occurred_at: utc(occurred_at),
                actor: user_column(row, "actor_user_id", "actor_name")?,
                record: None,
                payload,
            },
            link,
        ));
    }
    let mut mapped = std::collections::HashMap::new();
    for source in sources {
        let kind = source.record_type();
        let ids: Vec<_> = raw
            .iter()
            .filter_map(|(_, link)| {
                link.as_ref()
                    .filter(|(key, _)| key == kind.as_str())
                    .map(|(_, id)| *id)
            })
            .collect();
        for (id, target) in source.targets(pool, &ids).await? {
            if !ids.contains(&id) {
                return Err(AppError::Configuration(
                    "dashboard event scope returned an unrequested identity".into(),
                ));
            }
            mapped.insert(
                (kind.as_str().to_owned(), id),
                (target.record_type.as_str().to_owned(), target.record_id),
            );
        }
    }
    let mut events = Vec::new();
    for (mut event, link) in raw {
        let link = link.and_then(|link| {
            if source_types.iter().any(|kind| kind.as_str() == link.0) {
                mapped.get(&link).cloned()
            } else {
                Some(link)
            }
        });
        if let Some((record_type, id)) = link {
            let record = aphel_sdk::platform::application::shared::RecordRef::parse(
                &record_type,
                &id.to_string(),
            )?;
            if let Some(label) = aphel_sdk::records::visible_record_label(
                pool,
                &std::sync::Arc::new(query.registry().clone()),
                query.definitions(),
                scope,
                &record,
            )
            .await?
            {
                event.record = Some(RecordLink {
                    record_type,
                    record_id: id,
                    title: label.title,
                });
            }
        }
        events.push(event);
    }
    Ok(RecentEventsPage {
        events,
        next,
        exhausted,
    })
}
