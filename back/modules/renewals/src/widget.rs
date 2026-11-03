use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{Days, NaiveDate};
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

use deals_revenue::widgets::{base_revenue, money};

use ::dashboard::engine::{DashboardPorts, SettingsReader, WidgetContext, WidgetProvider};
use ::dashboard::engine_domain::local_today;
use aphel_kernel::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_kernel::application::query::{
    Filter, FilterOperator, QueryPlan, RecordQueryPorts, SortSpec,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::query::compiler::{Projection, build_query};
use aphel_kernel::infrastructure::repositories::mapper::invalid_column;
use aphel_kernel::records::constrain_query;
use aphel_kernel::state::runtime_pool;
use deals::domain::date;

pub(crate) const RENEWALS_DUE_WIDGET: &str = "renewals_due";

const DEFAULT_RENEWAL_WINDOW_DAYS: i64 = 60;
const DEFAULT_RENEWAL_LIMIT: i64 = 5;

fn missing_permission() -> AppError {
    AppError::forbidden_code(
        aphel_kernel::application::access::PERMISSION_DENIED,
        "missing permission deals.read",
    )
}

async fn deal_field_visible<P: DashboardPorts>(
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

struct RenewalDeal {
    id: Uuid,
    title: String,
    amount: Option<Decimal>,
    currency: String,
    stage_name: String,
    contract_end: Option<NaiveDate>,
    owner_id: Option<String>,
    owner_name: Option<String>,
}

async fn renewal_deals(
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<RenewalDeal>, AppError> {
    let pool = runtime_pool()?;
    let constrained = constrain_query(&pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let ids: Vec<String> = build_query(scope, plan, Projection::Ids)?
        .build_query_scalar()
        .fetch_all(&pool)
        .await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT d.id, d.title, d.amount, d.currency, s.name AS stage_name, d.contract_end, \
         d.owner_id, u.display_name AS owner_name FROM deals d \
         JOIN workflow_stages s ON s.id = d.stage_id \
         LEFT JOIN users u ON u.id = d.owner_id WHERE d.id IN (",
    );
    let mut separated = builder.separated(", ");
    for id in &ids {
        separated.push_bind(id.clone());
    }
    builder.push(")");
    let mut rows: HashMap<String, RenewalDeal> = HashMap::new();
    for row in builder.build().fetch_all(&pool).await? {
        let id: String = row.try_get("id")?;
        let deal = RenewalDeal {
            id: Uuid::parse_str(&id).map_err(|_| invalid_column("deals.id", &id))?,
            title: row.try_get("title")?,
            amount: row.try_get("amount")?,
            currency: row.try_get("currency")?,
            stage_name: row.try_get("stage_name")?,
            contract_end: row.try_get("contract_end")?,
            owner_id: row.try_get("owner_id")?,
            owner_name: row.try_get("owner_name")?,
        };
        rows.insert(id, deal);
    }
    Ok(ids.iter().filter_map(|id| rows.remove(id)).collect())
}

pub(crate) struct RenewalsDueWidget;

struct RenewalsDueSettings {
    days: i64,
    only_mine: bool,
    limit: i64,
}

fn renewals_settings(settings: &Value) -> Result<RenewalsDueSettings, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = RenewalsDueSettings {
        days: reader.integer("days", DEFAULT_RENEWAL_WINDOW_DAYS, 1, 365),
        only_mine: reader.boolean("only_mine", false),
        limit: reader.integer("limit", DEFAULT_RENEWAL_LIMIT, 1, 20),
    };
    reader.finish(Map::new())?;
    Ok(parsed)
}

fn renewals_json(settings: &RenewalsDueSettings) -> Value {
    json!({
        "days": settings.days,
        "only_mine": settings.only_mine,
        "limit": settings.limit,
    })
}

#[async_trait]
impl<P> WidgetProvider<P> for RenewalsDueWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        RENEWALS_DUE_WIDGET
    }

    fn title(&self) -> &str {
        "Contratos a renovar"
    }

    fn category(&self) -> &str {
        "sales"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::DEALS_READ]
    }

    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({
            "days": DEFAULT_RENEWAL_WINDOW_DAYS,
            "only_mine": false,
            "limit": DEFAULT_RENEWAL_LIMIT,
        })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        renewals_settings(settings).map(|parsed| renewals_json(&parsed))
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let settings = renewals_settings(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let Some(planner) = context.planner(&RecordType::DEAL).await? else {
            return Err(missing_permission());
        };
        let today = local_today(context.now, context.timezone);
        let until = today
            .checked_add_days(Days::new(u64::try_from(settings.days).unwrap_or(0)))
            .unwrap_or(today);
        let mut conditions = vec![
            Filter::condition("status", FilterOperator::Eq, json!("won")),
            Filter::condition("contract_end", FilterOperator::Gte, json!(date(today))),
            Filter::condition("contract_end", FilterOperator::Lte, json!(date(until))),
        ];
        if settings.only_mine {
            conditions.push(Filter::condition(
                "owner_id",
                FilterOperator::Eq,
                json!("me"),
            ));
        }
        let revenue = base_revenue(context, conditions.clone(), None).await?;
        let count = revenue.count;
        let items = if count == 0 {
            Vec::new()
        } else {
            let plan = planner.plan(
                conditions,
                vec![SortSpec::asc("contract_end")],
                u64::try_from(settings.limit).unwrap_or(1),
            )?;
            renewal_deals(context.scope, &plan).await?
        };
        let amount_visible = deal_field_visible(context, "amount").await?;
        let contract_end_visible = deal_field_visible(context, "contract_end").await?;
        Ok(json!({
            "currency": context.currency().await?,
            "days": settings.days,
            "count": count,
            "mrr": money(revenue.mrr),
            "arr": money(revenue.arr),
            "partial": revenue.partial,
            "items": items
                .iter()
                .map(|deal| json!({
                    "id": deal.id.to_string(),
                    "title": deal.title,
                    "amount": money(deal.amount.filter(|_| amount_visible)),
                    "currency": deal.currency,
                    "stage_name": deal.stage_name,
                    "contract_end": deal
                        .contract_end
                        .filter(|_| contract_end_visible)
                        .map(date),
                    "owner": deal.owner_id.as_ref().map(|id| json!({
                        "id": id,
                        "display_name": deal.owner_name.clone().unwrap_or_default(),
                    })),
                }))
                .collect::<Vec<_>>(),
        }))
    }
}
