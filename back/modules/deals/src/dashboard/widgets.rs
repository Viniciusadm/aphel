use super::queries;
use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::query::{
    AggregateFunction, Filter, FilterOperator, MetricSpec, RecordQueryPorts,
};
use aphel_kernel::application::query::{QueryPlan, SortSpec};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::dashboard::{
    DashboardPorts, DashboardRepository, Planner, SettingsReader, WidgetContext, WidgetProvider,
    WidgetSize, instant_value, period_totals,
};
use aphel_sdk::dashboard::{DashboardSource, WidgetPorts};
use aphel_sdk::money::{BASE_AMOUNT_FIELD, BASE_AMOUNT_METRIC};
use aphel_sdk::workflow::rotting::stale_record_filter;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde_json::{Map, Value, json};
const SALES_INDICATORS: &str = "sales_indicators";
const DEALS_BY_STAGE: &str = "deals_by_stage";
const AGGREGATE_LIMIT: u64 = 1;
fn eq(field: &str, value: impl Into<Value>) -> Filter {
    Filter::condition(field, FilterOperator::Eq, value.into())
}

fn amount(value: Decimal) -> Value {
    json!(format!("{:.2}", value.round_dp(2)))
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

pub(crate) struct SalesIndicatorsWidget;

#[async_trait]
impl<P> WidgetProvider<P> for SalesIndicatorsWidget
where
    P: DashboardPorts,
{
    fn default_order(&self) -> Option<i32> {
        Some(10)
    }
    fn key(&self) -> &str {
        SALES_INDICATORS
    }

    fn title(&self) -> &str {
        "Sales indicators"
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
        let open = queries::deal_totals(
            context.ports.pool(),
            context.ports.services(),
            context.scope,
            &open_plan,
        )
        .await?;
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
        mask_data(context, Value::Object(data)).await
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
    fn reference_settings(&self) -> Vec<aphel_sdk::dashboard::WidgetReferenceSetting> {
        vec![aphel_sdk::dashboard::WidgetReferenceSetting {
            field: "pipeline_id",
            package_field: Some("pipeline"),
            kind: aphel_sdk::ReferenceKind::new("workflow.pipeline"),
        }]
    }
    fn default_order(&self) -> Option<i32> {
        Some(50)
    }
    fn key(&self) -> &str {
        DEALS_BY_STAGE
    }

    fn title(&self) -> &str {
        "Deals by stage"
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
        let policy = FieldAccessPolicy::load(
            context.ports.query().definitions(),
            context.scope,
            &RecordType::DEAL,
        )
        .await?;
        if policy.key_access("stage_id") == FieldAccess::Hidden {
            return Err(
                aphel_kernel::application::fields::permissions::not_accessible(vec![
                    FieldViolation::new(
                        "stage_id",
                        aphel_kernel::application::fields::permissions::NOT_ACCESSIBLE_MESSAGE,
                    ),
                ]),
            );
        }
        let currency = context.currency().await?;
        let Some(pipeline) = queries::pipeline(
            context.ports.pool(),
            context.ports.services(),
            context.scope,
            pipeline_id,
        )
        .await?
        else {
            return Ok(json!({ "pipeline": null, "currency": currency, "stages": [] }));
        };
        let plan = planner.plan(
            vec![eq("pipeline_id", pipeline.id.to_string())],
            Vec::new(),
            AGGREGATE_LIMIT,
        )?;
        let aggregates =
            queries::stage_aggregates(context.ports.pool(), context.scope, &plan).await?;
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
        mask_data(
            context,
            json!({
                "pipeline": { "id": pipeline.id.to_string(), "name": pipeline.name },
                "currency": currency,
                "stages": stages,
            }),
        )
        .await
    }
}

pub(crate) struct DealPendingSource;
#[async_trait]
impl aphel_sdk::dashboard::DashboardSource for DealPendingSource {
    fn key(&self) -> &str {
        "deals.stale"
    }
    fn collection(&self) -> &str {
        "pending"
    }
    fn available(&self, scope: &aphel_kernel::domain::access::AccessScope) -> bool {
        scope.filter_for(&Permission::DEALS_READ).is_some()
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let stale_days = settings
            .get("stale_days")
            .and_then(Value::as_i64)
            .unwrap_or(14)
            .clamp(1, 365);
        let limit = settings
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(5)
            .clamp(1, 20);
        let only_mine = settings
            .get("only_mine")
            .and_then(Value::as_bool)
            .unwrap_or(true)
            && context.subject.is_none();
        let stale = match context.planner(&RecordType::DEAL).await? {
            None => Value::Null,
            Some(deals) => {
                let cutoff = context.now - chrono::Duration::days(stale_days);
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
                    let plan = listing_plan(&deals, conditions, "updated_at", limit)?;
                    queries::stale_deals(
                        context.ports.pool(),
                        context.ports.services(),
                        context.scope,
                        &plan,
                    )
                    .await?
                };
                json!({
                    "count": total,
                    "stale_days": stale_days,
                    "items": items.iter().map(stale_deal_json).collect::<Vec<_>>(),
                })
            }
        };
        mask_data(context, json!({"stale_deals": stale})).await
    }
}
fn stale_deal_json(deal: &queries::StaleDeal) -> Value {
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

fn before(field: &str, instant: DateTime<Utc>) -> Filter {
    Filter::condition(field, FilterOperator::Lt, instant_value(instant))
}

fn is_empty(field: &str) -> Filter {
    Filter::condition(field, FilterOperator::IsEmpty, Value::Null)
}

fn optional_amount(value: Option<Decimal>) -> Value {
    value.map(amount).unwrap_or(Value::Null)
}

fn user_json(user: &Option<UserReference>) -> Value {
    user.as_ref()
        .map(|user| json!({ "id": user.id.as_str(), "display_name": user.display_name }))
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

pub(crate) struct DealSummarySource;
#[async_trait]
impl DashboardSource for DealSummarySource {
    fn key(&self) -> &str {
        "deals.summary"
    }
    fn collection(&self) -> &str {
        "summary"
    }
    fn available(&self, scope: &AccessScope) -> bool {
        scope.filter_for(&Permission::DEALS_READ).is_some()
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, WidgetPorts>,
        _settings: &Value,
    ) -> Result<Value, AppError> {
        let mut totals = Map::new();
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

async fn mask_data<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    mut data: Value,
) -> Result<Value, AppError> {
    let policy = FieldAccessPolicy::load(
        context.ports.query().definitions(),
        context.scope,
        &RecordType::DEAL,
    )
    .await?;
    if policy.key_access("amount") == FieldAccess::Hidden
        || policy.key_access(BASE_AMOUNT_FIELD) == FieldAccess::Hidden
    {
        fn clear(value: &mut Value) {
            match value {
                Value::Object(values) => {
                    values.retain(|key, _| key != "amount" && !key.ends_with("_amount"));
                    values.values_mut().for_each(clear);
                }
                Value::Array(items) => items.iter_mut().for_each(clear),
                _ => {}
            }
        }
        clear(&mut data);
    }
    if let Some(items) = data
        .get_mut("stale_deals")
        .and_then(|bucket| bucket.get_mut("items"))
        .and_then(Value::as_array_mut)
    {
        for item in items {
            for (field, key) in [
                ("title", "title"),
                ("owner_id", "owner"),
                ("stage_id", "stage_name"),
            ] {
                if policy.key_access(field) == FieldAccess::Hidden {
                    item[key] = Value::Null;
                }
            }
        }
    }
    Ok(data)
}
