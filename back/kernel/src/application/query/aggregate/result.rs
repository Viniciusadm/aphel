use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::application::query::{AggregateFunction, MetricDuration, TimeGranularity};
use crate::domain::fields::FieldType;

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateGroupColumn {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub granularity: Option<TimeGranularity>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateMetricColumn {
    pub key: String,
    pub function: AggregateFunction,
    pub field: Option<String>,
    pub field_label: Option<String>,
    pub field_type: Option<FieldType>,
    pub duration: Option<MetricDuration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupValue {
    pub value: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateRow {
    pub groups: Vec<GroupValue>,
    pub values: Vec<Option<Decimal>>,
    pub previous: Option<Vec<Option<Decimal>>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AggregateTotals {
    pub values: Vec<Option<Decimal>>,
    pub previous: Option<Vec<Option<Decimal>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateRange {
    pub field: String,
    pub preset: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateResult {
    pub record_type: String,
    pub timezone: String,
    pub groups: Vec<AggregateGroupColumn>,
    pub metrics: Vec<AggregateMetricColumn>,
    pub rows: Vec<AggregateRow>,
    pub totals: AggregateTotals,
    pub period: Option<AggregateRange>,
    pub previous_period: Option<AggregateRange>,
    pub truncated: bool,
}

impl AggregateResult {
    pub fn metric_index(&self, key: &str) -> Option<usize> {
        self.metrics.iter().position(|metric| metric.key == key)
    }

    pub fn total(&self, key: &str) -> Option<Decimal> {
        self.metric_index(key)
            .and_then(|index| self.totals.values.get(index).copied().flatten())
    }

    pub fn previous_total(&self, key: &str) -> Option<Decimal> {
        let index = self.metric_index(key)?;
        self.totals
            .previous
            .as_ref()
            .and_then(|values| values.get(index).copied().flatten())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateFieldInfo {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub is_virtual: bool,
    pub groupable: bool,
    pub temporal: bool,
    pub functions: Vec<AggregateFunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateRelationInfo {
    pub key: String,
    pub label: String,
    pub target: String,
    pub single: bool,
    pub readable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateSchema {
    pub record_type: String,
    pub fields: Vec<AggregateFieldInfo>,
    pub relations: Vec<AggregateRelationInfo>,
}
