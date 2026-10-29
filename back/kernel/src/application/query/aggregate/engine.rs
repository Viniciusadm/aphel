use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;

use crate::application::access::Authorizer;
use crate::application::fields::permissions::{NOT_ACCESSIBLE_MESSAGE, query_error};
use crate::application::fields::{FieldSet, RecordTypeDefinition, ResolvedField};
use crate::application::organization_admin::ListWindow;
use crate::application::query::{
    AggregateFieldInfo, AggregateFunction, AggregateGroupColumn, AggregateMetricColumn,
    AggregateOrder, AggregateOrderKey, AggregatePeriod, AggregatePlan, AggregateQuery,
    AggregateRange, AggregateRelationInfo, AggregateResult, AggregateRow, AggregateRowData,
    AggregateSchema, AggregateSort, AggregateTotals, COUNT_METRIC, DEFAULT_AGGREGATE_LIMIT,
    FieldTarget, Filter, FilterOperator, GroupBySpec, GroupLabels, GroupPlan, GroupValue,
    LabelSource, MAX_AGGREGATE_LIMIT, MAX_GROUPS, MAX_METRICS, MAX_SORTS, MetricDuration,
    MetricPlan, MetricSpec, MetricTarget, OWNER_TEAM_FIELD, PlanNode, QueryPlan, RecordQuery,
    RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts, RelatedFieldSets, RelatedGroup,
    ResolveContext, SegmentResolution, SortDirection, TAGS_FIELD, TimeGranularity, buckets,
    load_related_sets, resolve_filter, resolve_query_related, split_relation_path, target_of,
    timezone_shift,
};
use crate::application::security::Clock;
use crate::domain::access::AccessScope;
use crate::domain::dashboard::{
    PeriodPreset, PeriodSelection, ResolvedPeriod, previous_period, resolve_period,
};
use crate::domain::fields::FieldType;
use crate::errors::{AppError, FieldViolation};

pub const TAGS_LABEL: &str = "Etiquetas";
pub const OWNER_TEAM_LABEL: &str = "Equipe do responsável";

#[async_trait]
pub trait AggregateExecutor: Send + Sync {
    async fn aggregate(
        &self,
        scope: &AccessScope,
        plan: &AggregatePlan,
    ) -> Result<Vec<AggregateRowData>, AppError>;

    async fn labels(
        &self,
        scope: &AccessScope,
        source: &LabelSource,
        ids: &[String],
    ) -> Result<HashMap<String, String>, AppError>;
}

pub trait AggregatePorts: RecordQueryPorts {
    type Aggregates: AggregateExecutor;

    fn aggregates(&self) -> &Self::Aggregates;
}

fn groupable(field_type: FieldType) -> bool {
    field_type != FieldType::LongText
}

fn temporal(field_type: FieldType) -> bool {
    matches!(field_type, FieldType::Date | FieldType::DateTime)
}

fn numeric(field_type: FieldType) -> bool {
    matches!(field_type, FieldType::Number | FieldType::Currency)
}

fn functions_for(field_type: FieldType) -> Vec<AggregateFunction> {
    if numeric(field_type) {
        AggregateFunction::NUMERIC.to_vec()
    } else if field_type == FieldType::MultiSelect {
        Vec::new()
    } else {
        vec![AggregateFunction::Count]
    }
}

fn relation_keys(query: &AggregateQuery) -> BTreeSet<String> {
    let mut keys = query
        .filter
        .as_ref()
        .map(Filter::relation_keys)
        .unwrap_or_default();
    for group in &query.group_by {
        if let Some((relation, _)) = split_relation_path(&group.field) {
            keys.insert(relation.to_owned());
        }
    }
    keys
}

fn zeros(metrics: &[MetricPlan]) -> Vec<Option<Decimal>> {
    metrics
        .iter()
        .map(|metric| metric.function.is_additive().then_some(Decimal::ZERO))
        .collect()
}

fn normalize_key(field_type: FieldType, value: Option<String>) -> Option<String> {
    let value = value?;
    match field_type {
        FieldType::Checkbox => Some(
            match value.as_str() {
                "1" => "true",
                _ => "false",
            }
            .to_owned(),
        ),
        FieldType::Number | FieldType::Currency => Some(
            value
                .parse::<Decimal>()
                .map(|number| number.normalize().to_string())
                .unwrap_or(value),
        ),
        _ if value.is_empty() => None,
        _ => Some(value),
    }
}

fn range_of(field: &str, period: &ResolvedPeriod) -> AggregateRange {
    AggregateRange {
        field: field.to_owned(),
        preset: period.preset.as_str().to_owned(),
        from: period.from,
        to: period.to,
    }
}

struct ResolvedAggregate {
    groups: Vec<GroupPlan>,
    metrics: Vec<MetricPlan>,
    order: Vec<AggregateOrder>,
    limit: u32,
    period: Option<(String, FieldType, ResolvedPeriod)>,
    previous: Option<ResolvedPeriod>,
}

pub struct AggregateEngine<'a, P> {
    ports: &'a P,
}

impl<'a, P> AggregateEngine<'a, P>
where
    P: AggregatePorts,
{
    pub fn new(ports: &'a P) -> Self {
        Self { ports }
    }

    async fn related_sets(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        keys: &BTreeSet<String>,
    ) -> Result<RelatedFieldSets, AppError> {
        load_related_sets(
            self.ports.registry(),
            self.ports.definitions(),
            scope,
            definition,
            keys,
        )
        .await
    }

    pub async fn describe(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<AggregateSchema, AppError> {
        let definition = self.ports.registry().require(record_type)?;
        scope.record_filter(&definition.read_permission)?;
        let fields = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        let mut infos: Vec<AggregateFieldInfo> = fields
            .active()
            .map(|field| {
                let field_type = field.definition.field_type;
                AggregateFieldInfo {
                    key: field.key().to_owned(),
                    label: field.definition.label.clone(),
                    field_type,
                    is_virtual: false,
                    groupable: groupable(field_type),
                    temporal: temporal(field_type),
                    functions: functions_for(field_type),
                }
            })
            .collect();
        if fields.find(TAGS_FIELD).is_none() {
            infos.push(AggregateFieldInfo {
                key: TAGS_FIELD.to_owned(),
                label: TAGS_LABEL.to_owned(),
                field_type: FieldType::MultiSelect,
                is_virtual: true,
                groupable: true,
                temporal: false,
                functions: Vec::new(),
            });
        }
        if definition.owner_column.is_some() && fields.find(OWNER_TEAM_FIELD).is_none() {
            infos.push(AggregateFieldInfo {
                key: OWNER_TEAM_FIELD.to_owned(),
                label: OWNER_TEAM_LABEL.to_owned(),
                field_type: FieldType::Record,
                is_virtual: true,
                groupable: true,
                temporal: false,
                functions: Vec::new(),
            });
        }
        let relations = definition
            .relations
            .iter()
            .map(|relation| AggregateRelationInfo {
                key: relation.key.clone(),
                label: relation.label.clone(),
                target: relation.target.as_str().to_owned(),
                single: relation.is_single(),
                readable: self
                    .ports
                    .registry()
                    .get(&relation.target)
                    .is_some_and(|target| scope.filter_for(&target.read_permission).is_some()),
            })
            .collect();
        Ok(AggregateSchema {
            record_type: definition.record_type.as_str().to_owned(),
            fields: infos,
            relations,
        })
    }

    pub async fn run(
        &self,
        scope: &AccessScope,
        query: AggregateQuery,
    ) -> Result<AggregateResult, AppError> {
        let definition = self.ports.registry().require(&query.record_type)?;
        let visibility = scope.record_filter(&definition.read_permission)?;
        let fields = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        let now = self.ports.clock().now();
        let timezone = self.ports.executor().timezone().await?;
        let context = ResolveContext {
            now,
            timezone,
            user_id: scope.user_id().clone(),
        };
        let relations = self
            .related_sets(scope, definition, &relation_keys(&query))
            .await?;
        let resolved = self.resolve(scope, &fields, &relations, &query, now, timezone)?;
        let segments = RecordQueryEngine::new(self.ports)
            .segments(&query.record_type, query.filter.as_ref())
            .await?;
        let base = resolve_query_related(
            &fields,
            &context,
            visibility,
            RecordQuery {
                filter: query.filter.clone(),
                sort: Vec::new(),
                search: None,
                archived: query.archived,
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
            },
            SegmentResolution::Expand(&segments),
            &relations,
        )?;
        let current_base = match &resolved.period {
            Some((field, field_type, period)) => {
                with_period(&fields, &context, &base, field, *field_type, period)?
            }
            None => base.clone(),
        };
        let shift = if resolved.groups.iter().any(GroupPlan::shifts_timezone) {
            let (from, until) = shift_window(&resolved, now);
            timezone_shift(timezone, from, until)
        } else {
            Default::default()
        };
        let limit = resolved.limit;
        let plan = AggregatePlan {
            base: current_base,
            groups: resolved.groups.clone(),
            metrics: resolved.metrics.clone(),
            order: resolved.order.clone(),
            limit: limit + 1,
            shift,
        };
        let executor = self.ports.aggregates();
        let mut rows = executor.aggregate(scope, &plan).await?;
        let truncated = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let totals = self.single_row(scope, &plan.totals()).await?;
        let mut previous_rows = None;
        let mut previous_totals = None;
        if let (Some((field, field_type, _)), Some(previous)) =
            (&resolved.period, &resolved.previous)
        {
            let previous_plan = AggregatePlan {
                base: with_period(&fields, &context, &base, field, *field_type, previous)?,
                limit: MAX_AGGREGATE_LIMIT + 1,
                ..plan.clone()
            };
            previous_rows = Some(executor.aggregate(scope, &previous_plan).await?);
            previous_totals = Some(self.single_row(scope, &previous_plan.totals()).await?);
        }
        let rows = self.assemble(
            &resolved,
            normalize_rows(&resolved.groups, &resolved.metrics, rows),
            previous_rows.map(|rows| normalize_rows(&resolved.groups, &resolved.metrics, rows)),
            truncated,
        );
        let rows = self.label_rows(scope, &resolved.groups, rows).await?;
        Ok(AggregateResult {
            record_type: definition.record_type.as_str().to_owned(),
            timezone: timezone.name().to_owned(),
            groups: resolved
                .groups
                .iter()
                .map(|group| AggregateGroupColumn {
                    key: group.key.clone(),
                    label: group.label.clone(),
                    field_type: group.field_type,
                    granularity: group.granularity,
                })
                .collect(),
            metrics: resolved
                .metrics
                .iter()
                .map(|metric| AggregateMetricColumn {
                    key: metric.key.clone(),
                    function: metric.function,
                    field: metric.field.clone(),
                    field_label: metric.field_label.clone(),
                    field_type: metric.field_type,
                    duration: metric.duration.clone(),
                })
                .collect(),
            rows,
            totals: AggregateTotals {
                values: totals,
                previous: previous_totals,
            },
            period: resolved
                .period
                .as_ref()
                .map(|(field, _, period)| range_of(field, period)),
            previous_period: match (&resolved.period, &resolved.previous) {
                (Some((field, _, _)), Some(previous)) => Some(range_of(field, previous)),
                _ => None,
            },
            truncated,
        })
    }

    async fn single_row(
        &self,
        scope: &AccessScope,
        plan: &AggregatePlan,
    ) -> Result<Vec<Option<Decimal>>, AppError> {
        Ok(self
            .ports
            .aggregates()
            .aggregate(scope, plan)
            .await?
            .into_iter()
            .next()
            .map(|row| additive_zeros(&plan.metrics, row.values))
            .unwrap_or_else(|| zeros(&plan.metrics)))
    }

    fn resolve(
        &self,
        scope: &AccessScope,
        fields: &FieldSet,
        relations: &RelatedFieldSets,
        query: &AggregateQuery,
        now: DateTime<Utc>,
        timezone: Tz,
    ) -> Result<ResolvedAggregate, AppError> {
        let mut violations = Vec::new();
        if query.group_by.len() > MAX_GROUPS {
            violations.push(FieldViolation::new(
                "group_by",
                format!("at most {MAX_GROUPS} groups are allowed"),
            ));
        }
        if query.metrics.is_empty() || query.metrics.len() > MAX_METRICS {
            violations.push(FieldViolation::new(
                "metrics",
                format!("must have between 1 and {MAX_METRICS} metrics"),
            ));
        }
        let limit = query.limit.unwrap_or(DEFAULT_AGGREGATE_LIMIT);
        if limit == 0 || limit > MAX_AGGREGATE_LIMIT {
            violations.push(FieldViolation::new(
                "limit",
                format!("must be between 1 and {MAX_AGGREGATE_LIMIT}"),
            ));
        }
        let groups: Vec<GroupPlan> = query
            .group_by
            .iter()
            .take(MAX_GROUPS)
            .enumerate()
            .filter_map(|(index, spec)| {
                self.group(
                    scope,
                    fields,
                    relations,
                    spec,
                    &format!("group_by.{index}"),
                    &mut violations,
                )
            })
            .collect();
        let mut metrics: Vec<MetricPlan> = Vec::new();
        for (index, spec) in query.metrics.iter().take(MAX_METRICS).enumerate() {
            let path = format!("metrics.{index}");
            if let Some(metric) = metric(fields, spec, &path, &mut violations) {
                if metrics.iter().any(|existing| existing.key == metric.key) {
                    violations.push(FieldViolation::new(path, "is repeated"));
                } else {
                    metrics.push(metric);
                }
            }
        }
        let order = order(&query.sort, &groups, &metrics, &mut violations);
        let period = query
            .period
            .as_ref()
            .and_then(|period| resolve_period_spec(fields, period, now, timezone, &mut violations));
        let previous = match (&period, query.compare_to_previous) {
            (Some((_, _, period)), true) => Some(previous_period(period, timezone)),
            (None, true) => {
                violations.push(FieldViolation::new(
                    "compare_to_previous",
                    "requires a period",
                ));
                None
            }
            _ => None,
        };
        if !violations.is_empty() {
            return Err(query_error("invalid aggregate query", violations));
        }
        Ok(ResolvedAggregate {
            groups,
            metrics,
            order,
            limit,
            period,
            previous,
        })
    }

    fn group(
        &self,
        scope: &AccessScope,
        fields: &FieldSet,
        relations: &RelatedFieldSets,
        spec: &GroupBySpec,
        path: &str,
        violations: &mut Vec<FieldViolation>,
    ) -> Option<GroupPlan> {
        let field_path = format!("{path}.field");
        let Some((relation, key)) = split_relation_path(&spec.field) else {
            return self.field_group(scope, fields, &spec.field, spec, path, true, violations);
        };
        let Some(entry) = relations.get(relation) else {
            violations.push(FieldViolation::new(field_path, "is not a known field"));
            return None;
        };
        let Some(join) = entry.relation.single_join() else {
            violations.push(FieldViolation::new(
                field_path,
                "only fields of a single related record can be used as groups",
            ));
            return None;
        };
        let Some((related_fields, visibility)) = &entry.readable else {
            violations.push(FieldViolation::new(field_path, NOT_ACCESSIBLE_MESSAGE));
            return None;
        };
        let mut group =
            self.field_group(scope, related_fields, key, spec, path, false, violations)?;
        group.key = spec.field.clone();
        group.label = format!("{} › {}", entry.relation.label, group.label);
        group.related = Some(RelatedGroup {
            relation: entry.relation.clone(),
            join,
            target: related_fields.record_type.clone(),
            visibility: visibility.clone(),
        });
        Some(group)
    }

    #[allow(clippy::too_many_arguments)]
    fn field_group(
        &self,
        scope: &AccessScope,
        fields: &FieldSet,
        key: &str,
        spec: &GroupBySpec,
        path: &str,
        allow_virtual: bool,
        violations: &mut Vec<FieldViolation>,
    ) -> Option<GroupPlan> {
        let field_path = format!("{path}.field");
        let virtual_group = |target, field_type, label: &str, labels| {
            Some(GroupPlan {
                key: key.to_owned(),
                label: label.to_owned(),
                target,
                field_type,
                granularity: None,
                related: None,
                labels,
            })
        };
        if fields.find(key).is_none() && allow_virtual {
            if key == TAGS_FIELD {
                return virtual_group(
                    FieldTarget::Tags,
                    FieldType::MultiSelect,
                    TAGS_LABEL,
                    GroupLabels::Lookup(LabelSource::Tags),
                );
            }
            if key == OWNER_TEAM_FIELD && fields.record_type.owner_column.is_some() {
                return virtual_group(
                    FieldTarget::OwnerTeam,
                    FieldType::Record,
                    OWNER_TEAM_LABEL,
                    GroupLabels::Lookup(LabelSource::Teams),
                );
            }
        }
        if fields.is_hidden(key) {
            violations.push(FieldViolation::new(field_path, NOT_ACCESSIBLE_MESSAGE));
            return None;
        }
        let Some(field) = fields.find_active(key) else {
            violations.push(FieldViolation::new(field_path, "is not a known field"));
            return None;
        };
        let field_type = field.definition.field_type;
        if !groupable(field_type) {
            violations.push(FieldViolation::new(field_path, "cannot be used as a group"));
            return None;
        }
        let granularity = if temporal(field_type) {
            Some(spec.granularity.unwrap_or(TimeGranularity::Day))
        } else {
            if spec.granularity.is_some() {
                violations.push(FieldViolation::new(
                    format!("{path}.granularity"),
                    "can only be used with date fields",
                ));
            }
            None
        };
        Some(GroupPlan {
            key: key.to_owned(),
            label: field.definition.label.clone(),
            target: target_of(field),
            field_type,
            granularity,
            related: None,
            labels: self.labels_of(scope, field),
        })
    }

    fn labels_of(&self, scope: &AccessScope, field: &ResolvedField) -> GroupLabels {
        let definition = &field.definition;
        match definition.field_type {
            FieldType::Select | FieldType::MultiSelect => {
                let mut ordered: Vec<_> = definition.options.iter().collect();
                ordered.sort_by_key(|option| option.position);
                let mut options: Vec<(String, String)> = ordered
                    .into_iter()
                    .map(|option| {
                        let key = if field.is_system() {
                            option.value.clone()
                        } else {
                            option.id.to_string()
                        };
                        (key, option.label.clone())
                    })
                    .collect();
                if let Some(system) = &field.system {
                    for option in &system.options {
                        if !options.iter().any(|(key, _)| *key == option.value) {
                            options.push((option.value.clone(), option.label.clone()));
                        }
                    }
                }
                GroupLabels::Options(options)
            }
            FieldType::User => GroupLabels::Lookup(LabelSource::Users),
            FieldType::Record => {
                let Some(record_type) = &definition.config.target_record_type else {
                    return GroupLabels::Raw;
                };
                let readable = match self.ports.registry().require(record_type) {
                    Ok(target) => match scope.filter_for(&target.read_permission) {
                        Some(visibility) => Some(Box::new((target.clone(), visibility))),
                        None => return GroupLabels::Raw,
                    },
                    Err(_) => None,
                };
                GroupLabels::Lookup(LabelSource::Records {
                    record_type: record_type.clone(),
                    readable,
                })
            }
            _ => GroupLabels::Raw,
        }
    }

    fn assemble(
        &self,
        resolved: &ResolvedAggregate,
        rows: Vec<AggregateRowData>,
        previous: Option<Vec<AggregateRowData>>,
        truncated: bool,
    ) -> Vec<AggregateRow> {
        let metrics = &resolved.metrics;
        let time_group = match (resolved.groups.as_slice(), &resolved.period) {
            ([group], Some((field, _, _)))
                if group.is_time() && group.related.is_none() && group.key == *field =>
            {
                group.granularity
            }
            _ => None,
        };
        let mut previous_by_key: HashMap<Vec<Option<String>>, Vec<Option<Decimal>>> =
            HashMap::new();
        let mut previous_order: Vec<Vec<Option<String>>> = Vec::new();
        if let Some(previous) = previous {
            let mapping = match (
                &resolved.period,
                &resolved.previous,
                resolved.groups.as_slice(),
            ) {
                (Some((field, _, current)), Some(before), groups) => groups
                    .iter()
                    .map(|group| {
                        let granularity = group
                            .granularity
                            .filter(|_| group.related.is_none() && group.key == *field)?;
                        Some(
                            buckets(before.from, before.to, granularity)
                                .into_iter()
                                .zip(buckets(current.from, current.to, granularity))
                                .collect::<HashMap<String, String>>(),
                        )
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            'rows: for row in previous {
                let mut keys = Vec::with_capacity(row.keys.len());
                for (index, key) in row.keys.into_iter().enumerate() {
                    match (mapping.get(index).and_then(Option::as_ref), key) {
                        (Some(map), Some(key)) => match map.get(&key) {
                            Some(mapped) => keys.push(Some(mapped.clone())),
                            None => continue 'rows,
                        },
                        (_, key) => keys.push(key),
                    }
                }
                if !previous_by_key.contains_key(&keys) {
                    previous_order.push(keys.clone());
                }
                previous_by_key.insert(keys, row.values);
            }
        }
        let comparing = resolved.previous.is_some();
        let mut assembled: Vec<AggregateRow> = Vec::with_capacity(rows.len());
        let mut seen: BTreeSet<Vec<Option<String>>> = BTreeSet::new();
        for row in rows {
            let previous = comparing.then(|| {
                previous_by_key
                    .get(&row.keys)
                    .cloned()
                    .unwrap_or_else(|| zeros(metrics))
            });
            seen.insert(row.keys.clone());
            assembled.push(AggregateRow {
                groups: row
                    .keys
                    .into_iter()
                    .map(|value| GroupValue { value, label: None })
                    .collect(),
                values: row.values,
                previous,
            });
        }
        if comparing && !truncated {
            for keys in previous_order {
                if seen.contains(&keys) {
                    continue;
                }
                let values = previous_by_key.remove(&keys).unwrap_or_default();
                seen.insert(keys.clone());
                assembled.push(AggregateRow {
                    groups: keys
                        .into_iter()
                        .map(|value| GroupValue { value, label: None })
                        .collect(),
                    values: zeros(metrics),
                    previous: Some(values),
                });
            }
        }
        let sorted_by_group = resolved
            .order
            .first()
            .is_none_or(|order| order.key == AggregateOrderKey::Group(0));
        if let (Some(granularity), Some((_, _, period)), false, true) =
            (time_group, &resolved.period, truncated, sorted_by_group)
        {
            for key in buckets(period.from, period.to, granularity) {
                let keys = vec![Some(key)];
                if seen.contains(&keys) {
                    continue;
                }
                assembled.push(AggregateRow {
                    groups: keys
                        .into_iter()
                        .map(|value| GroupValue { value, label: None })
                        .collect(),
                    values: zeros(metrics),
                    previous: comparing.then(|| zeros(metrics)),
                });
            }
            let descending = resolved.order.first().is_some_and(|order| order.descending);
            assembled.sort_by(|left, right| {
                let ordering = left.groups[0].value.cmp(&right.groups[0].value);
                if descending {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }
        assembled
    }

    async fn label_rows(
        &self,
        scope: &AccessScope,
        groups: &[GroupPlan],
        mut rows: Vec<AggregateRow>,
    ) -> Result<Vec<AggregateRow>, AppError> {
        for (index, group) in groups.iter().enumerate() {
            let labels: HashMap<String, String> = match &group.labels {
                GroupLabels::Raw => continue,
                GroupLabels::Options(options) => options
                    .iter()
                    .map(|(key, label)| (key.clone(), label.clone()))
                    .collect(),
                GroupLabels::Lookup(source) => {
                    let ids: Vec<String> = rows
                        .iter()
                        .filter_map(|row| row.groups.get(index)?.value.clone())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect();
                    if ids.is_empty() {
                        continue;
                    }
                    self.ports.aggregates().labels(scope, source, &ids).await?
                }
            };
            for row in &mut rows {
                if let Some(value) = row.groups.get_mut(index) {
                    value.label = value
                        .value
                        .as_ref()
                        .and_then(|key| labels.get(key).cloned());
                }
            }
        }
        Ok(rows)
    }
}

fn additive_zeros(metrics: &[MetricPlan], values: Vec<Option<Decimal>>) -> Vec<Option<Decimal>> {
    values
        .into_iter()
        .zip(metrics)
        .map(|(value, metric)| match value {
            None if metric.function.is_additive() => Some(Decimal::ZERO),
            other => other,
        })
        .collect()
}

fn normalize_rows(
    groups: &[GroupPlan],
    metrics: &[MetricPlan],
    rows: Vec<AggregateRowData>,
) -> Vec<AggregateRowData> {
    rows.into_iter()
        .map(|row| AggregateRowData {
            keys: row
                .keys
                .into_iter()
                .enumerate()
                .map(|(index, key)| match groups.get(index) {
                    Some(group) => normalize_key(group.field_type, key),
                    None => key,
                })
                .collect(),
            values: additive_zeros(metrics, row.values),
        })
        .collect()
}

fn shift_window(
    resolved: &ResolvedAggregate,
    now: DateTime<Utc>,
) -> (DateTime<Utc>, DateTime<Utc>) {
    match &resolved.period {
        Some((_, _, period)) => {
            let starts_at = resolved
                .previous
                .as_ref()
                .map_or(period.starts_at, |previous| {
                    previous.starts_at.min(period.starts_at)
                });
            (
                starts_at - Duration::days(2),
                period.ends_at + Duration::days(2),
            )
        }
        None => (
            NaiveDate::from_ymd_opt(1990, 1, 1)
                .and_then(|day| day.and_hms_opt(0, 0, 0))
                .map(|start| start.and_utc())
                .unwrap_or(now),
            now + Duration::days(3650),
        ),
    }
}

fn with_period(
    fields: &FieldSet,
    context: &ResolveContext,
    base: &QueryPlan,
    field: &str,
    field_type: FieldType,
    period: &ResolvedPeriod,
) -> Result<QueryPlan, AppError> {
    let day = |date: NaiveDate| serde_json::Value::String(date.format("%Y-%m-%d").to_string());
    let condition = Filter::condition(
        field,
        FilterOperator::Between,
        serde_json::Value::Array(vec![day(period.from), day(period.to)]),
    );
    let mut violations = Vec::new();
    let node = resolve_filter(fields, context, &condition, &mut violations);
    if !violations.is_empty() {
        return Err(query_error("invalid aggregate period", violations));
    }
    let Some(node) = node else {
        return Err(AppError::Internal(format!(
            "period on {field} ({}) did not resolve",
            field_type.as_str()
        )));
    };
    let mut plan = base.clone();
    plan.filter = Some(match plan.filter.take() {
        Some(PlanNode::And(mut nodes)) => {
            nodes.push(node);
            PlanNode::And(nodes)
        }
        Some(other) => PlanNode::And(vec![other, node]),
        None => node,
    });
    Ok(plan)
}

fn resolve_period_spec(
    fields: &FieldSet,
    period: &AggregatePeriod,
    now: DateTime<Utc>,
    timezone: Tz,
    violations: &mut Vec<FieldViolation>,
) -> Option<(String, FieldType, ResolvedPeriod)> {
    let field_type = if fields.is_hidden(&period.field) {
        violations.push(FieldViolation::new("period.field", NOT_ACCESSIBLE_MESSAGE));
        None
    } else {
        match fields.find_active(&period.field) {
            Some(field) if temporal(field.definition.field_type) && field.filterable() => {
                Some(field.definition.field_type)
            }
            Some(_) => {
                violations.push(FieldViolation::new(
                    "period.field",
                    "must be a date or date and time field",
                ));
                None
            }
            None => {
                violations.push(FieldViolation::new("period.field", "is not a known field"));
                None
            }
        }
    };
    let preset = match period.preset.as_deref() {
        None if period.from.is_some() || period.to.is_some() => Some(PeriodPreset::Custom),
        None => Some(PeriodPreset::Month),
        Some(value) => {
            let preset = PeriodPreset::parse(value.trim());
            if preset.is_none() {
                violations.push(FieldViolation::new(
                    "period.preset",
                    "is not a known period",
                ));
            }
            preset
        }
    };
    let day = |value: Option<&String>, path: &str, violations: &mut Vec<FieldViolation>| {
        let value = value?;
        let parsed = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok();
        if parsed.is_none() {
            violations.push(FieldViolation::new(
                path,
                "must be a date in the format YYYY-MM-DD",
            ));
        }
        parsed
    };
    let from = day(period.from.as_ref(), "period.from", violations);
    let to = day(period.to.as_ref(), "period.to", violations);
    let field_type = field_type?;
    let selection = match preset? {
        PeriodPreset::Custom => match (from, to) {
            (Some(from), Some(to)) => PeriodSelection::custom(from, to),
            _ => {
                violations.push(FieldViolation::new(
                    "period",
                    "a custom period requires from and to",
                ));
                return None;
            }
        },
        preset => PeriodSelection::preset(preset),
    };
    match resolve_period(&selection, now, timezone) {
        Ok(resolved) => Some((period.field.clone(), field_type, resolved)),
        Err(_) => {
            violations.push(FieldViolation::new(
                "period",
                "must be a valid range of at most 731 days",
            ));
            None
        }
    }
}

fn metric_field<'f>(
    fields: &'f FieldSet,
    key: &str,
    field_path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<&'f ResolvedField> {
    if split_relation_path(key).is_some() {
        violations.push(FieldViolation::new(
            field_path,
            "metrics can only use fields of the record type itself",
        ));
        return None;
    }
    if fields.is_hidden(key) {
        violations.push(FieldViolation::new(field_path, NOT_ACCESSIBLE_MESSAGE));
        return None;
    }
    let Some(field) = fields.find_active(key) else {
        violations.push(FieldViolation::new(field_path, "is not a known field"));
        return None;
    };
    Some(field)
}

fn metric_target(
    field: &ResolvedField,
    field_path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<MetricTarget> {
    match target_of(field) {
        FieldTarget::System { column } => Some(MetricTarget::System { column }),
        FieldTarget::Computed { computed } => Some(MetricTarget::Computed { computed }),
        FieldTarget::Custom { field_id, column } => Some(MetricTarget::Custom { field_id, column }),
        FieldTarget::Tags | FieldTarget::OwnerTeam => {
            violations.push(FieldViolation::new(
                field_path,
                "cannot be used as a metric",
            ));
            None
        }
    }
}

fn duration_metric(
    fields: &FieldSet,
    spec: &MetricSpec,
    (from_key, to_key): (&str, &str),
    path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<MetricPlan> {
    let from_path = format!("{path}.field");
    let to_path = format!("{path}.to");
    let from = metric_field(fields, from_key, &from_path, violations);
    let to = metric_field(fields, to_key, &to_path, violations);
    let (from, to) = (from?, to?);
    let mut valid = true;
    for (field, field_path) in [(from, &from_path), (to, &to_path)] {
        if !temporal(field.definition.field_type) {
            violations.push(FieldViolation::new(
                field_path.as_str(),
                "must be a date or date and time field to measure a duration",
            ));
            valid = false;
        }
    }
    if from_key == to_key {
        violations.push(FieldViolation::new(
            to_path.as_str(),
            "must be a different field than field",
        ));
        valid = false;
    }
    let from_target = metric_target(from, &from_path, violations);
    let to_target = metric_target(to, &to_path, violations);
    let (from_target, to_target) = (from_target?, to_target?);
    if !valid {
        return None;
    }
    let unit = spec.unit.unwrap_or_default();
    Some(MetricPlan {
        key: spec.key(),
        field_label: Some(from.definition.label.clone()),
        function: spec.function,
        field: Some(from_key.to_owned()),
        target: MetricTarget::Duration {
            from: Box::new(from_target),
            to: Box::new(to_target),
            unit,
        },
        field_type: Some(FieldType::Number),
        duration: Some(MetricDuration {
            to: to_key.to_owned(),
            to_label: to.definition.label.clone(),
            unit,
        }),
    })
}

fn metric(
    fields: &FieldSet,
    spec: &MetricSpec,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<MetricPlan> {
    let field_path = format!("{path}.field");
    let Some(key) = spec.field.as_deref() else {
        if spec.function != AggregateFunction::Count {
            violations.push(FieldViolation::new(field_path, "is required"));
            return None;
        }
        if spec.to.is_some() || spec.unit.is_some() {
            violations.push(FieldViolation::new(
                field_path,
                "is required to measure a duration",
            ));
            return None;
        }
        return Some(MetricPlan {
            key: COUNT_METRIC.to_owned(),
            field_label: None,
            function: AggregateFunction::Count,
            field: None,
            target: MetricTarget::Records,
            field_type: None,
            duration: None,
        });
    };
    if let Some(to) = spec.to.as_deref() {
        return duration_metric(fields, spec, (key, to), path, violations);
    }
    if spec.unit.is_some() {
        violations.push(FieldViolation::new(
            format!("{path}.unit"),
            "is only accepted together with to",
        ));
        return None;
    }
    let field = metric_field(fields, key, &field_path, violations)?;
    let field_type = field.definition.field_type;
    if !functions_for(field_type).contains(&spec.function) {
        violations.push(FieldViolation::new(
            format!("{path}.fn"),
            format!(
                "{} is not supported by {} fields",
                spec.function.as_str(),
                field_type.as_str()
            ),
        ));
        return None;
    }
    let target = metric_target(field, &field_path, violations)?;
    Some(MetricPlan {
        key: spec.key(),
        field_label: Some(field.definition.label.clone()),
        function: spec.function,
        field: Some(key.to_owned()),
        target,
        field_type: Some(field_type),
        duration: None,
    })
}

fn order(
    sort: &[AggregateSort],
    groups: &[GroupPlan],
    metrics: &[MetricPlan],
    violations: &mut Vec<FieldViolation>,
) -> Vec<AggregateOrder> {
    if sort.len() > MAX_SORTS {
        violations.push(FieldViolation::new(
            "sort",
            format!("at most {MAX_SORTS} sort keys are allowed"),
        ));
        return Vec::new();
    }
    let mut order = Vec::new();
    for (index, spec) in sort.iter().enumerate() {
        let key = if let Some(position) = groups.iter().position(|group| group.key == spec.key) {
            AggregateOrderKey::Group(position)
        } else if let Some(position) = metrics.iter().position(|metric| metric.key == spec.key) {
            AggregateOrderKey::Metric(position)
        } else {
            violations.push(FieldViolation::new(
                format!("sort.{index}.key"),
                "must be the field of a group or the key of a metric",
            ));
            continue;
        };
        order.push(AggregateOrder {
            key,
            descending: spec.direction == SortDirection::Desc,
        });
    }
    order
}
