use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::api::error::ApiError;
use crate::api::extractors::Pagination;
use crate::application::organization_admin::ListWindow;
use crate::application::query::{
    ArchivedFilter, Filter, FilterOperator, OperandKind, RecordQuery, parse_sort,
};
use crate::errors::{AppError, FieldViolation};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = FilterOperator)]
pub enum FilterOperatorSchema {
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
    InSavedFilter,
    InGroup,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = OperandKind)]
pub enum OperandKindSchema {
    None,
    Single,
    Pair,
    List,
    Days,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = LogicalOperator)]
pub enum LogicalOperatorSchema {
    And,
    Or,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = SortDirection)]
pub enum SortDirectionSchema {
    Asc,
    Desc,
}

#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
#[schema(as = FilterNode)]
pub enum FilterNodeSchema {
    Group(FilterGroupSchema),
    Condition(FilterConditionSchema),
    Related(FilterRelatedSchema),
}

#[derive(Deserialize, ToSchema)]
#[schema(as = FilterRelated)]
pub struct FilterRelatedSchema {
    pub related: RelatedFilterSchema,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = RelatedFilter)]
pub struct RelatedFilterSchema {
    #[schema(example = "organization_profile.organization")]
    pub relationship: String,
    #[schema(no_recursion)]
    pub filter: Box<FilterNodeSchema>,
    #[serde(default)]
    pub none: bool,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = FilterGroup)]
pub struct FilterGroupSchema {
    pub op: LogicalOperatorSchema,
    #[schema(no_recursion)]
    pub conditions: Vec<FilterNodeSchema>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = FilterCondition)]
pub struct FilterConditionSchema {
    #[schema(example = "name")]
    pub field: String,
    pub operator: FilterOperatorSchema,
    #[schema(value_type = Option<Value>)]
    pub value: Option<Value>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = SortSpec)]
pub struct SortSpecSchema {
    #[schema(example = "created_at")]
    pub field: String,
    pub direction: Option<SortDirectionSchema>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RecordQueryParams {
    #[param(
        example = r#"{"op":"and","conditions":[{"field":"name","operator":"contains","value":"ana"}]}"#
    )]
    pub filter: Option<String>,
    #[param(example = "-created_at,name")]
    pub sort: Option<String>,
    pub search: Option<String>,
    #[param(example = "exclude")]
    pub archived: Option<String>,
}

const MAX_BRACKET_FILTERS: usize = 50;

pub fn bracket_filters(raw_query: Option<&str>) -> Result<Vec<Filter>, Vec<FieldViolation>> {
    let mut filters = Vec::new();
    let mut violations = Vec::new();
    for (key, value) in form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let Some(path) = key.strip_prefix("filter[") else {
            continue;
        };
        let mut parts = path.split("][");
        let field = parts.next().unwrap_or_default().trim_end_matches(']');
        let operator = parts.next().map(|operator| operator.trim_end_matches(']'));
        if field.is_empty() || parts.next().is_some() {
            violations.push(FieldViolation::new(
                key.to_string(),
                "must be filter[field] or filter[field][operator]",
            ));
            continue;
        }
        let operator = match operator {
            None => Some(FilterOperator::Eq),
            Some(name) => {
                serde_json::from_value::<FilterOperator>(Value::String(name.to_owned())).ok()
            }
        };
        let Some(operator) = operator else {
            violations.push(FieldViolation::new(
                key.to_string(),
                "unknown filter operator",
            ));
            continue;
        };
        filters.push(Filter::condition(
            field,
            operator,
            bracket_value(operator, value.trim()),
        ));
    }
    if filters.len() > MAX_BRACKET_FILTERS {
        violations.push(FieldViolation::new(
            "filter",
            format!("at most {MAX_BRACKET_FILTERS} filter[...] conditions are allowed"),
        ));
    }
    if violations.is_empty() {
        Ok(filters)
    } else {
        Err(violations)
    }
}

fn bracket_value(operator: FilterOperator, value: &str) -> Value {
    let list = || {
        Value::Array(
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| Value::String(item.to_owned()))
                .collect(),
        )
    };
    match operator.value_kind() {
        OperandKind::None => Value::Null,
        OperandKind::List | OperandKind::Pair => list(),
        OperandKind::Days | OperandKind::Single => Value::String(value.to_owned()),
    }
}

pub struct RecordListQuery(pub RecordQuery);

impl<S> FromRequestParts<S> for RecordListQuery
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pagination = Pagination::from_request_parts(parts, state).await?;
        let Query(params) = Query::<RecordQueryParams>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| AppError::Validation(rejection.body_text()))?;
        let raw_query = parts.uri.query().map(str::to_owned);
        Ok(Self(
            params.into_query_with(pagination, raw_query.as_deref())?,
        ))
    }
}

impl RecordQueryParams {
    pub fn into_query(self, pagination: Pagination) -> Result<RecordQuery, AppError> {
        self.into_query_with(pagination, None)
    }

    pub fn into_query_with(
        self,
        pagination: Pagination,
        raw_query: Option<&str>,
    ) -> Result<RecordQuery, AppError> {
        let mut violations = Vec::new();
        let brackets = bracket_filters(raw_query).unwrap_or_else(|errors| {
            violations.extend(errors);
            Vec::new()
        });
        let filter = match self.filter.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(raw) => match serde_json::from_str::<Filter>(raw) {
                Ok(filter) => Some(filter),
                Err(_) => {
                    violations.push(FieldViolation::new(
                        "filter",
                        "must be a filter group or condition encoded as JSON",
                    ));
                    None
                }
            },
        };
        let archived = match self.archived.as_deref().map(str::trim) {
            None | Some("") => ArchivedFilter::Exclude,
            Some(value) => ArchivedFilter::parse(value).unwrap_or_else(|| {
                violations.push(FieldViolation::new(
                    "archived",
                    "must be exclude, include or only",
                ));
                ArchivedFilter::Exclude
            }),
        };
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid query", violations));
        }
        let filter = match (filter, brackets.is_empty()) {
            (filter, true) => filter,
            (None, false) => Some(Filter::and(brackets)),
            (Some(filter), false) => {
                let mut conditions = vec![filter];
                conditions.extend(brackets);
                Some(Filter::and(conditions))
            }
        };
        Ok(RecordQuery {
            filter,
            sort: self.sort.as_deref().map(parse_sort).unwrap_or_default(),
            search: self.search,
            archived,
            window: ListWindow {
                offset: pagination.offset(),
                limit: pagination.limit(),
            },
        })
    }
}
