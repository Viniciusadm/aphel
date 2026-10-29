use std::collections::{BTreeMap, HashMap};

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::Value;
use uuid::Uuid;

use crate::domain::builtin::{
    ConversionQuery, ConversionReport, ConversionSnapshot, ConversionSummary, CycleTime,
    FunnelBucket, InstantRange, LostReasonTotal, PerformanceDimension, PerformanceMetrics,
    PerformanceQuery, PerformanceReport, PerformanceRow, ReportPipeline, StageDuration,
    StageFunnel, StageLoss,
};
use crate::store::BuiltinReportStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::dashboard::{PeriodQuery, instant_value, parse_period};
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::fields::permissions::{NOT_ACCESSIBLE_MESSAGE, not_accessible};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    AggregateEngine, AggregateFunction, AggregatePeriod, AggregateQuery, AggregateResult,
    ArchivedFilter, Filter, FilterOperator, GroupBySpec, MAX_AGGREGATE_LIMIT, MetricSpec,
    QueryPlan, RecordQuery, RecordQueryExecutor, RecordQueryPorts, ResolveContext, resolve_query,
};
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::domain::dashboard::{ResolvedPeriod, previous_period, resolve_period};
use aphel_kernel::errors::{AppError, FieldViolation};
use currencies::domain::{BASE_AMOUNT_FIELD, BASE_AMOUNT_METRIC};
use pipelines::domain::PIPELINE_NOT_FOUND;
use pipelines::domain::StageKind;
use pipelines::domain::pipeline_visible;
use pipelines::store::PipelineStore;

const AMOUNT_FIELD: &str = "amount";
const DEAL: &str = "deal";
const ACTIVITY: &str = "activity";

struct DealPlans {
    fields: FieldSet,
    context: ResolveContext,
    visibility: RecordScopeFilter,
}

impl DealPlans {
    fn plan(&self, conditions: Vec<Filter>) -> Result<QueryPlan, AppError> {
        resolve_query(
            &self.fields,
            &self.context,
            self.visibility.clone(),
            RecordQuery {
                filter: Some(Filter::and(conditions)),
                sort: Vec::new(),
                search: None,
                archived: ArchivedFilter::Exclude,
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
            },
        )
    }

    fn amounts_visible(&self) -> bool {
        !self.fields.is_hidden(AMOUNT_FIELD) && !self.fields.is_hidden(BASE_AMOUNT_FIELD)
    }
}

fn eq(field: &str, value: impl Into<Value>) -> Filter {
    Filter::condition(field, FilterOperator::Eq, value.into())
}

fn within(field: &str, range: InstantRange) -> Vec<Filter> {
    vec![
        Filter::condition(field, FilterOperator::Gte, instant_value(range.starts_at)),
        Filter::condition(field, FilterOperator::Lt, instant_value(range.ends_at)),
    ]
}

fn range_of(period: &ResolvedPeriod) -> InstantRange {
    InstantRange {
        starts_at: period.starts_at,
        ends_at: period.ends_at,
    }
}

fn day(date: chrono::NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn rate(part: u64, whole: u64) -> Option<f64> {
    if whole == 0 {
        None
    } else {
        Some(((part as f64 / whole as f64) * 10_000.0).round() / 10_000.0)
    }
}

fn whole(value: Option<Decimal>) -> u64 {
    value.and_then(|value| value.to_u64()).unwrap_or_default()
}

fn parse_pipeline(value: &Option<String>) -> Result<Option<Uuid>, AppError> {
    match value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => Ok(None),
        Some(text) => Uuid::parse_str(text).map(Some).map_err(|_| {
            AppError::invalid_fields(
                "invalid report",
                vec![FieldViolation::new("pipeline_id", "must be a UUID")],
            )
        }),
    }
}

fn pipeline_not_found() -> AppError {
    AppError::not_found_code(PIPELINE_NOT_FOUND, "pipeline was not found")
}

async fn visible_pipeline_id(
    ctx: &Ctx<'_>,
    pipeline_id: Option<Uuid>,
) -> Result<Option<Uuid>, AppError> {
    let store = PipelineStore::of(ctx);
    if let Some(id) = pipeline_id {
        return match store.find_pipeline(id).await? {
            Some(pipeline) if pipeline_visible(ctx.scope, &pipeline) => Ok(Some(pipeline.id)),
            _ => Err(pipeline_not_found()),
        };
    }
    Ok(store
        .list_pipelines()
        .await?
        .into_iter()
        .filter(|pipeline| pipeline.is_active && pipeline_visible(ctx.scope, pipeline))
        .min_by_key(|pipeline| (!pipeline.is_default, pipeline.position, pipeline.id))
        .map(|pipeline| pipeline.id))
}

struct GroupCount {
    label: Option<String>,
    current: u64,
    previous: u64,
}

type GroupCounts = HashMap<Option<String>, GroupCount>;

fn grouped(result: &AggregateResult, metric: &str) -> GroupCounts {
    let Some(index) = result.metric_index(metric) else {
        return HashMap::new();
    };
    result
        .rows
        .iter()
        .filter_map(|row| {
            let group = row.groups.first()?;
            Some((
                group.value.clone(),
                GroupCount {
                    label: group.label.clone(),
                    current: whole(row.values.get(index).copied().flatten()),
                    previous: row
                        .previous
                        .as_ref()
                        .map(|values| whole(values.get(index).copied().flatten()))
                        .unwrap_or_default(),
                },
            ))
        })
        .collect()
}

fn grouped_decimal(
    result: &AggregateResult,
    metric: &str,
) -> HashMap<Option<String>, (Option<Decimal>, Option<Decimal>)> {
    let Some(index) = result.metric_index(metric) else {
        return HashMap::new();
    };
    result
        .rows
        .iter()
        .filter_map(|row| {
            let group = row.groups.first()?;
            Some((
                group.value.clone(),
                (
                    row.values.get(index).copied().flatten(),
                    row.previous
                        .as_ref()
                        .and_then(|values| values.get(index).copied().flatten()),
                ),
            ))
        })
        .collect()
}

async fn deal_plans(ctx: &Ctx<'_>) -> Result<DealPlans, AppError> {
    let scope = ctx.scope;
    let query = ctx.query();
    let definition = query.registry().require(DEAL)?;
    let visibility = scope.record_filter(&Permission::DEALS_READ)?;
    let fields = FieldSet::for_scope(query.definitions(), definition, scope).await?;
    Ok(DealPlans {
        fields,
        context: ResolveContext {
            now: ctx.now(),
            timezone: query.executor().timezone().await?,
            user_id: scope.user_id().clone(),
        },
        visibility,
    })
}

fn periods(
    plans: &DealPlans,
    period: &PeriodQuery,
    compare: bool,
) -> Result<(ResolvedPeriod, Option<ResolvedPeriod>), AppError> {
    let selection = parse_period(period, "period", "from", "to")
        .map_err(|violations| AppError::invalid_fields("invalid period", violations))?;
    let current =
        resolve_period(&selection, plans.context.now, plans.context.timezone).map_err(|_| {
            AppError::invalid_fields(
                "invalid period",
                vec![FieldViolation::new(
                    "to",
                    "must not be before from and span at most 731 days",
                )],
            )
        })?;
    let previous = compare.then(|| previous_period(&current, plans.context.timezone));
    Ok((current, previous))
}

pub(crate) async fn conversion(
    ctx: &Ctx<'_>,
    query: ConversionQuery,
) -> Result<ConversionReport, AppError> {
    let plans = deal_plans(ctx).await?;
    if plans.fields.is_hidden("stage_id") {
        return Err(not_accessible(vec![FieldViolation::new(
            "stage_id",
            NOT_ACCESSIBLE_MESSAGE,
        )]));
    }
    let pipeline_id = parse_pipeline(&query.pipeline_id)?;
    let (current, previous) = periods(&plans, &query.period, query.compare_to_previous)?;
    let reports = BuiltinReportStore::of(ctx);
    let currency = reports.currency().await?;
    let timezone = plans.context.timezone.name().to_owned();
    let amounts_visible = plans.amounts_visible();
    let pipeline = match visible_pipeline_id(ctx, pipeline_id).await? {
        Some(id) => reports.pipeline(Some(id)).await?,
        None => None,
    };
    let Some(pipeline) = pipeline else {
        if pipeline_id.is_some() {
            return Err(pipeline_not_found());
        }
        return Ok(ConversionReport {
            pipeline: None,
            currency,
            timezone,
            amounts_visible,
            current: None,
            previous: None,
        });
    };
    let snapshot = conversion_snapshot(ctx, &plans, &pipeline, current).await?;
    let previous = match previous {
        Some(period) => Some(conversion_snapshot(ctx, &plans, &pipeline, period).await?),
        None => None,
    };
    Ok(ConversionReport {
        pipeline: Some(pipeline),
        currency,
        timezone,
        amounts_visible,
        current: Some(snapshot),
        previous,
    })
}

async fn conversion_snapshot(
    ctx: &Ctx<'_>,
    plans: &DealPlans,
    pipeline: &ReportPipeline,
    period: ResolvedPeriod,
) -> Result<ConversionSnapshot, AppError> {
    let reports = BuiltinReportStore::of(ctx);
    let amounts = plans.amounts_visible();
    let money = |value: Decimal| amounts.then_some(value);
    let pipeline_condition = eq("pipeline_id", pipeline.id.to_string());
    let mut cohort = vec![pipeline_condition.clone()];
    cohort.extend(within("created_at", range_of(&period)));

    let buckets = reports
        .funnel(ctx.scope, &plans.plan(cohort.clone())?, pipeline.id)
        .await?;
    let mut lost_cohort = cohort.clone();
    lost_cohort.push(eq("status", "lost"));
    let losses = reports
        .losses_by_stage(ctx.scope, &plans.plan(lost_cohort)?, pipeline.id)
        .await?;
    let mut won_cohort = cohort.clone();
    won_cohort.push(eq("status", "won"));
    let cycle = reports
        .cycle_times(ctx.scope, &plans.plan(won_cohort)?, None)
        .await?;
    let durations = reports
        .stage_durations(
            ctx.scope,
            &plans.plan(vec![pipeline_condition])?,
            pipeline.id,
            range_of(&period),
            plans.context.now,
        )
        .await?;
    let lost_reasons = lost_reasons(ctx, &cohort, amounts).await?;

    let total = |filter: &dyn Fn(&FunnelBucket) -> bool| {
        buckets
            .iter()
            .filter(|bucket| filter(bucket))
            .fold((0u64, Decimal::ZERO), |(count, amount), bucket| {
                (count + bucket.count, amount + bucket.amount)
            })
    };
    let (created, created_amount) = total(&|_| true);
    let (open, _) = total(&|bucket| bucket.status == "open");
    let (won, won_amount) = total(&|bucket| bucket.status == "won");
    let (lost, lost_amount) = total(&|bucket| bucket.status == "lost");
    let summary = ConversionSummary {
        created,
        created_amount: money(created_amount),
        open,
        won,
        won_amount: money(won_amount),
        lost,
        lost_amount: money(lost_amount),
        conversion_rate: rate(won, created),
        win_rate: rate(won, won + lost),
        average_cycle_seconds: cycle.first().and_then(|row| row.average_seconds),
    };

    let open_stages: Vec<_> = pipeline
        .stages
        .iter()
        .filter(|stage| stage.kind == StageKind::Open)
        .collect();
    let reached: Vec<(u64, Decimal)> = open_stages
        .iter()
        .map(|stage| {
            total(&|bucket| {
                bucket.status == "won"
                    || bucket
                        .furthest_position
                        .is_some_and(|position| position >= i64::from(stage.position))
            })
        })
        .collect();
    let losses_by_stage: HashMap<Option<Uuid>, &StageLoss> =
        losses.iter().map(|loss| (loss.stage_id, loss)).collect();
    let durations_by_stage: HashMap<Uuid, &StageDuration> = durations
        .iter()
        .map(|duration| (duration.stage_id, duration))
        .collect();
    let stages = open_stages
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let (count, amount) = reached[index];
            let advanced = reached.get(index + 1).map(|next| next.0).unwrap_or(won);
            let loss = losses_by_stage.get(&Some(stage.id));
            let duration = durations_by_stage.get(&stage.id);
            StageFunnel {
                stage_id: stage.id,
                reached: count,
                reached_amount: money(amount),
                advanced,
                passage_rate: rate(advanced, count),
                lost: loss.map(|loss| loss.count).unwrap_or_default(),
                lost_amount: money(loss.map(|loss| loss.amount).unwrap_or_default()),
                completed_visits: duration.map(|row| row.completed).unwrap_or_default(),
                average_seconds: duration.and_then(|row| row.average_seconds),
                open_visits: duration.map(|row| row.open).unwrap_or_default(),
                open_average_seconds: duration.and_then(|row| row.open_average_seconds),
            }
        })
        .collect();
    let known: Vec<Uuid> = open_stages.iter().map(|stage| stage.id).collect();
    let (unattributed_lost, unattributed_amount) = losses
        .iter()
        .filter(|loss| loss.stage_id.is_none_or(|id| !known.contains(&id)))
        .fold((0u64, Decimal::ZERO), |(count, amount), loss| {
            (count + loss.count, amount + loss.amount)
        });
    Ok(ConversionSnapshot {
        period,
        summary,
        stages,
        unattributed_lost,
        unattributed_lost_amount: money(unattributed_amount),
        lost_reasons,
    })
}

async fn lost_reasons(
    ctx: &Ctx<'_>,
    cohort: &[Filter],
    amounts: bool,
) -> Result<Vec<LostReasonTotal>, AppError> {
    let mut conditions = cohort.to_vec();
    conditions.push(eq("status", "lost"));
    let mut metrics = vec![MetricSpec::count()];
    if amounts {
        metrics.push(MetricSpec::of(AggregateFunction::Sum, BASE_AMOUNT_FIELD));
    }
    let mut query = AggregateQuery::new(DEAL)
        .filter(Filter::and(conditions))
        .group(GroupBySpec::field("lost_reason_id"))
        .metrics(metrics);
    query.limit = Some(MAX_AGGREGATE_LIMIT);
    let query_ports = ctx.query();
    let result = AggregateEngine::new(&query_ports)
        .run(ctx.scope, query)
        .await?;
    let count_index = result.metric_index("count");
    let amount_index = result.metric_index(BASE_AMOUNT_METRIC);
    let mut reasons: Vec<LostReasonTotal> = result
        .rows
        .iter()
        .map(|row| {
            let group = row.groups.first();
            LostReasonTotal {
                id: group.and_then(|group| group.value.clone()),
                name: group.and_then(|group| group.label.clone()),
                count: whole(count_index.and_then(|index| row.values[index])),
                amount: amount_index.map(|index| row.values[index].unwrap_or_default()),
            }
        })
        .collect();
    reasons.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.id.is_none().cmp(&right.id.is_none()))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(reasons)
}

pub(crate) async fn performance(
    ctx: &Ctx<'_>,
    query: PerformanceQuery,
) -> Result<PerformanceReport, AppError> {
    let scope = ctx.scope;
    let plans = deal_plans(ctx).await?;
    let dimension = query.dimension.unwrap_or(PerformanceDimension::Owner);
    let pipeline_id = parse_pipeline(&query.pipeline_id)?;
    if pipeline_id.is_some() {
        visible_pipeline_id(ctx, pipeline_id).await?;
    }
    let (current, previous) = periods(&plans, &query.period, query.compare_to_previous)?;
    let compare = previous.is_some();
    let amounts = plans.amounts_visible();
    let (from, to) = (day(current.from), day(current.to));
    let mut base = Vec::new();
    if let Some(id) = pipeline_id {
        base.push(eq("pipeline_id", id.to_string()));
    }
    let query_ports = ctx.query();
    let engine = AggregateEngine::new(&query_ports);
    let run = |record_type: &str,
               conditions: Vec<Filter>,
               metrics: Vec<MetricSpec>,
               group: &str,
               field: &str| {
        let mut query = AggregateQuery::new(record_type)
            .group(GroupBySpec::field(group))
            .metrics(metrics)
            .period(AggregatePeriod::between(field, &from, &to))
            .compare(compare);
        if !conditions.is_empty() {
            query = query.filter(Filter::and(conditions));
        }
        query.limit = Some(MAX_AGGREGATE_LIMIT);
        engine.run(scope, query)
    };
    let with_status = |status: &str| {
        let mut conditions = base.clone();
        conditions.push(eq("status", status));
        conditions
    };
    let mut won_metrics = vec![MetricSpec::count()];
    if amounts {
        won_metrics.push(MetricSpec::of(AggregateFunction::Sum, BASE_AMOUNT_FIELD));
    }
    let field = dimension.field();
    let won = run(DEAL, with_status("won"), won_metrics, field, "closed_at").await?;
    let lost = run(
        DEAL,
        with_status("lost"),
        vec![MetricSpec::count()],
        field,
        "closed_at",
    )
    .await?;
    let created = run(
        DEAL,
        base.clone(),
        vec![MetricSpec::count()],
        field,
        "created_at",
    )
    .await?;
    let activities_visible = dimension.activity_field().is_some()
        && scope.filter_for(&Permission::ACTIVITIES_READ).is_some();
    let activities = match dimension.activity_field() {
        Some(group) if activities_visible => {
            let mut conditions = vec![eq("status", "done")];
            if let Some(id) = pipeline_id {
                conditions.push(eq("deal.pipeline_id", id.to_string()));
            }
            Some(
                run(
                    ACTIVITY,
                    conditions,
                    vec![MetricSpec::count()],
                    group,
                    "completed_at",
                )
                .await?,
            )
        }
        _ => None,
    };
    let won_plan = |range: InstantRange| {
        let mut conditions = with_status("won");
        conditions.extend(within("closed_at", range));
        plans.plan(conditions)
    };
    let reports = BuiltinReportStore::of(ctx);
    let cycle_rows = reports
        .cycle_times(scope, &won_plan(range_of(&current))?, Some(dimension))
        .await?;
    let cycle_total = reports
        .cycle_times(scope, &won_plan(range_of(&current))?, None)
        .await?;
    let (previous_cycle_rows, previous_cycle_total) = match &previous {
        Some(period) => (
            reports
                .cycle_times(scope, &won_plan(range_of(period))?, Some(dimension))
                .await?,
            reports
                .cycle_times(scope, &won_plan(range_of(period))?, None)
                .await?,
        ),
        None => (Vec::new(), Vec::new()),
    };

    let won_counts = grouped(&won, "count");
    let won_amounts = grouped_decimal(&won, BASE_AMOUNT_METRIC);
    let lost_counts = grouped(&lost, "count");
    let created_counts = grouped(&created, "count");
    let activity_counts = activities
        .as_ref()
        .map(|result| grouped(result, "count"))
        .unwrap_or_default();
    let cycle_of = |rows: &[CycleTime]| -> HashMap<Option<String>, Option<Decimal>> {
        rows.iter()
            .map(|row| (row.key.clone(), row.average_seconds))
            .collect()
    };
    let cycles = cycle_of(&cycle_rows);
    let previous_cycles = cycle_of(&previous_cycle_rows);

    let mut labels: BTreeMap<Option<String>, Option<String>> = BTreeMap::new();
    for map in [&won_counts, &lost_counts, &created_counts, &activity_counts] {
        for (key, group) in map {
            let entry = labels.entry(key.clone()).or_insert(None);
            if entry.is_none() {
                entry.clone_from(&group.label);
            }
        }
    }
    let metrics_for = |key: &Option<String>, previous: bool| {
        let count = |map: &GroupCounts| {
            map.get(key)
                .map(|group| {
                    if previous {
                        group.previous
                    } else {
                        group.current
                    }
                })
                .unwrap_or_default()
        };
        let won_count = count(&won_counts);
        let lost_count = count(&lost_counts);
        PerformanceMetrics {
            created: count(&created_counts),
            won: won_count,
            won_amount: amounts.then(|| {
                won_amounts
                    .get(key)
                    .and_then(|(current, before)| if previous { *before } else { *current })
                    .unwrap_or_default()
            }),
            lost: lost_count,
            conversion_rate: rate(won_count, won_count + lost_count),
            average_cycle_seconds: if previous {
                previous_cycles.get(key).copied().flatten()
            } else {
                cycles.get(key).copied().flatten()
            },
            activities_done: activities.as_ref().map(|_| count(&activity_counts)),
        }
    };
    let mut rows: Vec<PerformanceRow> = labels
        .into_iter()
        .map(|(key, label)| PerformanceRow {
            current: metrics_for(&key, false),
            previous: compare.then(|| metrics_for(&key, true)),
            key,
            label,
        })
        .collect();
    rows.sort_by(|left, right| {
        left.key
            .is_none()
            .cmp(&right.key.is_none())
            .then_with(|| {
                right
                    .current
                    .won_amount
                    .unwrap_or_default()
                    .cmp(&left.current.won_amount.unwrap_or_default())
            })
            .then_with(|| right.current.won.cmp(&left.current.won))
            .then_with(|| right.current.created.cmp(&left.current.created))
            .then_with(|| left.label.cmp(&right.label))
    });
    let totals_for = |previous: bool| {
        let pick = |result: &AggregateResult, key: &str| {
            if previous {
                result.previous_total(key)
            } else {
                result.total(key)
            }
        };
        let won_count = whole(pick(&won, "count"));
        let lost_count = whole(pick(&lost, "count"));
        PerformanceMetrics {
            created: whole(pick(&created, "count")),
            won: won_count,
            won_amount: amounts.then(|| pick(&won, BASE_AMOUNT_METRIC).unwrap_or_default()),
            lost: lost_count,
            conversion_rate: rate(won_count, won_count + lost_count),
            average_cycle_seconds: if previous {
                previous_cycle_total
                    .first()
                    .and_then(|row| row.average_seconds)
            } else {
                cycle_total.first().and_then(|row| row.average_seconds)
            },
            activities_done: activities
                .as_ref()
                .map(|result| whole(pick(result, "count"))),
        }
    };
    let truncated = won.truncated
        || lost.truncated
        || created.truncated
        || activities.as_ref().is_some_and(|result| result.truncated);
    Ok(PerformanceReport {
        dimension,
        currency: reports.currency().await?,
        timezone: plans.context.timezone.name().to_owned(),
        period: current,
        previous_period: previous,
        amounts_visible: amounts,
        activities_visible: activities.is_some(),
        totals: totals_for(false),
        previous_totals: compare.then(|| totals_for(true)),
        rows,
        truncated,
    })
}
