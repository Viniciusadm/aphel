use aphel_contract_crm::CrmPermissions;
use std::str::FromStr;

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde_json::{Value, json};

use crate::CATEGORY;
use crate::aggregate::{
    AggregateOutput, condition, count_at, has_field, is_field_not_accessible, period, run, text_at,
    visible_fields, with_subject,
};
use crate::settings::{SettingsReader, invalid_settings};
use aphel_kernel::application::dashboard::{
    DashboardPorts, WidgetContext, WidgetProvider, WidgetSize,
};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

const SOURCE_FIELD: &str = "source";
const AMOUNT_FIELD: &str = "amount";
const PERIOD_FIELD: &str = "created_at";

struct DealsBySourceSettings {
    limit: i64,
    only_won: bool,
}

fn parse(settings: &Value) -> Result<(DealsBySourceSettings, Value), Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = DealsBySourceSettings {
        limit: reader.integer("limit", 8, 3, 20),
        only_won: reader.boolean("only_won", false),
    };
    Ok((parsed, reader.finish()?))
}

pub(crate) struct DealsBySourceWidget;

impl DealsBySourceWidget {
    pub(crate) const KEY: &'static str = "insights.deals_by_source";

    async fn aggregate<P: DashboardPorts>(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &DealsBySourceSettings,
        with_amount: bool,
    ) -> Result<AggregateOutput, AppError> {
        let mut conditions = Vec::new();
        if settings.only_won {
            conditions.push(condition("status", "eq", json!("won")));
        }
        let mut metrics = vec![json!({ "fn": "count" })];
        if with_amount {
            metrics.push(json!({ "fn": "sum", "field": AMOUNT_FIELD }));
        }
        run(
            context,
            json!({
                "record_type": "deals.deal",
                "filter": with_subject(context, conditions)?,
                "group_by": [{ "field": SOURCE_FIELD }],
                "metrics": metrics,
                "period": period(context, PERIOD_FIELD),
                "sort": [{ "key": "count", "direction": "desc" }],
                "limit": settings.limit,
            }),
        )
        .await
    }
}

#[async_trait]
impl<P> WidgetProvider<P> for DealsBySourceWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        Self::KEY
    }

    fn title(&self) -> &str {
        "Oportunidades por origem"
    }

    fn category(&self) -> &str {
        CATEGORY
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::DEALS_READ]
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Medium
    }

    fn default_settings(&self) -> Value {
        json!({ "limit": 8, "only_won": false })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        parse(settings).map(|(_, normalized)| normalized)
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let (settings, _) = parse(settings).map_err(invalid_settings)?;
        let fields = visible_fields(context, "deals.deal").await?;
        let currency = context.currency().await?;
        if !has_field(&fields, SOURCE_FIELD) {
            return Ok(json!({
                "status": "source_hidden",
                "currency": currency,
                "only_won": settings.only_won,
                "amount_visible": false,
                "rows": [],
                "other": null,
                "total": { "count": 0, "amount": null },
            }));
        }
        let mut amount_visible = has_field(&fields, AMOUNT_FIELD);
        let output = match self.aggregate(context, &settings, amount_visible).await {
            Err(error) if amount_visible && is_field_not_accessible(&error) => {
                amount_visible = false;
                self.aggregate(context, &settings, false).await?
            }
            result => result?,
        };
        let amount = |values: &[Option<String>]| {
            if amount_visible {
                text_at(values, 1).map_or(json!("0"), Value::from)
            } else {
                Value::Null
            }
        };
        let rows: Vec<Value> = output
            .rows
            .iter()
            .map(|row| {
                let source = row
                    .groups
                    .first()
                    .and_then(|group| group.label.clone().or_else(|| group.value.clone()))
                    .filter(|source| !source.trim().is_empty());
                json!({
                    "source": source,
                    "count": count_at(&row.values, 0),
                    "amount": amount(&row.values),
                })
            })
            .collect();
        let total_count = count_at(&output.totals, 0);
        let shown_count: u64 = output.rows.iter().map(|row| count_at(&row.values, 0)).sum();
        let other = (output.truncated && total_count > shown_count).then(|| {
            json!({
                "count": total_count - shown_count,
                "amount": if amount_visible {
                    json!(remaining_amount(&output))
                } else {
                    Value::Null
                },
            })
        });
        Ok(json!({
            "status": "ok",
            "currency": currency,
            "only_won": settings.only_won,
            "amount_visible": amount_visible,
            "rows": rows,
            "other": other,
            "total": { "count": total_count, "amount": amount(&output.totals) },
        }))
    }
}

fn remaining_amount(output: &AggregateOutput) -> String {
    let parse = |values: &[Option<String>]| {
        text_at(values, 1)
            .and_then(|value| Decimal::from_str(&value).ok())
            .unwrap_or_default()
    };
    let shown: Decimal = output.rows.iter().map(|row| parse(&row.values)).sum();
    (parse(&output.totals) - shown)
        .max(Decimal::ZERO)
        .normalize()
        .to_string()
}
