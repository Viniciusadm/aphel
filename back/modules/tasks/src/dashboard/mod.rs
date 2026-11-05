use aphel_sdk::dashboard::period_totals;
use aphel_sdk::platform::application::query::MetricSpec;
use rust_decimal::{Decimal, prelude::ToPrimitive};
mod queries;
use self::queries::{PendingActivity, RecordLink};
use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::dashboard::{
    DashboardPorts, DashboardRepository, DashboardSource, Planner, SettingsReader, WidgetContext,
    WidgetPorts, instant_value,
};
use aphel_sdk::platform::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::query::{
    Filter, FilterOperator, QueryPlan, RecordQueryPorts, SortSpec,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::domain::period::{local_today, start_of_day};
use async_trait::async_trait;
use chrono::{DateTime, Days, Utc};
use serde_json::{Map, Value, json};
const AGGREGATE_LIMIT: u64 = 1;
fn eq(field: &str, value: impl Into<Value>) -> Filter {
    Filter::condition(field, FilterOperator::Eq, value.into())
}

fn at_or_after(field: &str, instant: DateTime<Utc>) -> Filter {
    Filter::condition(field, FilterOperator::Gte, instant_value(instant))
}

fn before(field: &str, instant: DateTime<Utc>) -> Filter {
    Filter::condition(field, FilterOperator::Lt, instant_value(instant))
}

fn user_json(user: &Option<UserReference>) -> Value {
    user.as_ref()
        .map(|user| json!({ "id": user.id.as_str(), "display_name": user.display_name }))
        .unwrap_or(Value::Null)
}

fn record_json(record: &Option<RecordLink>) -> Value {
    record
        .as_ref()
        .map(|record| {
            json!({
                "record_type": record.record_type,
                "record_id": record.record_id.to_string(),
                "title": record.title,
            })
        })
        .unwrap_or(Value::Null)
}

async fn count<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    planner: &Planner,
    conditions: Vec<Filter>,
) -> Result<u64, AppError> {
    let plan = planner.plan(conditions, Vec::new(), AGGREGATE_LIMIT)?;
    context.ports.dashboard().count(context.scope, &plan).await
}

struct PendingSettings {
    only_mine: bool,
    limit: i64,
}

fn pending_settings(settings: &Value) -> Result<PendingSettings, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = PendingSettings {
        only_mine: reader.boolean("only_mine", true),
        limit: reader.integer("limit", 5, 1, 20),
    };
    let mut normalized = Map::new();
    normalized.insert(
        "stale_days".to_owned(),
        json!(reader.integer("stale_days", 14, 1, 365)),
    );
    normalized.insert("only_mine".to_owned(), json!(parsed.only_mine));
    normalized.insert("limit".to_owned(), json!(parsed.limit));
    reader.finish(normalized)?;
    Ok(parsed)
}

#[derive(Clone, Copy)]
struct ActivityFieldVisibility {
    title: bool,
    owner: bool,
    record: bool,
    due_at: bool,
    priority: bool,
}

impl ActivityFieldVisibility {
    async fn load<P: DashboardPorts>(context: &WidgetContext<'_, P>) -> Result<Self, AppError> {
        let policy = FieldAccessPolicy::load(
            context.ports.query().definitions(),
            context.scope,
            &RecordType::ACTIVITY,
        )
        .await?;
        Ok(Self {
            title: policy.key_access("title") != FieldAccess::Hidden,
            owner: policy.key_access("owner_id") != FieldAccess::Hidden,
            record: policy.key_access("record_type") != FieldAccess::Hidden
                && policy.key_access("record_id") != FieldAccess::Hidden,
            due_at: policy.key_access("due_at") != FieldAccess::Hidden,
            priority: policy.key_access("priority") != FieldAccess::Hidden,
        })
    }
}

fn activity_json(activity: &PendingActivity, visibility: ActivityFieldVisibility) -> Value {
    json!({
        "id": activity.id.to_string(),
        "title": visibility.title.then_some(activity.title.as_str()),
        "priority": visibility.priority.then_some(activity.priority.as_str()),
        "due_at": activity.due_at.filter(|_| visibility.due_at).map(instant_value),
        "owner": if visibility.owner { user_json(&activity.owner) } else { Value::Null },
        "record": if visibility.record { record_json(&activity.record) } else { Value::Null },
    })
}

fn listing_plan(
    planner: &Planner,
    conditions: Vec<Filter>,
    sort: &str,
    limit: i64,
) -> Result<QueryPlan, AppError> {
    planner.plan(
        conditions,
        vec![SortSpec::asc(sort)],
        u64::try_from(limit).unwrap_or(1),
    )
}

async fn activity_bucket<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    planner: &Planner,
    conditions: Vec<Filter>,
    limit: i64,
    visibility: ActivityFieldVisibility,
) -> Result<Value, AppError> {
    if !visibility.due_at {
        return Ok(json!({ "count": 0, "items": [] }));
    }
    let total = count(context, planner, conditions.clone()).await?;
    let items = if total == 0 {
        Vec::new()
    } else {
        let plan = listing_plan(planner, conditions, "due_at", limit)?;
        queries::pending_activities(
            context.ports.pool(),
            context.scope,
            context.ports.query(),
            &plan,
        )
        .await?
    };
    Ok(json!({
        "count": total,
        "items": items
            .iter()
            .map(|activity| activity_json(activity, visibility))
            .collect::<Vec<_>>(),
    }))
}

pub(crate) struct TaskPendingSource;
#[async_trait]
impl DashboardSource for TaskPendingSource {
    fn key(&self) -> &str {
        "tasks.pending"
    }
    fn collection(&self) -> &str {
        "pending"
    }
    fn available(&self, scope: &AccessScope) -> bool {
        scope.filter_for(&Permission::TASKS_READ).is_some()
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, WidgetPorts>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let settings = pending_settings(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let Some(tasks) = context.planner(&RecordType::ACTIVITY).await? else {
            return Err(AppError::forbidden_code(
                aphel_sdk::platform::application::access::PERMISSION_DENIED,
                "missing permission tasks.read",
            ));
        };
        let visibility = ActivityFieldVisibility::load(context).await?;
        let today = local_today(context.now, context.timezone);
        let tomorrow = today.checked_add_days(Days::new(1)).unwrap_or(today);
        let after_tomorrow = tomorrow.checked_add_days(Days::new(1)).unwrap_or(tomorrow);
        let start_tomorrow = start_of_day(context.timezone, tomorrow);
        let end_tomorrow = start_of_day(context.timezone, after_tomorrow);
        let only_mine = settings.only_mine && context.subject.is_none();
        let base = || {
            let mut conditions = vec![eq("status", "open")];
            if only_mine {
                conditions.push(eq("owner_id", "me"));
            }
            conditions
        };
        let with = |extra: Vec<Filter>| {
            let mut conditions = base();
            conditions.extend(extra);
            conditions
        };
        let overdue = activity_bucket(
            context,
            &tasks,
            with(vec![before("due_at", context.now)]),
            settings.limit,
            visibility,
        )
        .await?;
        let due_today = activity_bucket(
            context,
            &tasks,
            with(vec![
                at_or_after("due_at", context.now),
                before("due_at", start_tomorrow),
            ]),
            settings.limit,
            visibility,
        )
        .await?;
        let due_tomorrow = activity_bucket(
            context,
            &tasks,
            with(vec![
                at_or_after("due_at", start_tomorrow),
                before("due_at", end_tomorrow),
            ]),
            settings.limit,
            visibility,
        )
        .await?;

        Ok(json!({"overdue": overdue, "due_today": due_today, "due_tomorrow": due_tomorrow}))
    }
}

pub(crate) struct TaskSummarySource;
#[async_trait]
impl DashboardSource for TaskSummarySource {
    fn key(&self) -> &str {
        "tasks.summary"
    }
    fn collection(&self) -> &str {
        "summary"
    }
    fn available(&self, scope: &AccessScope) -> bool {
        scope.filter_for(&Permission::TASKS_READ).is_some()
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, WidgetPorts>,
        _settings: &Value,
    ) -> Result<Value, AppError> {
        let mut totals = Map::new();
        let mut previous = Map::new();
        match context.planner(&RecordType::ACTIVITY).await? {
            Some(planner) => {
                totals.insert(
                    "pending_tasks".to_owned(),
                    json!(count(context, &planner, vec![eq("status", "open")]).await?),
                );
                let result = period_totals(
                    context,
                    "tasks.task",
                    vec![eq("status", "done")],
                    vec![MetricSpec::count()],
                    "completed_at",
                )
                .await?;
                totals.insert(
                    "completed_activities".to_owned(),
                    json!(whole(result.total("count"))),
                );
                previous.insert(
                    "completed_activities".to_owned(),
                    json!(whole(result.previous_total("count"))),
                );
            }
            None => {
                totals.insert("pending_tasks".to_owned(), Value::Null);
                totals.insert("completed_activities".to_owned(), Value::Null);
                previous.insert("completed_activities".to_owned(), Value::Null);
            }
        }

        totals.insert(
            "previous".into(),
            if context.compare_to_previous {
                Value::Object(previous)
            } else {
                Value::Null
            },
        );
        Ok(Value::Object(totals))
    }
}

fn whole(value: Option<Decimal>) -> u64 {
    value.and_then(|value| value.to_u64()).unwrap_or_default()
}
