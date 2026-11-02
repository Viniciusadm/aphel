use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use aphel_kernel::application::fields::{EntityDefinition, RecordTypeRegistry};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{ArchivedFilter, QueryPlan};
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper::utc;
use aphel_kernel::infrastructure::repositories::mysql_dashboard::{
    RecentEventExclusion, aggregate_query, in_order, link_column, push_ids, push_readable,
    record_ids, record_titles, user_column, uuid_column,
};
use aphel_kernel::records::constrain_query;
use aphel_kernel::state::runtime_state;
use pipelines::domain::StageKind;
use pipelines::store::PipelineStore;

const NOTE_RECORD_TYPE: &str = "notes.note";
const PAYLOAD_EVENT_PREFIXES: [&str; 1] = ["deal."];

fn pool() -> Result<MySqlPool, AppError> {
    runtime_state()?.db()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DealTotals {
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
    pub(crate) forecast: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageAggregate {
    pub(crate) stage_id: Uuid,
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardStage {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) kind: StageKind,
    pub(crate) position: i32,
    pub(crate) probability: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardPipeline {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) stages: Vec<DashboardStage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordLink {
    pub(crate) record_type: String,
    pub(crate) record_id: Uuid,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingActivity {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) priority: String,
    pub(crate) due_at: Option<DateTime<Utc>>,
    pub(crate) owner: Option<UserReference>,
    pub(crate) record: Option<RecordLink>,
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
    pub(crate) event_types: Vec<String>,
    pub(crate) records: Vec<ReadableRecords>,
    pub(crate) note_parents: Option<Vec<ReadableRecords>>,
    pub(crate) actor_ids: Option<Vec<String>>,
    pub(crate) limit: u32,
}

fn link_with_title(
    titles: &HashMap<(String, Uuid), String>,
    link: Option<(String, Uuid)>,
) -> Option<RecordLink> {
    link.map(|(record_type, record_id)| RecordLink {
        title: titles.get(&(record_type.clone(), record_id)).cloned(),
        record_type,
        record_id,
    })
}

pub(crate) async fn deal_totals(
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<DealTotals, AppError> {
    let pool = pool()?;
    let constrained = constrain_query(&pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let (statement, arguments) = aggregate_query(
        scope,
        plan,
        "COUNT(*), SUM(r.amount_base), SUM(r.amount_base * COALESCE(r.probability, \
         (SELECT ps.probability FROM pipeline_stages ps WHERE ps.id = r.stage_id)) / 100)",
        "",
    )?;
    let (count, amount, forecast): (i64, Option<Decimal>, Option<Decimal>) =
        sqlx::query_as_with(sqlx::AssertSqlSafe(statement), arguments)
            .fetch_one(&pool)
            .await?;
    Ok(DealTotals {
        count: u64::try_from(count).unwrap_or_default(),
        amount: amount.unwrap_or_default(),
        forecast: forecast.unwrap_or_default(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaleDeal {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) amount: Option<Decimal>,
    pub(crate) currency: String,
    pub(crate) stage_name: String,
    pub(crate) owner: Option<UserReference>,
    pub(crate) last_touched_at: DateTime<Utc>,
}

pub(crate) async fn stale_deals(
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<StaleDeal>, AppError> {
    let pool = pool()?;
    let ids = record_ids(&pool, scope, plan).await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT d.id, d.title, d.amount, d.currency, s.name AS stage_name, d.owner_id, \
         u.display_name AS owner_name, \
         GREATEST(d.updated_at, COALESCE(d.last_activity_at, d.updated_at)) AS last_touched_at \
         FROM deals d JOIN pipeline_stages s ON s.id = d.stage_id \
         LEFT JOIN users u ON u.id = d.owner_id WHERE d.id",
    );
    push_ids(&mut builder, &ids);
    let rows = builder.build().fetch_all(&pool).await?;
    let mut items = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let id = uuid_column(&id, "deals.id")?;
        let touched: NaiveDateTime = row.try_get("last_touched_at")?;
        items.insert(
            id,
            StaleDeal {
                id,
                title: row.try_get("title")?,
                amount: row.try_get("amount")?,
                currency: row.try_get("currency")?,
                stage_name: row.try_get("stage_name")?,
                owner: user_column(row, "owner_id", "owner_name")?,
                last_touched_at: utc(touched),
            },
        );
    }
    Ok(in_order(&ids, items))
}

pub(crate) async fn stage_aggregates(
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<StageAggregate>, AppError> {
    let pool = pool()?;
    let constrained = constrain_query(&pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let (statement, arguments) = aggregate_query(
        scope,
        plan,
        "r.stage_id, COUNT(*), SUM(r.amount_base)",
        " GROUP BY r.stage_id",
    )?;
    let rows: Vec<(String, i64, Option<Decimal>)> =
        sqlx::query_as_with(sqlx::AssertSqlSafe(statement), arguments)
            .fetch_all(&pool)
            .await?;
    rows.into_iter()
        .map(|(stage_id, count, amount)| {
            Ok(StageAggregate {
                stage_id: uuid_column(&stage_id, "deals.stage_id")?,
                count: u64::try_from(count).unwrap_or_default(),
                amount: amount.unwrap_or_default(),
            })
        })
        .collect()
}

pub(crate) async fn pipeline(
    pipeline_id: Option<Uuid>,
) -> Result<Option<DashboardPipeline>, AppError> {
    let store = PipelineStore::new(pool()?);
    let pipeline = match pipeline_id {
        Some(id) => store.find_pipeline(id).await?,
        None => store.default_pipeline().await?,
    };
    let Some(pipeline) = pipeline else {
        return Ok(None);
    };
    let stages = store
        .list_stages(pipeline.id)
        .await?
        .into_iter()
        .filter(|stage| stage.is_active)
        .map(|stage| DashboardStage {
            id: stage.id,
            name: stage.name,
            kind: stage.kind,
            position: stage.position,
            probability: stage.probability,
        })
        .collect();
    Ok(Some(DashboardPipeline {
        id: pipeline.id,
        name: pipeline.name,
        stages,
    }))
}

pub(crate) async fn pending_activities(
    scope: &AccessScope,
    registry: &RecordTypeRegistry,
    plan: &QueryPlan,
) -> Result<Vec<PendingActivity>, AppError> {
    let pool = pool()?;
    let ids = record_ids(&pool, scope, plan).await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT a.id, a.title, a.priority, a.due_at, a.owner_id, u.display_name AS owner_name, \
         a.record_type, a.record_id FROM activities a LEFT JOIN users u ON u.id = a.owner_id \
         WHERE a.id",
    );
    push_ids(&mut builder, &ids);
    let rows = builder.build().fetch_all(&pool).await?;
    let mut links = Vec::new();
    let mut items = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let id = uuid_column(&id, "activities.id")?;
        let link = link_column(row, "record_type", "record_id")?;
        if let Some(link) = &link {
            links.push(link.clone());
        }
        let due_at: Option<NaiveDateTime> = row.try_get("due_at")?;
        items.insert(
            id,
            (
                PendingActivity {
                    id,
                    title: row.try_get("title")?,
                    priority: row.try_get("priority")?,
                    due_at: due_at.map(utc),
                    owner: user_column(row, "owner_id", "owner_name")?,
                    record: None,
                },
                link,
            ),
        );
    }
    let titles = record_titles(&pool, registry, &links).await?;
    Ok(in_order(&ids, items)
        .into_iter()
        .map(|(mut activity, link)| {
            activity.record = link_with_title(&titles, link);
            activity
        })
        .collect())
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

async fn note_parents(
    pool: &MySqlPool,
    note_ids: &[Uuid],
) -> Result<HashMap<Uuid, (String, Uuid)>, AppError> {
    if note_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut builder =
        QueryBuilder::<MySql>::new("SELECT id, record_type, record_id FROM notes WHERE id");
    push_ids(&mut builder, note_ids);
    let rows = builder.build().fetch_all(pool).await?;
    let mut parents = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        if let Some(parent) = link_column(row, "record_type", "record_id")? {
            parents.insert(uuid_column(&id, "notes.id")?, parent);
        }
    }
    Ok(parents)
}

pub(crate) async fn recent_events(
    scope: &AccessScope,
    registry: &RecordTypeRegistry,
    request: &RecentEventsRequest,
) -> Result<Vec<RecentEvent>, AppError> {
    if request.event_types.is_empty() {
        return Ok(Vec::new());
    }
    let pool = pool()?;
    let records = readable_plans(&pool, scope, &request.records).await?;
    let parent_plans = match &request.note_parents {
        Some(parents) => Some(readable_plans(&pool, scope, parents).await?),
        None => None,
    };
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT e.id, e.event_type, e.record_type, e.record_id, e.payload, e.occurred_at, \
         u.id AS actor_user_id, u.display_name AS actor_name \
         FROM domain_events e \
         LEFT JOIN users u ON e.actor_type = 'user' AND u.id = e.actor_id \
         WHERE e.event_type IN (",
    );
    let mut types = builder.separated(", ");
    for event_type in &request.event_types {
        types.push_bind(event_type.clone());
    }
    builder.push(") AND (1 = 0");
    for plan in &records {
        push_readable(&mut builder, scope, "e.record_type", "e.record_id", plan)?;
    }
    if let Some(parents) = &parent_plans {
        builder
            .push(" OR (e.record_type = ")
            .push_bind(NOTE_RECORD_TYPE)
            .push(" AND e.record_id IN (SELECT n.id FROM notes n WHERE ")
            .push("n.deleted_at IS NULL AND (n.visibility = 'team' OR n.author_id = ")
            .push_bind(scope.user_id().as_str().to_owned())
            .push(") AND (1 = 0");
        for plan in parents {
            push_readable(&mut builder, scope, "n.record_type", "n.record_id", plan)?;
        }
        builder.push(")))");
    }
    builder.push(")");
    for exclusion in aphel_kernel::modules::app().contributions::<dyn RecentEventExclusion>() {
        exclusion.exclude(&mut builder);
    }
    if let Some(actor_ids) = &request.actor_ids {
        if actor_ids.is_empty() {
            builder.push(" AND 1 = 0");
        } else {
            builder.push(" AND e.actor_type = 'user' AND e.actor_id IN (");
            let mut actors = builder.separated(", ");
            for actor_id in actor_ids {
                actors.push_bind(actor_id.clone());
            }
            builder.push(")");
        }
    }
    builder
        .push(" ORDER BY e.occurred_at DESC, e.id DESC LIMIT ")
        .push_bind(request.limit);
    let rows = builder.build().fetch_all(&pool).await?;

    let mut events = Vec::with_capacity(rows.len());
    let mut note_ids = Vec::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let event_type: String = row.try_get("event_type")?;
        let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
        let Json(payload): Json<Value> = row.try_get("payload")?;
        let link = link_column(row, "record_type", "record_id")?;
        if let Some((record_type, record_id)) = &link
            && record_type == NOTE_RECORD_TYPE
        {
            note_ids.push(*record_id);
        }
        let keeps_payload = PAYLOAD_EVENT_PREFIXES
            .iter()
            .any(|prefix| event_type.starts_with(prefix));
        events.push((
            RecentEvent {
                id: uuid_column(&id, "domain_events.id")?,
                payload: if keeps_payload { payload } else { Value::Null },
                event_type,
                occurred_at: utc(occurred_at),
                actor: user_column(row, "actor_user_id", "actor_name")?,
                record: None,
            },
            link,
        ));
    }
    let parents = note_parents(&pool, &note_ids).await?;
    let resolved: Vec<(RecentEvent, Option<(String, Uuid)>)> = events
        .into_iter()
        .map(|(event, link)| {
            let link = match link {
                Some((record_type, record_id)) if record_type == NOTE_RECORD_TYPE => {
                    parents.get(&record_id).cloned()
                }
                other => other,
            };
            (event, link)
        })
        .collect();
    let links: Vec<(String, Uuid)> = resolved
        .iter()
        .filter_map(|(_, link)| link.clone())
        .collect();
    let titles = record_titles(&pool, registry, &links).await?;
    Ok(resolved
        .into_iter()
        .map(|(mut event, link)| {
            event.record = link_with_title(&titles, link);
            event
        })
        .collect())
}
