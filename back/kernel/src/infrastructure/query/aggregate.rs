use std::collections::HashMap;

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::fields::{RelationPath, SingleJoin};
use crate::application::query::{
    AggregateExecutor, AggregateFunction, AggregateOrderKey, AggregatePlan, AggregatePorts,
    AggregateRowData, FieldTarget, GroupLabels, GroupPlan, LabelSource, MetricPlan, MetricTarget,
    TimeGranularity, TimezoneShift,
};
use crate::domain::access::AccessScope;
use crate::domain::fields::{FieldType, ValueColumn};
use crate::errors::AppError;
use crate::infrastructure::query::MySqlRecordQueryPorts;
use crate::infrastructure::query::compiler::{
    Compiler, RECORD_ALIAS, aliased, push_live_record, push_where, quoted, tag_storage,
    taggings_source,
};
use crate::infrastructure::query::computed::computed_expression;
use crate::infrastructure::repositories::mapper;
use crate::records::constrain_query;

#[derive(Debug, Clone, Copy)]
pub struct LookupTitleSource {
    pub record_type: &'static str,
    pub table: &'static str,
    pub title_column: &'static str,
}

fn lookup_title_source(record_type: &str) -> Option<LookupTitleSource> {
    crate::modules::try_app()?
        .all_contributions::<LookupTitleSource>()
        .into_iter()
        .map(|(_, source)| *source)
        .find(|source| source.record_type == record_type)
}

#[derive(Clone)]
pub struct MySqlAggregateExecutor {
    pool: MySqlPool,
}

impl MySqlAggregateExecutor {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn group_alias(index: usize) -> String {
    format!("g{index}")
}

fn metric_alias(index: usize) -> String {
    format!("m{index}")
}

fn value_expression(group: &GroupPlan, index: usize) -> String {
    let source = match &group.related {
        Some(_) => format!("gr{index}"),
        None => RECORD_ALIAS.to_owned(),
    };
    let join = format!("gj{index}");
    match &group.target {
        FieldTarget::System { column } => aliased(&source, column),
        FieldTarget::Computed { computed } => computed_expression(computed, &source),
        FieldTarget::Custom {
            column: ValueColumn::MultiOption,
            ..
        } => format!("{join}.option_id"),
        FieldTarget::Custom { column, .. } => format!("{join}.{}", column.column_name()),
        FieldTarget::Tags => format!("{join}.tag_id"),
        FieldTarget::OwnerTeam => format!("{join}.team_id"),
    }
}

fn push_local(
    builder: &mut QueryBuilder<MySql>,
    expression: &str,
    field_type: FieldType,
    shift: &TimezoneShift,
) {
    if field_type.base_type() != FieldType::DateTime || shift.segments.is_empty() {
        builder.push(expression.to_owned());
        return;
    }
    builder.push(format!("DATE_ADD({expression}, INTERVAL "));
    match shift.segments.as_slice() {
        [single] => {
            builder.push_bind(single.seconds);
        }
        segments => {
            builder.push("(CASE");
            for segment in segments {
                match segment.until {
                    Some(until) => {
                        builder
                            .push(format!(" WHEN {expression} < "))
                            .push_bind(mapper::naive(until))
                            .push(" THEN ")
                            .push_bind(segment.seconds);
                    }
                    None => {
                        builder.push(" ELSE ").push_bind(segment.seconds);
                    }
                }
            }
            builder.push(" END)");
        }
    }
    builder.push(" SECOND)");
}

fn push_group_key(
    builder: &mut QueryBuilder<MySql>,
    group: &GroupPlan,
    index: usize,
    shift: &TimezoneShift,
) {
    let expression = value_expression(group, index);
    let Some(granularity) = group.granularity else {
        builder.push(format!("CAST({expression} AS CHAR)"));
        return;
    };
    let local = |builder: &mut QueryBuilder<MySql>| {
        push_local(builder, &expression, group.field_type, shift);
    };
    match granularity {
        TimeGranularity::Day => {
            builder.push("DATE_FORMAT(");
            local(builder);
            builder.push(", '%Y-%m-%d')");
        }
        TimeGranularity::Week => {
            builder.push("DATE_FORMAT(DATE_SUB(DATE(");
            local(builder);
            builder.push("), INTERVAL WEEKDAY(");
            local(builder);
            builder.push(") DAY), '%Y-%m-%d')");
        }
        TimeGranularity::Month => {
            builder.push("DATE_FORMAT(");
            local(builder);
            builder.push(", '%Y-%m-01')");
        }
        TimeGranularity::Quarter => {
            builder.push("CONCAT(YEAR(");
            local(builder);
            builder.push("), '-', LPAD((QUARTER(");
            local(builder);
            builder.push(") - 1) * 3 + 1, 2, '0'), '-01')");
        }
        TimeGranularity::Year => {
            builder.push("DATE_FORMAT(");
            local(builder);
            builder.push(", '%Y-01-01')");
        }
    }
}

fn metric_value(target: &MetricTarget, join: &str) -> Option<String> {
    match target {
        MetricTarget::Records => None,
        MetricTarget::System { column } => Some(aliased(RECORD_ALIAS, column)),
        MetricTarget::Computed { computed } => Some(computed_expression(computed, RECORD_ALIAS)),
        MetricTarget::Custom { column, .. } => Some(format!("{join}.{}", column.column_name())),
        MetricTarget::Duration { from, to, unit } => {
            let from = metric_value(from, join)?;
            let to = metric_value(to, &duration_join(join))?;
            Some(format!(
                "(TIMESTAMPDIFF(SECOND, {from}, {to}) / {})",
                unit.seconds()
            ))
        }
    }
}

fn duration_join(join: &str) -> String {
    format!("{join}t")
}

fn push_metric_join(
    builder: &mut QueryBuilder<MySql>,
    compiler: &Compiler<'_>,
    target: &MetricTarget,
    alias: &str,
) {
    match target {
        MetricTarget::Custom { field_id, .. } => {
            builder.push(format!(" LEFT JOIN custom_field_values {alias} ON "));
            compiler.push_value_owner(builder, alias);
            builder
                .push(format!(" AND {alias}.field_id = "))
                .push_bind(field_id.to_string());
        }
        MetricTarget::Duration { from, to, .. } => {
            push_metric_join(builder, compiler, from, alias);
            push_metric_join(builder, compiler, to, &duration_join(alias));
        }
        MetricTarget::Records | MetricTarget::System { .. } | MetricTarget::Computed { .. } => {}
    }
}

fn metric_expression(metric: &MetricPlan, index: usize) -> String {
    let value = metric_value(&metric.target, &format!("mv{index}"));
    let id = aliased(RECORD_ALIAS, "id");
    match (metric.function, value) {
        (AggregateFunction::Count, None) => format!("COUNT(DISTINCT {id})"),
        (AggregateFunction::Count, Some(value)) => {
            format!("COUNT(DISTINCT CASE WHEN {value} IS NOT NULL THEN {id} END)")
        }
        (function, Some(value)) => {
            let name = match function {
                AggregateFunction::Sum => "SUM",
                AggregateFunction::Avg => "AVG",
                AggregateFunction::Min => "MIN",
                _ => "MAX",
            };
            format!("CAST({name}({value}) AS DECIMAL(30,6))")
        }
        (_, None) => "NULL".to_owned(),
    }
}

fn push_group_join(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    compiler: &Compiler<'_>,
    group: &GroupPlan,
    index: usize,
) -> Result<(), AppError> {
    let join = format!("gj{index}");
    let (record_type, source) = match &group.related {
        Some(related) => {
            let alias = format!("gr{index}");
            let source = match &related.join {
                SingleJoin::Column(column) => aliased(RECORD_ALIAS, column),
                SingleJoin::Relationship {
                    relationship_id,
                    inverse,
                } => {
                    let link = format!("grl{index}");
                    let (source_column, target_column) = RelationPath::link_columns(*inverse);
                    builder
                        .push(format!(
                            " LEFT JOIN record_relations {link} ON {link}.relationship_id = "
                        ))
                        .push_bind(relationship_id.to_string())
                        .push(format!(" AND {link}.{source_column} = {RECORD_ALIAS}.id"));
                    format!("{link}.{target_column}")
                }
            };
            builder.push(format!(
                " LEFT JOIN {} {alias} ON {alias}.id = {source} AND ",
                quoted(&related.target.table),
            ));
            push_live_record(builder, scope, &related.target, &related.visibility, &alias);
            (related.target.record_type.as_str(), alias)
        }
        None => (compiler.record_type, RECORD_ALIAS.to_owned()),
    };
    match &group.target {
        FieldTarget::System { .. } | FieldTarget::Computed { .. } => {}
        FieldTarget::Custom { field_id, column } => {
            let table = if *column == ValueColumn::MultiOption {
                "custom_field_multi_values"
            } else {
                "custom_field_values"
            };
            builder.push(format!(" LEFT JOIN {table} {join} ON "));
            compiler.push_value_owner_of(builder, &join, record_type, &source);
            builder
                .push(format!(" AND {join}.field_id = "))
                .push_bind(field_id.to_string());
        }
        FieldTarget::Tags => {
            builder.push(format!(" LEFT JOIN {} {join} ON ", taggings_source()));
            compiler.push_value_owner_of(builder, &join, record_type, &source);
        }
        FieldTarget::OwnerTeam => {
            let owner = compiler.owner_column()?;
            builder
                .push(" LEFT JOIN (SELECT tm.user_id, tm.team_id FROM team_memberships tm")
                .push(format!(") {join} ON {join}.user_id = {owner}"));
        }
    }
    Ok(())
}

fn push_group_order(
    builder: &mut QueryBuilder<MySql>,
    group: &GroupPlan,
    index: usize,
    descending: bool,
) {
    let alias = group_alias(index);
    let direction = if descending { "DESC" } else { "ASC" };
    builder.push(format!("({alias} IS NULL) ASC, "));
    if let GroupLabels::Options(options) = &group.labels
        && !options.is_empty()
    {
        let push_position = |builder: &mut QueryBuilder<MySql>| {
            builder.push(format!("FIELD({alias}"));
            for (key, _) in options {
                builder.push(", ").push_bind(key.clone());
            }
            builder.push(")");
        };
        builder.push("(");
        push_position(builder);
        builder.push(" = 0) ASC, ");
        push_position(builder);
        builder.push(format!(" {direction}, "));
    }
    let expression = if matches!(
        group.field_type.base_type(),
        FieldType::Number | FieldType::Currency
    ) {
        format!("CAST({alias} AS DECIMAL(30,6))")
    } else {
        alias
    };
    builder.push(format!("{expression} {direction}"));
}

pub(crate) fn build_aggregate(
    scope: &AccessScope,
    plan: &AggregatePlan,
) -> Result<QueryBuilder<MySql>, AppError> {
    let definition = &plan.base.record_type;
    let compiler = Compiler::new(scope, definition);
    let mut builder = QueryBuilder::<MySql>::new("SELECT ");
    let mut first = true;
    for (index, group) in plan.groups.iter().enumerate() {
        if !first {
            builder.push(", ");
        }
        first = false;
        push_group_key(&mut builder, group, index, &plan.shift);
        builder.push(format!(" AS {}", group_alias(index)));
    }
    for (index, metric) in plan.metrics.iter().enumerate() {
        if !first {
            builder.push(", ");
        }
        first = false;
        builder.push(format!(
            "{} AS {}",
            metric_expression(metric, index),
            metric_alias(index)
        ));
    }
    builder.push(format!(
        " FROM {} {RECORD_ALIAS}",
        quoted(&definition.table)
    ));
    for (index, group) in plan.groups.iter().enumerate() {
        push_group_join(&mut builder, scope, &compiler, group, index)?;
    }
    for (index, metric) in plan.metrics.iter().enumerate() {
        push_metric_join(
            &mut builder,
            &compiler,
            &metric.target,
            &format!("mv{index}"),
        );
    }
    push_where(&mut builder, scope, &plan.base)?;
    if !plan.groups.is_empty() {
        let groups: Vec<String> = (0..plan.groups.len()).map(group_alias).collect();
        builder.push(format!(" GROUP BY {} ORDER BY ", groups.join(", ")));
        let mut ordered_groups = Vec::new();
        let mut first = true;
        let mut separate = |builder: &mut QueryBuilder<MySql>| {
            if !first {
                builder.push(", ");
            }
            first = false;
        };
        for item in &plan.order {
            match item.key {
                AggregateOrderKey::Group(index) => {
                    if let Some(group) = plan.groups.get(index) {
                        separate(&mut builder);
                        push_group_order(&mut builder, group, index, item.descending);
                        ordered_groups.push(index);
                    }
                }
                AggregateOrderKey::Metric(index) => {
                    let alias = metric_alias(index);
                    let direction = if item.descending { "DESC" } else { "ASC" };
                    separate(&mut builder);
                    builder.push(format!("({alias} IS NULL) ASC, {alias} {direction}"));
                }
            }
        }
        for (index, group) in plan.groups.iter().enumerate() {
            if !ordered_groups.contains(&index) {
                separate(&mut builder);
                push_group_order(&mut builder, group, index, false);
            }
        }
    }
    builder.push(" LIMIT ").push_bind(plan.limit);
    Ok(builder)
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[String]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.clone());
    }
    builder.push(")");
}

#[async_trait]
impl AggregateExecutor for MySqlAggregateExecutor {
    async fn aggregate(
        &self,
        scope: &AccessScope,
        plan: &AggregatePlan,
    ) -> Result<Vec<AggregateRowData>, AppError> {
        let constrained = constrain_query(&self.pool, scope, &plan.base)
            .await?
            .map(|base| AggregatePlan {
                base,
                ..plan.clone()
            });
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut builder = build_aggregate(scope, plan)?;
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let keys = (0..plan.groups.len())
                    .map(|index| row.try_get::<Option<String>, _>(group_alias(index).as_str()))
                    .collect::<Result<Vec<_>, _>>()?;
                let values = plan
                    .metrics
                    .iter()
                    .enumerate()
                    .map(|(index, metric)| {
                        let alias = metric_alias(index);
                        if metric.function == AggregateFunction::Count {
                            row.try_get::<i64, _>(alias.as_str())
                                .map(|count| Some(Decimal::from(count)))
                        } else {
                            row.try_get::<Option<Decimal>, _>(alias.as_str())
                                .map(|value| value.map(|number| number.normalize()))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(AggregateRowData { keys, values })
            })
            .collect()
    }

    async fn labels(
        &self,
        scope: &AccessScope,
        source: &LabelSource,
        ids: &[String],
    ) -> Result<HashMap<String, String>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = match source {
            LabelSource::Users => {
                QueryBuilder::<MySql>::new("SELECT u.id, u.display_name FROM users u WHERE u.id")
            }
            LabelSource::Teams => {
                QueryBuilder::<MySql>::new("SELECT r.id, r.name FROM teams r WHERE r.id")
            }
            LabelSource::Tags => {
                let Some(storage) = tag_storage() else {
                    return Ok(HashMap::new());
                };
                QueryBuilder::<MySql>::new(format!(
                    "SELECT r.id, r.name FROM {} r WHERE r.id",
                    quoted(storage.tags)
                ))
            }
            LabelSource::Records {
                record_type,
                readable,
            } => {
                let mut builder = match readable {
                    Some(readable) => {
                        let (definition, visibility) = readable.as_ref();
                        let Some(column) = definition.title_column() else {
                            return Ok(HashMap::new());
                        };
                        let mut builder = QueryBuilder::<MySql>::new(format!(
                            "SELECT r.id, r.{} FROM {} r WHERE ",
                            quoted(column),
                            quoted(&definition.table)
                        ));
                        push_live_record(&mut builder, scope, definition, visibility, RECORD_ALIAS);
                        builder
                    }
                    None => {
                        let Some(source) = lookup_title_source(record_type) else {
                            return Ok(HashMap::new());
                        };
                        QueryBuilder::<MySql>::new(format!(
                            "SELECT r.id, r.{} FROM {} r WHERE TRUE",
                            quoted(source.title_column),
                            quoted(source.table)
                        ))
                    }
                };
                builder.push(" AND r.id");
                builder
            }
        };
        push_ids(&mut builder, ids);
        let rows: Vec<(String, Option<String>)> =
            builder.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, label)| Some((id, label?)))
            .collect())
    }
}

impl AggregatePorts for MySqlRecordQueryPorts {
    type Aggregates = MySqlAggregateExecutor;

    fn aggregates(&self) -> &Self::Aggregates {
        self.aggregate_executor()
    }
}
