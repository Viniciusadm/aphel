use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::application::organization_admin::ListWindow;

pub const MAX_FILTER_DEPTH: usize = 4;
pub const MAX_FILTER_CONDITIONS: usize = 50;
pub const MAX_LIST_VALUES: usize = 100;
pub const MAX_SORTS: usize = 3;
pub const MAX_SEARCH_LENGTH: usize = 100;
pub const MAX_RELATIVE_DAYS: i64 = 3650;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq,
    Neq,
    Contains,
    NotContains,
    StartsWith,
    Gt,
    Gte,
    Lt,
    Lte,
    Between,
    In,
    NotIn,
    IsEmpty,
    IsNotEmpty,
    Today,
    Yesterday,
    Tomorrow,
    ThisWeek,
    LastWeek,
    NextWeek,
    ThisMonth,
    LastMonth,
    NextMonth,
    ThisYear,
    WithinLastDays,
    WithinNextDays,
    OlderThanDays,
    Anniversary,
    InSegment,
    InGroup,
}

impl FilterOperator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Neq => "neq",
            Self::Contains => "contains",
            Self::NotContains => "not_contains",
            Self::StartsWith => "starts_with",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Between => "between",
            Self::In => "in",
            Self::NotIn => "not_in",
            Self::IsEmpty => "is_empty",
            Self::IsNotEmpty => "is_not_empty",
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::Tomorrow => "tomorrow",
            Self::ThisWeek => "this_week",
            Self::LastWeek => "last_week",
            Self::NextWeek => "next_week",
            Self::ThisMonth => "this_month",
            Self::LastMonth => "last_month",
            Self::NextMonth => "next_month",
            Self::ThisYear => "this_year",
            Self::WithinLastDays => "within_last_days",
            Self::WithinNextDays => "within_next_days",
            Self::OlderThanDays => "older_than_days",
            Self::Anniversary => "anniversary",
            Self::InSegment => "in_segment",
            Self::InGroup => "in_group",
        }
    }

    pub fn value_kind(&self) -> OperandKind {
        match self {
            Self::IsEmpty
            | Self::IsNotEmpty
            | Self::Today
            | Self::Yesterday
            | Self::Tomorrow
            | Self::ThisWeek
            | Self::LastWeek
            | Self::NextWeek
            | Self::ThisMonth
            | Self::LastMonth
            | Self::NextMonth
            | Self::ThisYear => OperandKind::None,
            Self::Between => OperandKind::Pair,
            Self::In | Self::NotIn => OperandKind::List,
            Self::WithinLastDays
            | Self::WithinNextDays
            | Self::OlderThanDays
            | Self::Anniversary => OperandKind::Days,
            _ => OperandKind::Single,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandKind {
    None,
    Single,
    Pair,
    List,
    Days,
}

impl OperandKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Single => "single",
            Self::Pair => "pair",
            Self::List => "list",
            Self::Days => "days",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Group(FilterGroup),
    Condition(FilterCondition),
    Related(FilterRelated),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterRelated {
    pub related: RelatedFilter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedFilter {
    pub relationship: String,
    pub filter: Box<Filter>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub none: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterGroup {
    pub op: LogicalOperator,
    pub conditions: Vec<Filter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterCondition {
    pub field: String,
    pub operator: FilterOperator,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub value: Value,
}

impl Filter {
    pub fn and(conditions: Vec<Filter>) -> Self {
        Self::Group(FilterGroup {
            op: LogicalOperator::And,
            conditions,
        })
    }

    pub fn or(conditions: Vec<Filter>) -> Self {
        Self::Group(FilterGroup {
            op: LogicalOperator::Or,
            conditions,
        })
    }

    pub fn related(relationship: impl Into<String>, filter: Filter) -> Self {
        Self::Related(FilterRelated {
            related: RelatedFilter {
                relationship: relationship.into(),
                filter: Box::new(filter),
                none: false,
            },
        })
    }

    pub fn relation_keys(&self) -> std::collections::BTreeSet<String> {
        fn visit(filter: &Filter, keys: &mut std::collections::BTreeSet<String>) {
            match filter {
                Filter::Group(group) => {
                    for child in &group.conditions {
                        visit(child, keys);
                    }
                }
                Filter::Condition(condition) => {
                    if let Some((relation, _)) = condition
                        .field
                        .split_once('.')
                        .filter(|(relation, field)| !relation.is_empty() && !field.is_empty())
                    {
                        keys.insert(relation.to_owned());
                    }
                }
                Filter::Related(related) => {
                    keys.insert(related.related.relationship.clone());
                }
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        visit(self, &mut keys);
        keys
    }

    pub fn condition(field: impl Into<String>, operator: FilterOperator, value: Value) -> Self {
        Self::Condition(FilterCondition {
            field: field.into(),
            operator,
            value,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SortSpec {
    pub field: String,
    #[serde(default)]
    pub direction: SortDirection,
}

impl SortSpec {
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Asc,
        }
    }

    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Desc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivedFilter {
    #[default]
    Exclude,
    Include,
    Only,
}

impl ArchivedFilter {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "exclude" => Some(Self::Exclude),
            "include" => Some(Self::Include),
            "only" => Some(Self::Only),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordQuery {
    pub filter: Option<Filter>,
    pub sort: Vec<SortSpec>,
    pub search: Option<String>,
    pub archived: ArchivedFilter,
    pub window: ListWindow,
}

impl Default for RecordQuery {
    fn default() -> Self {
        Self {
            filter: None,
            sort: Vec::new(),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: 0,
                limit: 25,
            },
        }
    }
}

pub fn parse_sort(value: &str) -> Vec<SortSpec> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| match item.strip_prefix('-') {
            Some(field) => SortSpec::desc(field),
            None => SortSpec::asc(item.strip_prefix('+').unwrap_or(item)),
        })
        .collect()
}
