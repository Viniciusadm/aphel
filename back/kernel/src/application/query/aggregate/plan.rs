use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::application::fields::{ComputedColumn, EntityDefinition, RecordRelation, SingleJoin};
use crate::application::query::{
    AggregateFunction, DurationUnit, FieldTarget, QueryPlan, TimeGranularity,
};
use crate::domain::access::RecordScopeFilter;
use crate::domain::fields::{FieldType, ValueColumn};

#[derive(Debug, Clone, PartialEq)]
pub struct RelatedGroup {
    pub relation: RecordRelation,
    pub join: SingleJoin,
    pub target: EntityDefinition,
    pub visibility: RecordScopeFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupPlan {
    pub key: String,
    pub label: String,
    pub target: FieldTarget,
    pub field_type: FieldType,
    pub granularity: Option<TimeGranularity>,
    pub related: Option<RelatedGroup>,
    pub labels: GroupLabels,
}

impl GroupPlan {
    pub fn is_time(&self) -> bool {
        self.granularity.is_some()
    }

    pub fn shifts_timezone(&self) -> bool {
        self.granularity.is_some() && self.field_type.base_type() == FieldType::DateTime
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LabelSource {
    Users,
    Teams,
    Tags,
    Records {
        record_type: String,
        readable: Option<Box<(EntityDefinition, RecordScopeFilter)>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupLabels {
    Raw,
    Options(Vec<(String, String)>),
    Lookup(LabelSource),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetricTarget {
    Records,
    System {
        column: String,
    },
    Computed {
        computed: ComputedColumn,
    },
    Custom {
        field_id: Uuid,
        column: ValueColumn,
    },
    Duration {
        from: Box<MetricTarget>,
        to: Box<MetricTarget>,
        unit: DurationUnit,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricDuration {
    pub to: String,
    pub to_label: String,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricPlan {
    pub key: String,
    pub field_label: Option<String>,
    pub function: AggregateFunction,
    pub field: Option<String>,
    pub target: MetricTarget,
    pub field_type: Option<FieldType>,
    pub duration: Option<MetricDuration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateOrderKey {
    Group(usize),
    Metric(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateOrder {
    pub key: AggregateOrderKey,
    pub descending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetSegment {
    pub until: Option<DateTime<Utc>>,
    pub seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimezoneShift {
    pub segments: Vec<OffsetSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatePlan {
    pub base: QueryPlan,
    pub groups: Vec<GroupPlan>,
    pub metrics: Vec<MetricPlan>,
    pub order: Vec<AggregateOrder>,
    pub limit: u32,
    pub shift: TimezoneShift,
}

impl AggregatePlan {
    pub fn totals(&self) -> Self {
        Self {
            base: self.base.clone(),
            groups: Vec::new(),
            metrics: self.metrics.clone(),
            order: Vec::new(),
            limit: 1,
            shift: self.shift.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateRowData {
    pub keys: Vec<Option<String>>,
    pub values: Vec<Option<Decimal>>,
}
