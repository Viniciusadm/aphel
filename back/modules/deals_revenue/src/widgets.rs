use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value, json};

use std::str::FromStr;

use aphel_kernel::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_kernel::application::query::{
    AggregateEngine, AggregateFunction, AggregatePeriod, AggregateQuery, Filter, FilterOperator,
    GroupBySpec, MAX_AGGREGATE_LIMIT, MetricSpec, RecordQueryPorts,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::domain::period::ResolvedPeriod;
use aphel_kernel::errors::AppError;
use aphel_sdk::dashboard::{DashboardPorts, WidgetContext, WidgetProvider};

pub(crate) const RECURRING_REVENUE_WIDGET: &str = "recurring_revenue";
const EXCHANGE_RATE_FIELD: &str = "exchange_rate";

pub use aphel_sdk::deals::revenue_money as money;

pub(crate) fn whole(value: Option<Decimal>) -> u64 {
    value.and_then(|value| value.to_u64()).unwrap_or_default()
}

fn missing_permission() -> AppError {
    AppError::forbidden_code(
        aphel_kernel::application::access::PERMISSION_DENIED,
        "missing permission deals.read",
    )
}

async fn field_visible<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    key: &str,
) -> Result<bool, AppError> {
    let policy = FieldAccessPolicy::load(
        context.ports.query().definitions(),
        context.scope,
        &RecordType::DEAL,
    )
    .await?;
    Ok(policy.key_access(key) != FieldAccess::Hidden)
}

pub(crate) async fn revenue_visible<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
) -> Result<bool, AppError> {
    field_visible(context, "mrr").await
}

fn revenue_metrics(visible: bool) -> Vec<MetricSpec> {
    let mut metrics = vec![MetricSpec::count()];
    if visible {
        metrics.extend([
            MetricSpec::of(AggregateFunction::Sum, "mrr"),
            MetricSpec::of(AggregateFunction::Sum, "arr"),
            MetricSpec::of(AggregateFunction::Sum, "tcv"),
        ]);
    }
    metrics
}

pub use aphel_sdk::deals::RevenueTotals as BaseRevenue;

trait RevenueJson {
    fn json(&self) -> Value;
}
impl RevenueJson for BaseRevenue {
    fn json(&self) -> Value {
        json!({
            "won_count": self.count,
            "mrr": money(self.mrr),
            "arr": money(self.arr),
            "tcv": money(self.tcv),
            "partial": self.partial,
        })
    }
}

pub async fn base_revenue<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    conditions: Vec<Filter>,
    period: Option<AggregatePeriod>,
) -> Result<BaseRevenue, AppError> {
    let visible = revenue_visible(context).await?;
    let convertible = visible && field_visible(context, EXCHANGE_RATE_FIELD).await?;
    let mut query = AggregateQuery::new("deals.deal")
        .filter(Filter::and(context.with_subject(conditions)))
        .metrics(revenue_metrics(visible));
    if let Some(period) = period {
        query = query.period(period);
    }
    if convertible {
        query = query.group(GroupBySpec::field(EXCHANGE_RATE_FIELD));
        query.limit = Some(MAX_AGGREGATE_LIMIT);
    }
    let result = AggregateEngine::new(context.ports.query())
        .run(context.scope, query)
        .await?;
    let mut revenue = BaseRevenue {
        count: whole(result.total("count")),
        ..BaseRevenue::default()
    };
    if !visible {
        return Ok(revenue);
    }
    if !convertible {
        revenue.mrr = Some(result.total("sum:mrr").unwrap_or_default());
        revenue.arr = Some(result.total("sum:arr").unwrap_or_default());
        revenue.tcv = Some(result.total("sum:tcv").unwrap_or_default());
        revenue.partial = true;
        return Ok(revenue);
    }
    let indexes = ["sum:mrr", "sum:arr", "sum:tcv"].map(|key| result.metric_index(key));
    let mut sums = [Decimal::ZERO; 3];
    for row in &result.rows {
        let rate = row
            .groups
            .first()
            .and_then(|group| group.value.as_deref())
            .and_then(|value| Decimal::from_str(value).ok());
        let Some(rate) = rate else {
            if indexes
                .iter()
                .flatten()
                .any(|index| row.values.get(*index).copied().flatten().is_some())
            {
                revenue.partial = true;
            }
            continue;
        };
        for (sum, index) in sums.iter_mut().zip(indexes) {
            if let Some(value) = index.and_then(|index| row.values.get(index).copied().flatten()) {
                *sum += value * rate;
            }
        }
    }
    revenue.partial |= result.truncated;
    revenue.mrr = Some(sums[0]);
    revenue.arr = Some(sums[1]);
    revenue.tcv = Some(sums[2]);
    Ok(revenue)
}

fn period_of(field: &str, period: &ResolvedPeriod) -> AggregatePeriod {
    AggregatePeriod::between(
        field,
        &period.from.format("%Y-%m-%d").to_string(),
        &period.to.format("%Y-%m-%d").to_string(),
    )
}

pub(crate) struct RecurringRevenueWidget;

#[async_trait]
impl<P> WidgetProvider<P> for RecurringRevenueWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        RECURRING_REVENUE_WIDGET
    }

    fn title(&self) -> &str {
        "MRR ganho"
    }

    fn category(&self) -> &str {
        "sales"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::DEALS_READ]
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        _settings: &Value,
    ) -> Result<Value, AppError> {
        if !context.can_read(&Permission::DEALS_READ) {
            return Err(missing_permission());
        }
        let conditions = vec![
            Filter::condition("status", FilterOperator::Eq, json!("won")),
            Filter::condition("mrr", FilterOperator::IsNotEmpty, Value::Null),
        ];
        let current = base_revenue(
            context,
            conditions.clone(),
            Some(context.aggregate_period("closed_at")),
        )
        .await?;
        let previous = match (&context.previous_period, context.compare_to_previous) {
            (Some(period), true) => {
                Some(base_revenue(context, conditions, Some(period_of("closed_at", period))).await?)
            }
            _ => None,
        };
        let mut data = Map::new();
        data.insert("currency".to_owned(), json!(context.currency().await?));
        if let Value::Object(current) = current.json() {
            data.extend(current);
        }
        data.insert(
            "previous".to_owned(),
            previous
                .map(|previous| previous.json())
                .unwrap_or(Value::Null),
        );
        Ok(Value::Object(data))
    }
}

pub(crate) struct Summary;
#[async_trait]
impl aphel_sdk::deals::RevenueSummary for Summary {
    async fn summarize(
        &self,
        context: &WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
        conditions: Vec<Filter>,
        period: Option<AggregatePeriod>,
    ) -> Result<BaseRevenue, AppError> {
        base_revenue(context, conditions, period).await
    }
}
