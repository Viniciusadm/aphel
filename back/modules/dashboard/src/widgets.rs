use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use async_trait::async_trait;
use chrono::{DateTime, Days, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value, json};

use crate::engine::{
    DashboardPorts, DashboardRepository, Planner, SettingsReader, WidgetContext, WidgetProvider,
    WidgetSize, instant_value, period_totals,
};
use crate::engine_domain::{local_today, start_of_day};
use aphel_kernel::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::query::{
    AggregateFunction, Filter, FilterOperator, MetricSpec, QueryPlan, RecordQueryPorts, SortSpec,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use currencies::domain::{BASE_AMOUNT_FIELD, BASE_AMOUNT_METRIC};
use workflow::rotting::stale_record_filter;

use crate::queries::{
    self, PendingActivity, ReadableRecords, RecentEvent, RecentEventsRequest, RecordLink, StaleDeal,
};

pub(crate) const SALES_INDICATORS: &str = "sales_indicators";
pub(crate) const PENDING_CENTER: &str = "pending_center";
pub(crate) const RECENT_ACTIVITY: &str = "recent_activity";
pub(crate) const REPORT_TOTALS: &str = "report_totals";
pub(crate) const DEALS_BY_STAGE: &str = "deals_by_stage";

pub(crate) const RECORD_EVENT_TYPES: &[(&str, &[&str])] = &[
    ("people.person", &["contact.created"]),
    ("organizations.organization", &["company.created"]),
    (
        "deals.deal",
        &[
            "deal.created",
            "deal.stage_changed",
            "deal.won",
            "deal.lost",
            "deal.reopened",
            "deal.owner_changed",
        ],
    ),
    (
        "activities.activity",
        &["activity.created", "activity.completed"],
    ),
];

pub(crate) const NOTE_EVENT_TYPES: &[&str] = &["note.created"];

const NOTE_PARENT_TYPES: [RecordType; 4] = [
    RecordType::CONTACT,
    RecordType::COMPANY,
    RecordType::DEAL,
    RecordType::ACTIVITY,
];

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

fn is_empty(field: &str) -> Filter {
    Filter::condition(field, FilterOperator::IsEmpty, Value::Null)
}

fn amount(value: Decimal) -> Value {
    json!(format!("{:.2}", value.round_dp(2)))
}

fn optional_amount(value: Option<Decimal>) -> Value {
    value.map(amount).unwrap_or(Value::Null)
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

fn whole(value: Option<Decimal>) -> u64 {
    value.and_then(|value| value.to_u64()).unwrap_or_default()
}

struct ClosedDeals {
    won_count: u64,
    won_amount: Option<Decimal>,
    lost_count: u64,
}

impl ClosedDeals {
    fn json(&self) -> Value {
        let mut object = Map::new();
        object.insert("won_count".to_owned(), json!(self.won_count));
        if let Some(won_amount) = self.won_amount {
            object.insert("won_amount".to_owned(), amount(won_amount));
        }
        object.insert("lost_count".to_owned(), json!(self.lost_count));
        object.insert(
            "conversion_rate".to_owned(),
            conversion_rate(self.won_count, self.lost_count),
        );
        Value::Object(object)
    }
}

async fn deal_amount_visible<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
) -> Result<bool, AppError> {
    let policy = FieldAccessPolicy::load(
        context.ports.query().definitions(),
        context.scope,
        &RecordType::DEAL,
    )
    .await?;
    Ok(policy.key_access("amount") != FieldAccess::Hidden)
}

async fn closed_deals<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
) -> Result<(ClosedDeals, Option<ClosedDeals>), AppError> {
    let with_amount = deal_amount_visible(context).await?;
    let mut metrics = vec![MetricSpec::count()];
    if with_amount {
        metrics.push(MetricSpec::of(AggregateFunction::Sum, BASE_AMOUNT_FIELD));
    }
    let won = period_totals(
        context,
        "deals.deal",
        vec![eq("status", "won")],
        metrics,
        "closed_at",
    )
    .await?;
    let lost = period_totals(
        context,
        "deals.deal",
        vec![eq("status", "lost")],
        vec![MetricSpec::count()],
        "closed_at",
    )
    .await?;
    let amount_of = |value: Option<Decimal>| with_amount.then(|| value.unwrap_or_default());
    let current = ClosedDeals {
        won_count: whole(won.total("count")),
        won_amount: amount_of(won.total(BASE_AMOUNT_METRIC)),
        lost_count: whole(lost.total("count")),
    };
    let previous = context.compare_to_previous.then(|| ClosedDeals {
        won_count: whole(won.previous_total("count")),
        won_amount: amount_of(won.previous_total(BASE_AMOUNT_METRIC)),
        lost_count: whole(lost.previous_total("count")),
    });
    Ok((current, previous))
}

fn conversion_rate(won: u64, lost: u64) -> Value {
    let closed = won + lost;
    if closed == 0 {
        Value::Null
    } else {
        json!(((won as f64 / closed as f64) * 10_000.0).round() / 10_000.0)
    }
}

async fn count<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    planner: &Planner,
    conditions: Vec<Filter>,
) -> Result<u64, AppError> {
    let plan = planner.plan(conditions, Vec::new(), AGGREGATE_LIMIT)?;
    context.ports.dashboard().count(context.scope, &plan).await
}

pub(crate) struct SalesIndicatorsWidget;

#[async_trait]
impl<P> WidgetProvider<P> for SalesIndicatorsWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        SALES_INDICATORS
    }

    fn title(&self) -> &str {
        "Indicadores de vendas"
    }

    fn category(&self) -> &str {
        "sales"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::DEALS_READ]
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Large
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        _settings: &Value,
    ) -> Result<Value, AppError> {
        let Some(planner) = context.planner(&RecordType::DEAL).await? else {
            return Err(AppError::forbidden_code(
                aphel_kernel::application::access::PERMISSION_DENIED,
                "missing permission deals.read",
            ));
        };
        let open_plan = planner.plan(vec![eq("status", "open")], Vec::new(), AGGREGATE_LIMIT)?;
        let open = queries::deal_totals(context.scope, &open_plan).await?;
        let (closed, previous) = closed_deals(context).await?;
        let mut data = Map::new();
        data.insert("currency".to_owned(), json!(context.currency().await?));
        data.insert("open_count".to_owned(), json!(open.count));
        data.insert("pipeline_amount".to_owned(), amount(open.amount));
        data.insert("forecast_amount".to_owned(), amount(open.forecast));
        if let Value::Object(closed) = closed.json() {
            data.extend(closed);
        }
        data.insert(
            "previous".to_owned(),
            previous
                .as_ref()
                .map(ClosedDeals::json)
                .unwrap_or(Value::Null),
        );
        Ok(Value::Object(data))
    }
}

pub(crate) struct PendingCenterWidget;

struct PendingSettings {
    stale_days: i64,
    only_mine: bool,
    limit: i64,
}

fn pending_settings(settings: &Value) -> Result<PendingSettings, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = PendingSettings {
        stale_days: reader.integer("stale_days", 14, 1, 365),
        only_mine: reader.boolean("only_mine", true),
        limit: reader.integer("limit", 5, 1, 20),
    };
    let mut normalized = Map::new();
    normalized.insert("stale_days".to_owned(), json!(parsed.stale_days));
    normalized.insert("only_mine".to_owned(), json!(parsed.only_mine));
    normalized.insert("limit".to_owned(), json!(parsed.limit));
    reader.finish(normalized)?;
    Ok(parsed)
}

#[derive(Clone, Copy)]
struct ActivityFieldVisibility {
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
            due_at: policy.key_access("due_at") != FieldAccess::Hidden,
            priority: policy.key_access("priority") != FieldAccess::Hidden,
        })
    }
}

fn activity_json(activity: &PendingActivity, visibility: ActivityFieldVisibility) -> Value {
    json!({
        "id": activity.id.to_string(),
        "title": activity.title,
        "priority": visibility.priority.then_some(activity.priority.as_str()),
        "due_at": activity.due_at.filter(|_| visibility.due_at).map(instant_value),
        "owner": user_json(&activity.owner),
        "record": record_json(&activity.record),
    })
}

fn stale_deal_json(deal: &StaleDeal) -> Value {
    json!({
        "id": deal.id.to_string(),
        "title": deal.title,
        "amount": optional_amount(deal.amount),
        "currency": deal.currency,
        "stage_name": deal.stage_name,
        "owner": user_json(&deal.owner),
        "last_touched_at": instant_value(deal.last_touched_at),
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
    let total = count(context, planner, conditions.clone()).await?;
    let items = if total == 0 {
        Vec::new()
    } else {
        let plan = listing_plan(planner, conditions, "due_at", limit)?;
        queries::pending_activities(context.scope, context.ports.query().registry(), &plan).await?
    };
    Ok(json!({
        "count": total,
        "items": items
            .iter()
            .map(|activity| activity_json(activity, visibility))
            .collect::<Vec<_>>(),
    }))
}

#[async_trait]
impl<P> WidgetProvider<P> for PendingCenterWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        PENDING_CENTER
    }

    fn title(&self) -> &str {
        "Central de pendências"
    }

    fn category(&self) -> &str {
        "productivity"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::ACTIVITIES_READ]
    }

    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({ "stale_days": 14, "only_mine": true, "limit": 5 })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        let parsed = pending_settings(settings)?;
        Ok(json!({
            "stale_days": parsed.stale_days,
            "only_mine": parsed.only_mine,
            "limit": parsed.limit,
        }))
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let settings = pending_settings(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let Some(activities) = context.planner(&RecordType::ACTIVITY).await? else {
            return Err(AppError::forbidden_code(
                aphel_kernel::application::access::PERMISSION_DENIED,
                "missing permission activities.read",
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
            &activities,
            with(vec![before("due_at", context.now)]),
            settings.limit,
            visibility,
        )
        .await?;
        let due_today = activity_bucket(
            context,
            &activities,
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
            &activities,
            with(vec![
                at_or_after("due_at", start_tomorrow),
                before("due_at", end_tomorrow),
            ]),
            settings.limit,
            visibility,
        )
        .await?;

        let stale = match context.planner(&RecordType::DEAL).await? {
            None => Value::Null,
            Some(deals) => {
                let cutoff = context.now - chrono::Duration::days(settings.stale_days);
                let mut conditions = vec![
                    eq("status", "open"),
                    stale_record_filter(vec![
                        before("updated_at", cutoff),
                        Filter::or(vec![
                            is_empty("last_activity_at"),
                            before("last_activity_at", cutoff),
                        ]),
                    ]),
                ];
                if only_mine {
                    conditions.push(eq("owner_id", "me"));
                }
                let total = count(context, &deals, conditions.clone()).await?;
                let items = if total == 0 {
                    Vec::new()
                } else {
                    let plan = listing_plan(&deals, conditions, "updated_at", settings.limit)?;
                    queries::stale_deals(context.scope, &plan).await?
                };
                json!({
                    "count": total,
                    "stale_days": settings.stale_days,
                    "items": items.iter().map(stale_deal_json).collect::<Vec<_>>(),
                })
            }
        };
        Ok(json!({
            "overdue": overdue,
            "due_today": due_today,
            "due_tomorrow": due_tomorrow,
            "stale_deals": stale,
        }))
    }
}

pub(crate) struct RecentActivityWidget;

fn recent_limit(settings: &Value) -> Result<i64, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let limit = reader.integer("limit", 15, 5, 50);
    let mut normalized = Map::new();
    normalized.insert("limit".to_owned(), json!(limit));
    reader.finish(normalized)?;
    Ok(limit)
}

fn event_json(event: &RecentEvent) -> Value {
    json!({
        "id": event.id.to_string(),
        "event_type": event.event_type,
        "occurred_at": instant_value(event.occurred_at),
        "actor": user_json(&event.actor),
        "record": record_json(&event.record),
        "payload": event.payload,
    })
}

pub(crate) fn recent_events_request<P: RecordQueryPorts>(
    query: &P,
    scope: &aphel_kernel::domain::access::AccessScope,
    limit: u32,
) -> RecentEventsRequest {
    let readable = |record_type: &RecordType| {
        let definition = query.registry().get(record_type)?;
        let visibility = scope.filter_for(&definition.read_permission)?;
        Some(ReadableRecords {
            definition: definition.clone(),
            visibility,
        })
    };
    let mut event_types = Vec::new();
    let mut records = Vec::new();
    for (record_type, types) in RECORD_EVENT_TYPES {
        let Ok(record_type) = RecordType::parse(*record_type) else {
            continue;
        };
        if let Some(readable) = readable(&record_type) {
            records.push(readable);
            event_types.extend(types.iter().map(|event| (*event).to_owned()));
        }
    }
    let note_parents = if scope.filter_for(&Permission::NOTES_READ).is_some() {
        let parents: Vec<ReadableRecords> = NOTE_PARENT_TYPES.iter().filter_map(readable).collect();
        if parents.is_empty() {
            None
        } else {
            event_types.extend(NOTE_EVENT_TYPES.iter().map(|event| (*event).to_owned()));
            Some(parents)
        }
    } else {
        None
    };
    RecentEventsRequest {
        event_types,
        records,
        note_parents,
        actor_ids: None,
        limit,
    }
}

#[async_trait]
impl<P> WidgetProvider<P> for RecentActivityWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        RECENT_ACTIVITY
    }

    fn title(&self) -> &str {
        "Atividades recentes"
    }

    fn category(&self) -> &str {
        "activities.activity"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![
            Permission::CONTACTS_READ,
            Permission::COMPANIES_READ,
            Permission::DEALS_READ,
            Permission::ACTIVITIES_READ,
        ]
    }

    fn is_available(&self, scope: &aphel_kernel::domain::access::AccessScope) -> bool {
        <Self as WidgetProvider<P>>::required_permissions(self)
            .iter()
            .any(|permission| scope.scope_for(permission).is_some())
    }

    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({ "limit": 15 })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        Ok(json!({ "limit": recent_limit(settings)? }))
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let limit = recent_limit(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let mut request = recent_events_request(
            context.ports.query(),
            context.scope,
            u32::try_from(limit).unwrap_or(15),
        );
        request.actor_ids = context.subject.as_ref().map(|subject| {
            subject
                .member_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect()
        });
        let events = if request.records.is_empty() && request.note_parents.is_none() {
            Vec::new()
        } else {
            queries::recent_events(context.scope, context.ports.query().registry(), &request)
                .await?
        };
        Ok(json!({ "items": events.iter().map(event_json).collect::<Vec<_>>() }))
    }
}

pub(crate) struct ReportTotalsWidget;

#[async_trait]
impl<P> WidgetProvider<P> for ReportTotalsWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        REPORT_TOTALS
    }

    fn title(&self) -> &str {
        "Relatório resumido"
    }

    fn category(&self) -> &str {
        "reports"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![
            Permission::CONTACTS_READ,
            Permission::DEALS_READ,
            Permission::ACTIVITIES_READ,
        ]
    }

    fn is_available(&self, scope: &aphel_kernel::domain::access::AccessScope) -> bool {
        <Self as WidgetProvider<P>>::required_permissions(self)
            .iter()
            .any(|permission| scope.scope_for(permission).is_some())
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Large
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        _settings: &Value,
    ) -> Result<Value, AppError> {
        let mut totals = Map::new();
        let people = match context.planner(&RecordType::CONTACT).await? {
            Some(planner) => json!(count(context, &planner, Vec::new()).await?),
            None => Value::Null,
        };
        totals.insert("contacts_total".to_owned(), people);
        let mut previous = Map::new();
        match context.planner(&RecordType::DEAL).await? {
            Some(planner) => {
                totals.insert(
                    "open_deals".to_owned(),
                    json!(count(context, &planner, vec![eq("status", "open")]).await?),
                );
                for (key, status) in [("won_deals", "won"), ("lost_deals", "lost")] {
                    let result = period_totals(
                        context,
                        "deals.deal",
                        vec![eq("status", status)],
                        vec![MetricSpec::count()],
                        "closed_at",
                    )
                    .await?;
                    totals.insert(key.to_owned(), json!(whole(result.total("count"))));
                    previous.insert(key.to_owned(), json!(whole(result.previous_total("count"))));
                }
            }
            None => {
                for key in ["open_deals", "won_deals", "lost_deals"] {
                    totals.insert(key.to_owned(), Value::Null);
                }
                previous.insert("won_deals".to_owned(), Value::Null);
                previous.insert("lost_deals".to_owned(), Value::Null);
            }
        }
        match context.planner(&RecordType::ACTIVITY).await? {
            Some(planner) => {
                totals.insert(
                    "pending_tasks".to_owned(),
                    json!(count(context, &planner, vec![eq("status", "open")]).await?),
                );
                let result = period_totals(
                    context,
                    "activities.activity",
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
            "previous".to_owned(),
            if context.compare_to_previous {
                Value::Object(previous)
            } else {
                Value::Null
            },
        );
        Ok(Value::Object(totals))
    }
}

pub(crate) struct DealsByStageWidget;

fn pipeline_setting(settings: &Value) -> Result<Option<uuid::Uuid>, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let pipeline_id = reader.uuid("pipeline_id");
    let mut normalized = Map::new();
    normalized.insert(
        "pipeline_id".to_owned(),
        pipeline_id
            .map(|id| Value::String(id.to_string()))
            .unwrap_or(Value::Null),
    );
    reader.finish(normalized)?;
    Ok(pipeline_id)
}

#[async_trait]
impl<P> WidgetProvider<P> for DealsByStageWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        DEALS_BY_STAGE
    }

    fn title(&self) -> &str {
        "Oportunidades por etapa"
    }

    fn category(&self) -> &str {
        "reports"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::DEALS_READ]
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Large
    }

    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({ "pipeline_id": null })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        let pipeline_id = pipeline_setting(settings)?;
        Ok(json!({ "pipeline_id": pipeline_id.map(|id| id.to_string()) }))
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let pipeline_id = pipeline_setting(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let Some(planner) = context.planner(&RecordType::DEAL).await? else {
            return Err(AppError::forbidden_code(
                aphel_kernel::application::access::PERMISSION_DENIED,
                "missing permission deals.read",
            ));
        };
        let currency = context.currency().await?;
        let Some(pipeline) = queries::pipeline(pipeline_id).await? else {
            return Ok(json!({ "pipeline": null, "currency": currency, "stages": [] }));
        };
        let plan = planner.plan(
            vec![eq("pipeline_id", pipeline.id.to_string())],
            Vec::new(),
            AGGREGATE_LIMIT,
        )?;
        let aggregates = queries::stage_aggregates(context.scope, &plan).await?;
        let stages: Vec<Value> = pipeline
            .stages
            .iter()
            .map(|stage| {
                let aggregate = aggregates.iter().find(|total| total.stage_id == stage.id);
                json!({
                    "id": stage.id.to_string(),
                    "name": stage.name,
                    "kind": stage.kind.as_str(),
                    "probability": stage.probability,
                    "count": aggregate.map(|total| total.count).unwrap_or_default(),
                    "amount": amount(aggregate.map(|total| total.amount).unwrap_or_default()),
                })
            })
            .collect();
        Ok(json!({
            "pipeline": { "id": pipeline.id.to_string(), "name": pipeline.name },
            "currency": currency,
            "stages": stages,
        }))
    }
}
