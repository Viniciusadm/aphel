use std::collections::BTreeMap;

use chrono::{Datelike, Days, Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DATE_FORMAT, FORECAST_SETTINGS_RECORD_TYPE, FORECAST_SETTINGS_UPDATED, ForecastBucket,
    ForecastCategory, ForecastDealView, ForecastDealsPage, ForecastDealsQuery,
    ForecastDealsRequest, ForecastGroupRow, ForecastMethod, ForecastMonth, ForecastOwnerFilter,
    ForecastReport, ForecastReportQuery, ForecastSettings, ForecastSummary, MAX_HORIZON_MONTHS,
    ProbabilitySource, UpdateForecastSettingsCommand, is_unavailable, weighted,
};
use crate::store::ForecastStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::fields::permissions::{NOT_ACCESSIBLE_MESSAGE, not_accessible};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::query::{
    AggregateEngine, AggregateFunction, AggregatePeriod, AggregateQuery, ArchivedFilter, Filter,
    FilterOperator, GroupBySpec, MetricSpec, OWNER_TEAM_FIELD, RecordQuery, RecordQueryExecutor,
    RecordQueryPorts, ResolveContext, TimeGranularity,
};
use aphel_kernel::application::shared::{RecordRef, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::dashboard::local_today;
use aphel_sdk::deals::{CreditedUsers, credited_from_owner_filter, credited_won_months};
use aphel_sdk::money::{BASE_AMOUNT_FIELD, BASE_AMOUNT_METRIC};

const DEAL: &str = "deals.deal";
const AMOUNT: &str = "amount";
const PROBABILITY: &str = "probability";
const EXPECTED_CLOSE_DATE: &str = "expected_close_date";
const CLOSED_AT: &str = "closed_at";
const OWNER_ID: &str = "owner_id";
const STAGE_ID: &str = "stage_id";
const WON_COUNT_METRIC: &str = "count";
const INVALID_QUERY: &str = "invalid forecast query";

pub(crate) fn settings_snapshot(settings: &ForecastSettings) -> Value {
    json!({
        "method": settings.method.as_str(),
        "categories_enabled": settings.categories_enabled,
        "best_case_min_probability": settings.best_case_min_probability,
        "commit_min_probability": settings.commit_min_probability,
        "horizon_months": settings.horizon_months,
    })
}

fn percentage(
    value: Option<i64>,
    current: u8,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) -> u8 {
    match value {
        None => current,
        Some(number) if (1..=100).contains(&number) => u8::try_from(number).unwrap_or(current),
        Some(_) => {
            violations.push(FieldViolation::new(path, "must be between 1 and 100"));
            current
        }
    }
}

fn month_start(day: NaiveDate) -> NaiveDate {
    day.with_day(1).unwrap_or(day)
}

fn month_end(start: NaiveDate, months: u32) -> NaiveDate {
    start
        .checked_add_months(Months::new(months))
        .and_then(|next| next.checked_sub_days(Days::new(1)))
        .unwrap_or(start)
}

fn date(day: NaiveDate) -> Value {
    Value::String(day.format(DATE_FORMAT).to_string())
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<ForecastSettings, AppError> {
    Ok(ForecastStore::of(ctx).settings().await?.unwrap_or_default())
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    command: UpdateForecastSettingsCommand,
) -> Result<ForecastSettings, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = settings(ctx).await?;
    if let Some(updated_at) = current.updated_at {
        ensure_version(command.expected_updated_at, updated_at)?;
    }
    let next = merge_settings(&current, &command)
        .map_err(|violations| AppError::invalid_fields("invalid forecast settings", violations))?;
    let mut tx = ctx.begin().await?;
    let saved = write_settings(ctx, &mut tx, &current, next, command.ip).await?;
    tx.commit().await?;
    Ok(saved)
}

pub(crate) fn merge_settings(
    current: &ForecastSettings,
    command: &UpdateForecastSettingsCommand,
) -> Result<ForecastSettings, Vec<FieldViolation>> {
    let mut next = current.clone();
    let mut violations = Vec::new();
    if let Some(method) = &command.method {
        match ForecastMethod::parse(method.trim()) {
            Some(method) => next.method = method,
            None => violations.push(FieldViolation::new(
                "method",
                "must be stage_probability, deal_probability or commit_only",
            )),
        }
    }
    if let Some(enabled) = command.categories_enabled {
        next.categories_enabled = enabled;
    }
    next.best_case_min_probability = percentage(
        command.best_case_min_probability,
        next.best_case_min_probability,
        "best_case_min_probability",
        &mut violations,
    );
    next.commit_min_probability = percentage(
        command.commit_min_probability,
        next.commit_min_probability,
        "commit_min_probability",
        &mut violations,
    );
    if next.best_case_min_probability >= next.commit_min_probability {
        violations.push(FieldViolation::new(
            "commit_min_probability",
            "must be greater than best_case_min_probability",
        ));
    }
    if let Some(horizon) = command.horizon_months {
        match u8::try_from(horizon) {
            Ok(horizon) if (1..=MAX_HORIZON_MONTHS).contains(&horizon) => {
                next.horizon_months = horizon;
            }
            _ => violations.push(FieldViolation::new(
                "horizon_months",
                format!("must be between 1 and {MAX_HORIZON_MONTHS}"),
            )),
        }
    }
    if violations.is_empty() {
        Ok(next)
    } else {
        Err(violations)
    }
}

pub(crate) async fn write_settings(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    current: &ForecastSettings,
    mut next: ForecastSettings,
    ip: Option<String>,
) -> Result<ForecastSettings, AppError> {
    let before = settings_snapshot(current);
    let after = settings_snapshot(&next);
    if before == after && current.updated_at.is_some() {
        return Ok(current.clone());
    }
    let now = ctx.now();
    next.updated_at = Some(now);
    ForecastStore::of(ctx)
        .save_settings(tx, ctx.scope, &next, now)
        .await?;
    publish(ctx, tx, &next, (&before, &after), ip).await?;
    Ok(next)
}

pub(crate) async fn report(
    ctx: &Ctx<'_>,
    query: ForecastReportQuery,
) -> Result<ForecastReport, AppError> {
    let settings = settings(ctx).await?;
    let mut violations = Vec::new();
    let horizon = match query.months {
        None => settings.horizon_months,
        Some(months) => match u8::try_from(months) {
            Ok(months) if (1..=MAX_HORIZON_MONTHS).contains(&months) => months,
            _ => {
                violations.push(FieldViolation::new(
                    "months",
                    format!("must be between 1 and {MAX_HORIZON_MONTHS}"),
                ));
                settings.horizon_months
            }
        },
    };
    let credited = credited_owners(&query.owners);
    let owner_conditions = owner_conditions(&query.owners, &mut violations);
    let owner_conditions = if credited.is_some() {
        Vec::new()
    } else {
        owner_conditions
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_QUERY, violations));
    }
    let context = resolve_context(ctx).await?;
    let from = month_start(local_today(context.now, context.timezone));
    let to = month_end(from, u32::from(horizon));
    let (fields, plan) = open_plan(ctx, owner_conditions.clone(), from, to).await?;
    let source = probability_source(&settings, &fields);
    let store = ForecastStore::of(ctx);
    let rows = aphel_sdk::deals::probability_reader(ctx)?
        .groups(ctx, plan, source, credited.as_ref())
        .await?;
    let won = won(ctx, owner_conditions, from, to, credited.as_ref()).await?;
    let mut months: BTreeMap<NaiveDate, ForecastSummary> = BTreeMap::new();
    let mut cursor = from;
    while cursor <= to {
        months.insert(cursor, empty_summary(&settings));
        match cursor.checked_add_months(Months::new(1)) {
            Some(next) => cursor = next,
            None => break,
        }
    }
    let mut totals = empty_summary(&settings);
    for row in rows {
        let month = month_start(row.month);
        let summary = months
            .entry(month)
            .or_insert_with(|| empty_summary(&settings));
        accumulate(&settings, summary, &row);
        accumulate(&settings, &mut totals, &row);
    }
    match won {
        Some(won) => {
            let mut total = ForecastBucket::default();
            for (month, summary) in months.iter_mut() {
                let bucket = won.get(month).copied().unwrap_or_default();
                total.add(bucket.count, bucket.amount);
                summary.won = Some(bucket);
            }
            totals.won = Some(total);
        }
        None => {
            for summary in months.values_mut() {
                summary.won = None;
            }
            totals.won = None;
        }
    }
    Ok(ForecastReport {
        currency: store.currency().await?,
        timezone: context.timezone.name().to_owned(),
        from,
        to,
        probability_source: source,
        months: months
            .into_iter()
            .map(|(month, summary)| ForecastMonth { month, summary })
            .collect(),
        totals,
        settings,
    })
}

pub(crate) async fn deals(
    ctx: &Ctx<'_>,
    query: ForecastDealsQuery,
) -> Result<ForecastDealsPage, AppError> {
    let settings = settings(ctx).await?;
    let mut violations = Vec::new();
    let month = NaiveDate::parse_from_str(&format!("{}-01", query.month.trim()), DATE_FORMAT).ok();
    if month.is_none() {
        violations.push(FieldViolation::new(
            "month",
            "must be a month in the format YYYY-MM",
        ));
    }
    let category = query.category;
    let credited = credited_owners(&query.owners);
    let owner_conditions = owner_conditions(&query.owners, &mut violations);
    let owner_conditions = if credited.is_some() {
        Vec::new()
    } else {
        owner_conditions
    };
    let Some(month) = month else {
        return Err(AppError::invalid_fields(INVALID_QUERY, violations));
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_QUERY, violations));
    }
    let (fields, plan) = open_plan(ctx, owner_conditions, month, month_end(month, 1)).await?;
    let source = probability_source(&settings, &fields);
    let store = ForecastStore::of(ctx);
    let listing = aphel_sdk::deals::probability_reader(ctx)?
        .deals(
            ctx,
            plan,
            &ForecastDealsRequest {
                source,
                probability_range: category.map(|category| settings.probability_range(category)),
                window: query.window,
                credited,
            },
        )
        .await?;
    let uses_categories = settings.uses_categories();
    let hide_owner = fields.is_hidden(OWNER_ID);
    let hide_stage = fields.is_hidden(STAGE_ID);
    Ok(ForecastDealsPage {
        currency: store.currency().await?,
        month,
        listing: Listing {
            total: listing.total,
            items: listing
                .items
                .into_iter()
                .map(|mut deal| {
                    if hide_owner {
                        deal.owner = None;
                    }
                    if hide_stage {
                        deal.stage_name = String::new();
                    }
                    let category = settings.category(deal.probability);
                    let weighted_amount = deal.amount.map(|amount| match settings.method {
                        ForecastMethod::CommitOnly => {
                            if category == ForecastCategory::Commit {
                                amount
                            } else {
                                Decimal::ZERO
                            }
                        }
                        _ => weighted(amount, deal.probability),
                    });
                    ForecastDealView {
                        category: uses_categories.then_some(category),
                        weighted_amount,
                        deal,
                    }
                })
                .collect(),
        },
    })
}

async fn resolve_context(ctx: &Ctx<'_>) -> Result<ResolveContext, AppError> {
    Ok(ResolveContext {
        now: ctx.now(),
        timezone: ctx.query().executor().timezone().await?,
        user_id: ctx.scope.user_id().clone(),
    })
}

async fn open_plan(
    ctx: &Ctx<'_>,
    mut conditions: Vec<Filter>,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<(FieldSet, RecordQuery), AppError> {
    let scope = ctx.scope;
    let query = ctx.query();
    let definition = query.registry().require(DEAL)?;
    scope.record_filter(&definition.read_permission)?;
    let fields = FieldSet::for_scope(query.definitions(), definition, scope).await?;
    if fields.is_hidden(AMOUNT) {
        return Err(not_accessible(vec![FieldViolation::new(
            AMOUNT,
            NOT_ACCESSIBLE_MESSAGE,
        )]));
    }
    conditions.push(Filter::condition(
        "status",
        FilterOperator::Eq,
        Value::String("open".to_owned()),
    ));
    conditions.push(Filter::condition(
        EXPECTED_CLOSE_DATE,
        FilterOperator::Gte,
        date(from),
    ));
    conditions.push(Filter::condition(
        EXPECTED_CLOSE_DATE,
        FilterOperator::Lte,
        date(to),
    ));
    let plan = RecordQuery {
        filter: Some(Filter::and(conditions)),
        sort: Vec::new(),
        search: None,
        archived: ArchivedFilter::Exclude,
        window: ListWindow {
            offset: 0,
            limit: 1,
        },
    };
    Ok((fields, plan))
}

async fn won(
    ctx: &Ctx<'_>,
    mut conditions: Vec<Filter>,
    from: NaiveDate,
    to: NaiveDate,
    credited: Option<&CreditedUsers>,
) -> Result<Option<BTreeMap<NaiveDate, ForecastBucket>>, AppError> {
    let scope = ctx.scope;
    let query_ports = ctx.query();
    if let Some(credited) = credited {
        return match credited_won_months(ctx, credited, BASE_AMOUNT_FIELD, from, to).await {
            Ok(Some(months)) => Ok(Some(
                months
                    .into_iter()
                    .map(|(month, totals)| {
                        (
                            month,
                            ForecastBucket {
                                count: totals.count,
                                amount: totals.amount,
                            },
                        )
                    })
                    .collect(),
            )),
            Ok(None) => Ok(None),
            Err(error) if is_unavailable(&error) => Ok(None),
            Err(error) => Err(error),
        };
    }
    conditions.push(Filter::condition(
        "status",
        FilterOperator::Eq,
        Value::String("won".to_owned()),
    ));
    let query = AggregateQuery::new(DEAL)
        .filter(Filter::and(conditions))
        .group(GroupBySpec::by_time(CLOSED_AT, TimeGranularity::Month))
        .metrics(vec![
            MetricSpec::count(),
            MetricSpec::of(AggregateFunction::Sum, BASE_AMOUNT_FIELD),
        ])
        .period(AggregatePeriod::between(
            CLOSED_AT,
            &from.format(DATE_FORMAT).to_string(),
            &to.format(DATE_FORMAT).to_string(),
        ));
    let result = match AggregateEngine::new(&query_ports).run(scope, query).await {
        Ok(result) => result,
        Err(error) if is_unavailable(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    let count_index = result.metric_index(WON_COUNT_METRIC);
    let amount_index = result.metric_index(BASE_AMOUNT_METRIC);
    let mut buckets = BTreeMap::new();
    for row in &result.rows {
        let Some(month) = row
            .groups
            .first()
            .and_then(|group| group.value.as_deref())
            .and_then(|value| NaiveDate::parse_from_str(value, DATE_FORMAT).ok())
        else {
            continue;
        };
        let value = |index: Option<usize>| {
            index
                .and_then(|index| row.values.get(index).copied().flatten())
                .unwrap_or_default()
        };
        let count = value(count_index).to_u64().unwrap_or_default();
        buckets.insert(
            month_start(month),
            ForecastBucket {
                count,
                amount: value(amount_index),
            },
        );
    }
    Ok(Some(buckets))
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    settings: &ForecastSettings,
    (before, after): (&Value, &Value),
    ip: Option<String>,
) -> Result<(), AppError> {
    let changes = diff_values(before, after, &DiffOptions::default())?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(FORECAST_SETTINGS_RECORD_TYPE, Uuid::nil()),
            event_type: FORECAST_SETTINGS_UPDATED,
            payload: settings_snapshot(settings),
            changes,
            ip,
        },
    )
    .await
}

fn probability_source(settings: &ForecastSettings, fields: &FieldSet) -> ProbabilitySource {
    if settings.method == ForecastMethod::StageProbability || fields.is_hidden(PROBABILITY) {
        ProbabilitySource::Stage
    } else {
        ProbabilitySource::DealOrStage
    }
}

fn empty_summary(settings: &ForecastSettings) -> ForecastSummary {
    ForecastSummary {
        categories: settings.uses_categories().then(|| {
            ForecastCategory::ALL
                .into_iter()
                .map(|category| (category, ForecastBucket::default()))
                .collect()
        }),
        ..ForecastSummary::default()
    }
}

fn accumulate(settings: &ForecastSettings, summary: &mut ForecastSummary, row: &ForecastGroupRow) {
    summary.open.add(row.count, row.amount);
    let category = settings.category(row.probability);
    if let Some(categories) = summary.categories.as_mut() {
        categories
            .entry(category)
            .or_default()
            .add(row.count, row.amount);
    }
    summary.forecast += match settings.method {
        ForecastMethod::CommitOnly => {
            if category == ForecastCategory::Commit {
                row.amount
            } else {
                Decimal::ZERO
            }
        }
        _ => weighted(row.amount, row.probability),
    };
}

fn credited_owners(owners: &ForecastOwnerFilter) -> Option<CreditedUsers> {
    credited_from_owner_filter(owners.owner_id.as_deref(), owners.team_id.as_deref())
}

fn owner_conditions(
    owners: &ForecastOwnerFilter,
    violations: &mut Vec<FieldViolation>,
) -> Vec<Filter> {
    let mut conditions = Vec::new();
    let mut push = |value: &Option<String>, field: &str, path: &str| {
        let Some(value) = value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return;
        };
        match Uuid::parse_str(value) {
            Ok(id) => conditions.push(Filter::condition(
                field,
                FilterOperator::In,
                json!([id.to_string()]),
            )),
            Err(_) => violations.push(FieldViolation::new(path, "must be a valid id")),
        }
    };
    push(&owners.owner_id, OWNER_ID, "owner_id");
    push(&owners.team_id, OWNER_TEAM_FIELD, "team_id");
    conditions
}
