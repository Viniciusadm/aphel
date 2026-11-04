use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};

use async_trait::async_trait;
use chrono::Days;
use serde_json::{Map, Value, json};

use aphel_sdk::deals::{RevenueSummary, revenue_money as money};

use aphel_kernel::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_kernel::application::query::{Filter, FilterOperator, RecordQueryPorts, SortSpec};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::dashboard::local_today;
use aphel_sdk::dashboard::{DashboardPorts, SettingsReader, WidgetContext, WidgetProvider};
use aphel_sdk::deals::date;

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
impl WidgetProvider<aphel_sdk::dashboard::WidgetPorts> for RenewalsDueWidget {
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
        context: &WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
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
        let revenue = context
            .ports
            .services()
            .require::<dyn RevenueSummary>()?
            .summarize(context, conditions.clone(), None)
            .await?;
        let count = revenue.count;
        let items = if count == 0 {
            Vec::new()
        } else {
            let plan = planner.plan(
                conditions,
                vec![SortSpec::asc("contract_end")],
                u64::try_from(settings.limit).unwrap_or(1),
            )?;
            context
                .ports
                .services()
                .require::<dyn aphel_sdk::deals::DealRenewalReader>()?
                .widget_rows(context, &plan)
                .await?
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
