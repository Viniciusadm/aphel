use serde::{Deserialize, Serialize};

use crate::application::query::{ArchivedFilter, Filter, SortDirection};

pub const MAX_GROUPS: usize = 3;
pub const MAX_METRICS: usize = 10;
pub const DEFAULT_AGGREGATE_LIMIT: u32 = 500;
pub const MAX_AGGREGATE_LIMIT: u32 = 1000;
pub const COUNT_METRIC: &str = "count";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeGranularity {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl TimeGranularity {
    pub const ALL: [Self; 5] = [
        Self::Day,
        Self::Week,
        Self::Month,
        Self::Quarter,
        Self::Year,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggregateFunction {
    pub const NUMERIC: [Self; 5] = [Self::Count, Self::Sum, Self::Avg, Self::Min, Self::Max];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Min => "min",
            Self::Max => "max",
        }
    }

    pub fn is_additive(&self) -> bool {
        matches!(self, Self::Count | Self::Sum)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupBySpec {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granularity: Option<TimeGranularity>,
}

impl GroupBySpec {
    pub fn field(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            granularity: None,
        }
    }

    pub fn by_time(field: impl Into<String>, granularity: TimeGranularity) -> Self {
        Self {
            field: field.into(),
            granularity: Some(granularity),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurationUnit {
    #[default]
    Days,
    Hours,
}

impl DurationUnit {
    pub const ALL: [Self; 2] = [Self::Days, Self::Hours];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Days => "days",
            Self::Hours => "hours",
        }
    }

    pub fn seconds(&self) -> u32 {
        match self {
            Self::Days => 86_400,
            Self::Hours => 3_600,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSpec {
    #[serde(rename = "fn")]
    pub function: AggregateFunction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<DurationUnit>,
}

impl MetricSpec {
    pub fn count() -> Self {
        Self {
            function: AggregateFunction::Count,
            field: None,
            to: None,
            unit: None,
        }
    }

    pub fn of(function: AggregateFunction, field: impl Into<String>) -> Self {
        Self {
            function,
            field: Some(field.into()),
            to: None,
            unit: None,
        }
    }

    pub fn duration(
        function: AggregateFunction,
        from: impl Into<String>,
        to: impl Into<String>,
        unit: DurationUnit,
    ) -> Self {
        Self {
            function,
            field: Some(from.into()),
            to: Some(to.into()),
            unit: Some(unit),
        }
    }

    pub fn key(&self) -> String {
        match (&self.field, &self.to) {
            (Some(field), Some(to)) => format!(
                "{}:{field}..{to}:{}",
                self.function.as_str(),
                self.unit.unwrap_or_default().as_str()
            ),
            (Some(field), None) => format!("{}:{field}", self.function.as_str()),
            (None, _) => self.function.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregatePeriod {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

impl AggregatePeriod {
    pub fn preset(field: impl Into<String>, preset: &str) -> Self {
        Self {
            field: field.into(),
            preset: Some(preset.to_owned()),
            from: None,
            to: None,
        }
    }

    pub fn between(field: impl Into<String>, from: &str, to: &str) -> Self {
        Self {
            field: field.into(),
            preset: Some("custom".to_owned()),
            from: Some(from.to_owned()),
            to: Some(to.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateSort {
    pub key: String,
    #[serde(default)]
    pub direction: SortDirection,
}

fn default_metrics() -> Vec<MetricSpec> {
    vec![MetricSpec::count()]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateQuery {
    pub record_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub group_by: Vec<GroupBySpec>,
    #[serde(default = "default_metrics")]
    pub metrics: Vec<MetricSpec>,
    #[serde(default)]
    pub compare_to_previous: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<AggregatePeriod>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<AggregateSort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default)]
    pub archived: ArchivedFilter,
}

impl AggregateQuery {
    pub fn new(record_type: impl Into<String>) -> Self {
        Self {
            record_type: record_type.into(),
            filter: None,
            group_by: Vec::new(),
            metrics: default_metrics(),
            compare_to_previous: false,
            period: None,
            sort: Vec::new(),
            limit: None,
            archived: ArchivedFilter::Exclude,
        }
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn group(mut self, group: GroupBySpec) -> Self {
        self.group_by.push(group);
        self
    }

    pub fn metrics(mut self, metrics: Vec<MetricSpec>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn period(mut self, period: AggregatePeriod) -> Self {
        self.period = Some(period);
        self
    }

    pub fn compare(mut self, compare: bool) -> Self {
        self.compare_to_previous = compare;
        self
    }
}
