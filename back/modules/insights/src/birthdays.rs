use aphel_contract_crm::CrmPermissions;
use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aggregate::{
    combine, condition, count_at, is_field_not_accessible, run, visible_fields,
};
use crate::settings::{SettingsReader, invalid_settings};
use crate::{CATEGORY, KEY};
use ::dashboard::engine::{DashboardPorts, WidgetContext, WidgetProvider, WidgetSize};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, RecordQuery, RecordQueryEngine, SortSpec,
};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::modules::module_settings;
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::runtime_state;

pub(crate) const BIRTHDAY_FIELD_SETTING: &str = "birthday_field";

const CONTACT: &str = "contacts.contact";
const DATE_FIELD_TYPE: &str = "date";
const FIRST_YEAR: i32 = 1900;
const YEARS_PER_QUERY: i32 = 40;
const DATES_PER_QUERY: usize = 40;
const MAX_GROUPS: u32 = 1000;

struct BirthdaySettings {
    limit: i64,
}

fn parse(settings: &Value) -> Result<(BirthdaySettings, Value), Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = BirthdaySettings {
        limit: reader.integer("limit", 10, 5, 50),
    };
    Ok((parsed, reader.finish()?))
}

fn state(status: &str, month: u32, today: u32) -> Value {
    json!({
        "status": status,
        "month": month,
        "today": today,
        "field_label": null,
        "items": [],
        "total": 0,
        "truncated": false,
    })
}

fn month_bounds(year: i32, month: u32) -> Option<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)?
    };
    Some((first, next.pred_opt()?))
}

fn date_text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.get(..10)?, "%Y-%m-%d").ok()
}

fn record_date(record: &Value, field: &str) -> Option<NaiveDate> {
    record
        .get(field)
        .and_then(Value::as_str)
        .or_else(|| record["custom_fields"].get(field).and_then(Value::as_str))
        .and_then(parse_date)
}

fn is_missing(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

pub(crate) struct BirthdaysWidget;

impl BirthdaysWidget {
    pub(crate) const KEY: &'static str = "insights.birthdays";

    async fn dates_in_month<P: DashboardPorts>(
        &self,
        context: &WidgetContext<'_, P>,
        field: &str,
        month: u32,
        current_year: i32,
    ) -> Result<(BTreeMap<NaiveDate, u64>, bool), AppError> {
        let mut dates = BTreeMap::new();
        let mut truncated = false;
        let mut start = FIRST_YEAR;
        while start <= current_year {
            let end = (start + YEARS_PER_QUERY - 1).min(current_year);
            let ranges: Vec<Value> = (start..=end)
                .filter_map(|year| month_bounds(year, month))
                .map(|(first, last)| {
                    condition(field, "between", json!([date_text(first), date_text(last)]))
                })
                .collect();
            let output = run(
                context,
                json!({
                    "record_type": CONTACT,
                    "filter": combine("or", ranges),
                    "group_by": [{ "field": field, "granularity": "day" }],
                    "metrics": [{ "fn": "count" }],
                    "limit": MAX_GROUPS,
                }),
            )
            .await?;
            truncated |= output.truncated;
            for row in &output.rows {
                let date = row
                    .groups
                    .first()
                    .and_then(|group| group.value.as_deref())
                    .and_then(parse_date);
                if let Some(date) = date.filter(|date| date.month() == month) {
                    *dates.entry(date).or_insert(0) += count_at(&row.values, 0);
                }
            }
            start = end + 1;
        }
        Ok((dates, truncated))
    }

    async fn find_ids<P: DashboardPorts>(
        &self,
        context: &WidgetContext<'_, P>,
        filter: Value,
        limit: u64,
    ) -> Result<Vec<Uuid>, AppError> {
        let filter: Filter = serde_json::from_value(filter)
            .map_err(|error| AppError::validation_code("invalid_filter", error.to_string()))?;
        let listing = RecordQueryEngine::new(context.ports.query())
            .query(
                context.scope,
                CONTACT,
                RecordQuery {
                    filter: Some(filter),
                    sort: vec![SortSpec::asc("name")],
                    search: None,
                    archived: ArchivedFilter::Exclude,
                    window: ListWindow {
                        offset: 0,
                        limit: limit.clamp(1, 500),
                    },
                },
            )
            .await?;
        Ok(listing.items)
    }
}

#[async_trait]
impl<P> WidgetProvider<P> for BirthdaysWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        Self::KEY
    }

    fn title(&self) -> &str {
        "Aniversariantes do mês"
    }

    fn category(&self) -> &str {
        CATEGORY
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::CONTACTS_READ]
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Small
    }

    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({ "limit": 10 })
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
        let today = context.now.with_timezone(&context.timezone).date_naive();
        let (month, today_day) = (today.month(), today.day());
        let state_handle = runtime_state()?;
        let module_settings = module_settings(&state_handle.db()?, KEY).await?;
        let Some(field) = module_settings
            .get(BIRTHDAY_FIELD_SETTING)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|field| !field.is_empty())
        else {
            return Ok(state("not_configured", month, today_day));
        };
        let fields = visible_fields(context, CONTACT).await?;
        let Some(info) = fields
            .iter()
            .find(|info| info.key == field && info.field_type == DATE_FIELD_TYPE)
        else {
            return Ok(state("field_unavailable", month, today_day));
        };
        let (dates, truncated) = match self
            .dates_in_month(context, field, month, today.year())
            .await
        {
            Err(error) if is_field_not_accessible(&error) => {
                return Ok(state("field_unavailable", month, today_day));
            }
            result => result?,
        };
        let total: u64 = dates.values().sum();
        let mut ordered: Vec<NaiveDate> = dates.keys().copied().collect();
        ordered.sort_by_key(|date| (date.day() < today_day, date.day(), date.year()));
        let limit = usize::try_from(settings.limit).unwrap_or(10);
        let mut wanted = Vec::new();
        let mut planned = 0_u64;
        for date in ordered {
            if planned >= limit as u64 {
                break;
            }
            planned += dates.get(&date).copied().unwrap_or(0);
            wanted.push(date);
        }
        let mut ids: Vec<Uuid> = Vec::new();
        for chunk in wanted.chunks(DATES_PER_QUERY) {
            let remaining = limit.saturating_sub(ids.len());
            if remaining == 0 {
                break;
            }
            let matches: Vec<Value> = chunk
                .iter()
                .map(|date| condition(field, "eq", json!(date_text(*date))))
                .collect();
            let Some(filter) = combine("or", matches) else {
                continue;
            };
            ids.extend(self.find_ids(context, filter, remaining as u64).await?);
        }
        let registry = state_handle.registry().await?;
        let handler = registry.handler(CONTACT)?;
        let record_ctx = RecordCtx::new(context.scope, &state_handle);
        let mut items = Vec::new();
        for id in ids {
            let record = match handler.load(&record_ctx, id).await {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(error) if is_missing(&error) => continue,
                Err(error) => return Err(error),
            };
            let Some(date) = record_date(&record, field) else {
                continue;
            };
            let name = record["name"].as_str().unwrap_or_default().to_owned();
            items.push((date.day(), name, id));
        }
        items.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
        let items: Vec<Value> = items
            .into_iter()
            .map(|(day, name, id)| {
                json!({
                    "id": id.to_string(),
                    "name": name,
                    "day": day,
                    "is_today": day == today_day,
                })
            })
            .collect();
        Ok(json!({
            "status": "ok",
            "month": month,
            "today": today_day,
            "field_label": info.label,
            "items": items,
            "total": total,
            "truncated": truncated,
        }))
    }
}
