use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::fields::FieldTypeSchema;
use crate::api::dto::query::{FilterNodeSchema, SortDirectionSchema};
use crate::application::query::{
    AggregateFieldInfo, AggregateFunction, AggregateGroupColumn, AggregateMetricColumn,
    AggregatePeriod, AggregateQuery, AggregateRange, AggregateRelationInfo, AggregateResult,
    AggregateRow, AggregateSchema, AggregateSort, ArchivedFilter, Filter, GroupBySpec, MetricSpec,
    TimeGranularity,
};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = TimeGranularity)]
pub enum TimeGranularitySchema {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AggregateFunction)]
pub enum AggregateFunctionSchema {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = AggregateGroupBy)]
pub struct GroupBySchema {
    #[schema(example = "closed_at")]
    pub field: String,
    pub granularity: Option<TimeGranularitySchema>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AggregateDurationUnit)]
pub enum DurationUnitSchema {
    Days,
    Hours,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = AggregateMetric)]
pub struct MetricSchema {
    #[serde(rename = "fn")]
    #[schema(rename = "fn")]
    pub function: AggregateFunctionSchema,
    #[schema(example = "amount")]
    pub field: Option<String>,
    #[schema(example = "closed_at")]
    pub to: Option<String>,
    pub unit: Option<DurationUnitSchema>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = AggregatePeriod)]
pub struct AggregatePeriodSchema {
    #[schema(example = "closed_at")]
    pub field: String,
    #[schema(example = "month")]
    pub preset: Option<String>,
    #[schema(example = "2026-01-01")]
    pub from: Option<String>,
    #[schema(example = "2026-03-31")]
    pub to: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = AggregateSort)]
pub struct AggregateSortSchema {
    #[schema(example = "sum:amount")]
    pub key: String,
    pub direction: Option<SortDirectionSchema>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AggregateQueryRequest {
    #[schema(example = "deal")]
    pub record_type: String,
    #[serde(default)]
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub filter: Option<Filter>,
    #[serde(default)]
    #[schema(value_type = Vec<GroupBySchema>)]
    pub group_by: Vec<GroupBySpec>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<MetricSchema>>)]
    pub metrics: Option<Vec<MetricSpec>>,
    #[serde(default)]
    pub compare_to_previous: bool,
    #[serde(default)]
    #[schema(value_type = Option<AggregatePeriodSchema>)]
    pub period: Option<AggregatePeriod>,
    #[serde(default)]
    #[schema(value_type = Vec<AggregateSortSchema>)]
    pub sort: Vec<AggregateSort>,
    #[schema(example = 500)]
    pub limit: Option<u32>,
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "exclude")]
    pub archived: Option<ArchivedFilter>,
}

impl AggregateQueryRequest {
    pub fn into_query(self) -> AggregateQuery {
        let mut query = AggregateQuery::new(self.record_type);
        query.filter = self.filter;
        query.group_by = self.group_by;
        if let Some(metrics) = self.metrics {
            query.metrics = metrics;
        }
        query.compare_to_previous = self.compare_to_previous;
        query.period = self.period;
        query.sort = self.sort;
        query.limit = self.limit;
        query.archived = self.archived.unwrap_or_default();
        query
    }
}

impl From<AggregateQuery> for AggregateQueryRequest {
    fn from(query: AggregateQuery) -> Self {
        Self {
            record_type: query.record_type,
            filter: query.filter,
            group_by: query.group_by,
            metrics: Some(query.metrics),
            compare_to_previous: query.compare_to_previous,
            period: query.period,
            sort: query.sort,
            limit: query.limit,
            archived: Some(query.archived),
        }
    }
}

pub fn decimal(value: Option<Decimal>) -> Option<String> {
    value.map(|value| value.normalize().to_string())
}

fn decimals(values: Vec<Option<Decimal>>) -> Vec<Option<String>> {
    values.into_iter().map(decimal).collect()
}

fn granularity(value: Option<TimeGranularity>) -> Option<String> {
    value.map(|value| value.as_str().to_owned())
}

fn function(value: AggregateFunction) -> String {
    value.as_str().to_owned()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateGroupColumnResponse {
    #[schema(example = "closed_at")]
    pub key: String,
    #[schema(example = "Closed at")]
    pub label: String,
    #[schema(value_type = FieldTypeSchema)]
    pub field_type: String,
    #[schema(value_type = Option<TimeGranularitySchema>)]
    pub granularity: Option<String>,
}

impl From<AggregateGroupColumn> for AggregateGroupColumnResponse {
    fn from(column: AggregateGroupColumn) -> Self {
        Self {
            key: column.key,
            label: column.label,
            field_type: column.field_type.as_str().to_owned(),
            granularity: granularity(column.granularity),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateMetricColumnResponse {
    #[schema(example = "sum:amount")]
    pub key: String,
    #[serde(rename = "fn")]
    #[schema(value_type = AggregateFunctionSchema, rename = "fn")]
    pub function: String,
    #[schema(example = "amount")]
    pub field: Option<String>,
    #[schema(example = "Amount")]
    pub field_label: Option<String>,
    #[schema(value_type = Option<FieldTypeSchema>)]
    pub field_type: Option<String>,
    #[schema(example = "closed_at")]
    pub to: Option<String>,
    #[schema(example = "Closed at")]
    pub to_label: Option<String>,
    #[schema(value_type = Option<DurationUnitSchema>)]
    pub unit: Option<String>,
}

impl From<AggregateMetricColumn> for AggregateMetricColumnResponse {
    fn from(column: AggregateMetricColumn) -> Self {
        let (to, to_label, unit) = match column.duration {
            Some(duration) => (
                Some(duration.to),
                Some(duration.to_label),
                Some(duration.unit.as_str().to_owned()),
            ),
            None => (None, None, None),
        };
        Self {
            to,
            to_label,
            unit,
            key: column.key,
            function: function(column.function),
            field: column.field,
            field_label: column.field_label,
            field_type: column
                .field_type
                .map(|field_type| field_type.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateGroupValueResponse {
    #[schema(example = "2026-10-01")]
    pub value: Option<String>,
    #[schema(example = "Ana Souza")]
    pub label: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateRowResponse {
    pub groups: Vec<AggregateGroupValueResponse>,
    #[schema(example = json!(["12", "45000.5"]))]
    pub values: Vec<Option<String>>,
    pub previous: Option<Vec<Option<String>>>,
}

impl From<AggregateRow> for AggregateRowResponse {
    fn from(row: AggregateRow) -> Self {
        Self {
            groups: row
                .groups
                .into_iter()
                .map(|group| AggregateGroupValueResponse {
                    value: group.value,
                    label: group.label,
                })
                .collect(),
            values: decimals(row.values),
            previous: row.previous.map(decimals),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateTotalsResponse {
    pub values: Vec<Option<String>>,
    pub previous: Option<Vec<Option<String>>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateRangeResponse {
    #[schema(example = "closed_at")]
    pub field: String,
    #[schema(example = "month")]
    pub preset: String,
    #[schema(example = "2026-10-01")]
    pub from: String,
    #[schema(example = "2026-10-31")]
    pub to: String,
}

impl From<AggregateRange> for AggregateRangeResponse {
    fn from(range: AggregateRange) -> Self {
        Self {
            field: range.field,
            preset: range.preset,
            from: range.from.format("%Y-%m-%d").to_string(),
            to: range.to.format("%Y-%m-%d").to_string(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateResultResponse {
    #[schema(example = "deal")]
    pub record_type: String,
    #[schema(example = "America/Sao_Paulo")]
    pub timezone: String,
    pub groups: Vec<AggregateGroupColumnResponse>,
    pub metrics: Vec<AggregateMetricColumnResponse>,
    pub rows: Vec<AggregateRowResponse>,
    pub totals: AggregateTotalsResponse,
    pub period: Option<AggregateRangeResponse>,
    pub previous_period: Option<AggregateRangeResponse>,
    pub truncated: bool,
}

impl From<AggregateResult> for AggregateResultResponse {
    fn from(result: AggregateResult) -> Self {
        Self {
            record_type: result.record_type,
            timezone: result.timezone,
            groups: result.groups.into_iter().map(Into::into).collect(),
            metrics: result.metrics.into_iter().map(Into::into).collect(),
            rows: result.rows.into_iter().map(Into::into).collect(),
            totals: AggregateTotalsResponse {
                values: decimals(result.totals.values),
                previous: result.totals.previous.map(decimals),
            },
            period: result.period.map(Into::into),
            previous_period: result.previous_period.map(Into::into),
            truncated: result.truncated,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateFieldResponse {
    #[schema(example = "amount")]
    pub key: String,
    #[schema(example = "Amount")]
    pub label: String,
    #[schema(value_type = FieldTypeSchema)]
    pub field_type: String,
    pub is_virtual: bool,
    pub groupable: bool,
    pub temporal: bool,
    #[schema(value_type = Vec<AggregateFunctionSchema>)]
    pub functions: Vec<String>,
}

impl From<AggregateFieldInfo> for AggregateFieldResponse {
    fn from(field: AggregateFieldInfo) -> Self {
        Self {
            key: field.key,
            label: field.label,
            field_type: field.field_type.as_str().to_owned(),
            is_virtual: field.is_virtual,
            groupable: field.groupable,
            temporal: field.temporal,
            functions: field.functions.into_iter().map(function).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateRelationResponse {
    #[schema(example = "company")]
    pub key: String,
    #[schema(example = "Company")]
    pub label: String,
    #[schema(example = "company")]
    pub target_record_type: String,
    pub single: bool,
    pub readable: bool,
}

impl From<AggregateRelationInfo> for AggregateRelationResponse {
    fn from(relation: AggregateRelationInfo) -> Self {
        Self {
            key: relation.key,
            label: relation.label,
            target_record_type: relation.target,
            single: relation.single,
            readable: relation.readable,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateSchemaResponse {
    #[schema(example = "deal")]
    pub record_type: String,
    #[schema(value_type = Vec<TimeGranularitySchema>)]
    pub granularities: Vec<String>,
    pub fields: Vec<AggregateFieldResponse>,
    pub relations: Vec<AggregateRelationResponse>,
}

impl From<AggregateSchema> for AggregateSchemaResponse {
    fn from(schema: AggregateSchema) -> Self {
        Self {
            record_type: schema.record_type,
            granularities: TimeGranularity::ALL
                .iter()
                .map(|value| value.as_str().to_owned())
                .collect(),
            fields: schema.fields.into_iter().map(Into::into).collect(),
            relations: schema.relations.into_iter().map(Into::into).collect(),
        }
    }
}
