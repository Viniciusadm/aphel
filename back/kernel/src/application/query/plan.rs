use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::application::fields::{ComputedColumn, EntityDefinition, RecordRelation};
use crate::application::organization_admin::ListWindow;
use crate::application::query::ArchivedFilter;
use crate::domain::access::RecordScopeFilter;
use crate::domain::fields::{FieldType, ValueColumn};

#[derive(Debug, Clone, PartialEq)]
pub enum PlanValue {
    Text(String),
    Number(Decimal),
    Date(NaiveDate),
    DateTime(DateTime<Utc>),
    Bool(bool),
    OptionId(Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl CompareOp {
    pub fn sql(&self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordGroup {
    pub root: Uuid,
    pub table: String,
    pub parent_column: String,
    pub deleted_column: Option<String>,
}

impl RecordGroup {
    pub fn of(definition: &EntityDefinition, root: Uuid) -> Option<Self> {
        let hierarchy = definition.available_hierarchy()?;
        Some(Self {
            root,
            table: definition.table.clone(),
            parent_column: hierarchy.parent_column.clone(),
            deleted_column: definition.deleted_column.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Compare(CompareOp, PlanValue),
    Between(PlanValue, PlanValue),
    Range {
        from: Option<PlanValue>,
        until: Option<PlanValue>,
    },
    In(Vec<PlanValue>),
    Like(String),
    Present,
    InGroup(RecordGroup),
    Anniversary {
        from: NaiveDate,
        until: NaiveDate,
        offset_seconds: i32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldTarget {
    System { column: String },
    Computed { computed: ComputedColumn },
    Custom { field_id: Uuid, column: ValueColumn },
    Tags,
    OwnerTeam,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanCondition {
    pub target: FieldTarget,
    pub field_type: FieldType,
    pub predicate: Predicate,
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelatedNode {
    pub relation: RecordRelation,
    pub target: EntityDefinition,
    pub visibility: RecordScopeFilter,
    pub node: Box<PlanNode>,
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlanNode {
    And(Vec<PlanNode>),
    Or(Vec<PlanNode>),
    Condition(PlanCondition),
    Related(Box<RelatedNode>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanSort {
    pub target: FieldTarget,
    pub field_type: FieldType,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchPlan {
    pub pattern: String,
    pub system_columns: Vec<String>,
    pub custom_field_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryPlan {
    pub record_type: EntityDefinition,
    pub visibility: RecordScopeFilter,
    pub filter: Option<PlanNode>,
    pub search: Option<SearchPlan>,
    pub sort: Vec<PlanSort>,
    pub archived: ArchivedFilter,
    pub window: ListWindow,
}

pub const LIKE_ESCAPE: char = '!';

pub fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '%' | '_' | LIKE_ESCAPE) {
            escaped.push(LIKE_ESCAPE);
        }
        escaped.push(character);
    }
    escaped
}
