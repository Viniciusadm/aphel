use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use crate::application::fields::permissions::{NOT_ACCESSIBLE_MESSAGE, query_error};
use std::collections::BTreeSet;

use crate::application::fields::{
    EntityDefinition, FieldDefinitionRepository, FieldSet, RecordRelation, RecordTypeRegistry,
    ResolvedField, raw_from_json,
};
use crate::application::query::{
    ArchivedFilter, CompareOp, FieldTarget, Filter, FilterCondition, FilterExpansion,
    FilterOperator, ID_FIELD, LogicalOperator, MAX_FILTER_CONDITIONS, MAX_FILTER_DEPTH,
    MAX_LIST_VALUES, MAX_RELATIVE_DAYS, MAX_SEARCH_LENGTH, MAX_SORTS, OWNER_TEAM_FIELD,
    OperandKind, PlanCondition, PlanNode, PlanSort, PlanValue, Predicate, QueryPlan, RecordGroup,
    RecordQuery, RelatedNode, SearchPlan, SortDirection, SortSpec, escape_like,
    filter_source_field, hierarchical_target, operators, reference_set_field,
};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::domain::fields::{FieldType, normalize_phone, parse_decimal};
use crate::errors::{AppError, FieldViolation};

pub const CURRENT_USER: &str = "me";
pub const RELATION_SEPARATOR: char = '.';

#[derive(Debug, Clone, PartialEq)]
pub struct RelatedFieldSet {
    pub relation: RecordRelation,
    pub readable: Option<(FieldSet, RecordScopeFilter)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RelatedFieldSets {
    entries: Vec<RelatedFieldSet>,
}

impl RelatedFieldSets {
    pub fn new(entries: Vec<RelatedFieldSet>) -> Self {
        Self { entries }
    }

    pub fn get(&self, key: &str) -> Option<&RelatedFieldSet> {
        self.entries.iter().find(|entry| entry.relation.key == key)
    }

    pub fn entries(&self) -> &[RelatedFieldSet] {
        &self.entries
    }
}

pub async fn load_related_sets<D>(
    registry: &RecordTypeRegistry,
    definitions: &D,
    scope: &AccessScope,
    definition: &EntityDefinition,
    keys: &BTreeSet<String>,
) -> Result<RelatedFieldSets, AppError>
where
    D: FieldDefinitionRepository + ?Sized,
{
    let mut entries = Vec::new();
    for relation in &definition.relations {
        if !keys.contains(&relation.key) {
            continue;
        }
        let Some(target) = registry.get(&relation.target) else {
            continue;
        };
        let readable = match scope.filter_for(&target.read_permission) {
            Some(visibility) => Some((
                FieldSet::for_scope(definitions, target, scope).await?,
                visibility,
            )),
            None => None,
        };
        entries.push(RelatedFieldSet {
            relation: relation.clone(),
            readable,
        });
    }
    Ok(RelatedFieldSets::new(entries))
}

pub fn split_relation_path(path: &str) -> Option<(&str, &str)> {
    path.split_once(RELATION_SEPARATOR)
        .filter(|(relation, field)| !relation.is_empty() && !field.is_empty())
}

static NO_RELATIONS: std::sync::OnceLock<RelatedFieldSets> = std::sync::OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub struct ResolveContext {
    pub now: DateTime<Utc>,
    pub timezone: Tz,
    pub user_id: UserId,
}

impl ResolveContext {
    fn today(&self) -> NaiveDate {
        self.now.with_timezone(&self.timezone).date_naive()
    }

    fn start_of(&self, day: NaiveDate) -> DateTime<Utc> {
        let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
        self.timezone
            .from_local_datetime(&midnight)
            .earliest()
            .or_else(|| {
                self.timezone
                    .from_local_datetime(&(midnight + chrono::Duration::hours(1)))
                    .earliest()
            })
            .map(|local| local.with_timezone(&Utc))
            .unwrap_or_else(|| midnight.and_utc())
    }
}

pub fn resolve_query(
    fields: &FieldSet,
    context: &ResolveContext,
    visibility: RecordScopeFilter,
    query: RecordQuery,
) -> Result<QueryPlan, AppError> {
    resolve_query_with(fields, context, visibility, query, FilterExpansion::Reject)
}

pub fn resolve_query_with(
    fields: &FieldSet,
    context: &ResolveContext,
    visibility: RecordScopeFilter,
    query: RecordQuery,
    expansions: FilterExpansion<'_>,
) -> Result<QueryPlan, AppError> {
    resolve_query_related(
        fields,
        context,
        visibility,
        query,
        expansions,
        NO_RELATIONS.get_or_init(RelatedFieldSets::default),
    )
}

pub fn resolve_query_related(
    fields: &FieldSet,
    context: &ResolveContext,
    visibility: RecordScopeFilter,
    query: RecordQuery,
    expansions: FilterExpansion<'_>,
    relations: &RelatedFieldSets,
) -> Result<QueryPlan, AppError> {
    let mut violations = Vec::new();
    let filter = match &query.filter {
        Some(filter) => resolve_filter_related(
            fields,
            context,
            filter,
            expansions,
            relations,
            &mut violations,
        ),
        None => None,
    };
    let sort = resolve_sort(fields, &query.sort, &mut violations);
    let search = resolve_search(fields, query.search.as_deref(), &mut violations);
    if fields.record_type.archived_column.is_none() && query.archived == ArchivedFilter::Only {
        violations.push(FieldViolation::new(
            "archived",
            "records of this type cannot be archived",
        ));
    }
    if !violations.is_empty() {
        return Err(query_error("invalid query", violations));
    }
    Ok(QueryPlan {
        record_type: fields.record_type.clone(),
        visibility,
        filter,
        search,
        sort,
        archived: query.archived,
        window: query.window,
    })
}

pub fn validate_view_query(
    fields: &FieldSet,
    context: &ResolveContext,
    filter: Option<&Filter>,
    sort: &[SortSpec],
) -> Vec<FieldViolation> {
    validate_view_query_related(
        fields,
        context,
        filter,
        sort,
        NO_RELATIONS.get_or_init(RelatedFieldSets::default),
    )
}

pub fn validate_view_query_related(
    fields: &FieldSet,
    context: &ResolveContext,
    filter: Option<&Filter>,
    sort: &[SortSpec],
    relations: &RelatedFieldSets,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    if let Some(filter) = filter {
        resolve_filter_related(
            fields,
            context,
            filter,
            FilterExpansion::Accept,
            relations,
            &mut violations,
        );
    }
    resolve_sort(fields, sort, &mut violations);
    violations
}

pub fn resolve_filter(
    fields: &FieldSet,
    context: &ResolveContext,
    filter: &Filter,
    violations: &mut Vec<FieldViolation>,
) -> Option<PlanNode> {
    resolve_filter_with(fields, context, filter, FilterExpansion::Reject, violations)
}

pub fn resolve_filter_with(
    fields: &FieldSet,
    context: &ResolveContext,
    filter: &Filter,
    expansions: FilterExpansion<'_>,
    violations: &mut Vec<FieldViolation>,
) -> Option<PlanNode> {
    resolve_filter_related(
        fields,
        context,
        filter,
        expansions,
        NO_RELATIONS.get_or_init(RelatedFieldSets::default),
        violations,
    )
}

pub fn resolve_filter_related(
    fields: &FieldSet,
    context: &ResolveContext,
    filter: &Filter,
    expansions: FilterExpansion<'_>,
    relations: &RelatedFieldSets,
    violations: &mut Vec<FieldViolation>,
) -> Option<PlanNode> {
    let mut resolver = FilterResolver {
        fields,
        context,
        violations,
        conditions: 0,
        expansions,
        relations,
    };
    resolver.node(filter, "filter", 1)
}

struct FilterResolver<'a> {
    fields: &'a FieldSet,
    context: &'a ResolveContext,
    violations: &'a mut Vec<FieldViolation>,
    conditions: usize,
    expansions: FilterExpansion<'a>,
    relations: &'a RelatedFieldSets,
}

impl FilterResolver<'_> {
    fn violation(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.violations.push(FieldViolation::new(path, message));
    }

    fn node(&mut self, filter: &Filter, path: &str, depth: usize) -> Option<PlanNode> {
        match filter {
            Filter::Group(group) => {
                if depth > MAX_FILTER_DEPTH {
                    self.violation(
                        path,
                        format!("groups can be nested at most {MAX_FILTER_DEPTH} levels deep"),
                    );
                    return None;
                }
                let nodes: Vec<PlanNode> = group
                    .conditions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, child)| {
                        self.node(child, &format!("{path}.conditions.{index}"), depth + 1)
                    })
                    .collect();
                match nodes.len() {
                    0 => None,
                    1 => nodes.into_iter().next(),
                    _ => Some(match group.op {
                        LogicalOperator::And => PlanNode::And(nodes),
                        LogicalOperator::Or => PlanNode::Or(nodes),
                    }),
                }
            }
            Filter::Related(related) => {
                if depth > MAX_FILTER_DEPTH {
                    self.violation(
                        path,
                        format!("groups can be nested at most {MAX_FILTER_DEPTH} levels deep"),
                    );
                    return None;
                }
                self.related_node(
                    &related.related.relationship,
                    &related.related.filter,
                    &format!("{path}.related"),
                    depth,
                )
                .map(|node| match node {
                    PlanNode::Related(mut node) => {
                        node.negated = related.related.none;
                        PlanNode::Related(node)
                    }
                    other => other,
                })
            }
            Filter::Condition(condition) => {
                self.conditions += 1;
                if self.conditions == MAX_FILTER_CONDITIONS + 1 {
                    self.violation(
                        "filter",
                        format!("a filter can have at most {MAX_FILTER_CONDITIONS} conditions"),
                    );
                }
                if self.conditions > MAX_FILTER_CONDITIONS {
                    return None;
                }
                if let Some(node) = self.virtual_condition(condition, path) {
                    return node;
                }
                if self.fields.is_hidden(&condition.field) {
                    self.violation(format!("{path}.field"), NOT_ACCESSIBLE_MESSAGE);
                    return None;
                }
                let Some(field) = self.fields.find_active(&condition.field) else {
                    self.violation(format!("{path}.field"), "is not a known field");
                    return None;
                };
                if !field.filterable() {
                    self.violation(format!("{path}.field"), "cannot be used in filters");
                    return None;
                }
                let field_type = field.definition.field_type;
                if !operators::supports_definition(&field.definition, condition.operator) {
                    self.violation(
                        format!("{path}.operator"),
                        format!(
                            "{} is not supported by {} fields",
                            condition.operator.as_str(),
                            field_type.as_str()
                        ),
                    );
                    return None;
                }
                match self.condition(field, condition.operator, &condition.value) {
                    Ok((predicate, negated)) => Some(PlanNode::Condition(PlanCondition {
                        target: target_of(field),
                        field_type,
                        predicate,
                        negated,
                    })),
                    Err(message) => {
                        self.violation(format!("{path}.value"), message);
                        None
                    }
                }
            }
        }
    }

    fn virtual_condition(
        &mut self,
        condition: &FilterCondition,
        path: &str,
    ) -> Option<Option<PlanNode>> {
        if condition.operator == FilterOperator::InSavedFilter {
            return Some(self.expanded_condition(condition, path));
        }
        if self.fields.find(&condition.field).is_some() {
            return None;
        }
        if let Some((relation, field)) = split_relation_path(&condition.field) {
            return Some(self.related_condition(condition, relation, field, path));
        }
        match condition.field.as_str() {
            field if reference_set_field().is_some_and(|source| source.key == field) => {
                Some(self.reference_set_condition(condition, path))
            }
            ID_FIELD => Some(self.id_condition(condition, path)),
            OWNER_TEAM_FIELD if self.fields.record_type.owner_column.is_some() => {
                Some(self.owner_team_condition(condition, path))
            }
            _ => None,
        }
    }

    fn related_condition(
        &mut self,
        condition: &FilterCondition,
        relation: &str,
        field: &str,
        path: &str,
    ) -> Option<PlanNode> {
        let relations = self.relations;
        let Some(entry) = relations.get(relation) else {
            self.violation(format!("{path}.field"), "is not a known field");
            return None;
        };
        let Some((fields, visibility)) = &entry.readable else {
            self.violation(format!("{path}.field"), NOT_ACCESSIBLE_MESSAGE);
            return None;
        };
        let inner = Filter::Condition(FilterCondition {
            field: field.to_owned(),
            operator: condition.operator,
            value: condition.value.clone(),
        });
        let mut nested = FilterResolver {
            fields,
            context: self.context,
            violations: &mut *self.violations,
            conditions: self.conditions.saturating_sub(1),
            expansions: FilterExpansion::Reject,
            relations: NO_RELATIONS.get_or_init(RelatedFieldSets::default),
        };
        let node = nested.node(&inner, path, MAX_FILTER_DEPTH);
        self.conditions = nested.conditions;
        Some(PlanNode::Related(Box::new(RelatedNode {
            relation: entry.relation.clone(),
            target: fields.record_type.clone(),
            visibility: visibility.clone(),
            node: Box::new(node?),
            negated: false,
        })))
    }

    fn related_node(
        &mut self,
        relation: &str,
        filter: &Filter,
        path: &str,
        depth: usize,
    ) -> Option<PlanNode> {
        let relations = self.relations;
        let Some(entry) = relations.get(relation) else {
            self.violation(format!("{path}.relationship"), "is not a known relation");
            return None;
        };
        let Some((fields, visibility)) = &entry.readable else {
            self.violation(format!("{path}.relationship"), NOT_ACCESSIBLE_MESSAGE);
            return None;
        };
        let mut nested = FilterResolver {
            fields,
            context: self.context,
            violations: &mut *self.violations,
            conditions: self.conditions,
            expansions: FilterExpansion::Reject,
            relations: NO_RELATIONS.get_or_init(RelatedFieldSets::default),
        };
        let node = nested.node(filter, &format!("{path}.filter"), depth + 1);
        self.conditions = nested.conditions;
        Some(PlanNode::Related(Box::new(RelatedNode {
            relation: entry.relation.clone(),
            target: fields.record_type.clone(),
            visibility: visibility.clone(),
            node: Box::new(node.unwrap_or(PlanNode::And(Vec::new()))),
            negated: false,
        })))
    }

    fn owner_team_condition(
        &mut self,
        condition: &FilterCondition,
        path: &str,
    ) -> Option<PlanNode> {
        let (predicate, negated) = match condition.operator {
            FilterOperator::IsEmpty | FilterOperator::IsNotEmpty => {
                if !condition.value.is_null() {
                    self.violation(
                        format!("{path}.value"),
                        format!("must be omitted for {}", condition.operator.as_str()),
                    );
                    return None;
                }
                (
                    Predicate::Present,
                    condition.operator == FilterOperator::IsEmpty,
                )
            }
            FilterOperator::In | FilterOperator::NotIn => match id_list(&condition.value) {
                Ok(ids) => (
                    Predicate::In(
                        ids.into_iter()
                            .map(|id| PlanValue::Text(id.to_string()))
                            .collect(),
                    ),
                    condition.operator == FilterOperator::NotIn,
                ),
                Err(message) => {
                    self.violation(format!("{path}.value"), message);
                    return None;
                }
            },
            _ => {
                self.unsupported_operator(condition, path, OWNER_TEAM_FIELD);
                return None;
            }
        };
        Some(PlanNode::Condition(PlanCondition {
            target: FieldTarget::OwnerTeam,
            field_type: FieldType::Record,
            predicate,
            negated,
        }))
    }

    fn unsupported_operator(&mut self, condition: &FilterCondition, path: &str, kind: &str) {
        self.violation(
            format!("{path}.operator"),
            format!(
                "{} is not supported by the {kind} filter",
                condition.operator.as_str()
            ),
        );
    }

    fn reference_set_condition(
        &mut self,
        condition: &FilterCondition,
        path: &str,
    ) -> Option<PlanNode> {
        let (predicate, negated) = match condition.operator {
            FilterOperator::IsEmpty | FilterOperator::IsNotEmpty => {
                if !condition.value.is_null() {
                    self.violation(
                        format!("{path}.value"),
                        format!("must be omitted for {}", condition.operator.as_str()),
                    );
                    return None;
                }
                (
                    Predicate::Present,
                    condition.operator == FilterOperator::IsEmpty,
                )
            }
            FilterOperator::In | FilterOperator::NotIn => match id_list(&condition.value) {
                Ok(ids) => (
                    Predicate::In(ids.into_iter().map(PlanValue::OptionId).collect()),
                    condition.operator == FilterOperator::NotIn,
                ),
                Err(message) => {
                    self.violation(format!("{path}.value"), message);
                    return None;
                }
            },
            _ => {
                self.unsupported_operator(condition, path, &condition.field);
                return None;
            }
        };
        Some(PlanNode::Condition(PlanCondition {
            target: FieldTarget::ReferenceSet,
            field_type: FieldType::MultiSelect,
            predicate,
            negated,
        }))
    }

    fn id_condition(&mut self, condition: &FilterCondition, path: &str) -> Option<PlanNode> {
        let result = match condition.operator {
            FilterOperator::In | FilterOperator::NotIn => id_list(&condition.value).map(|ids| {
                Predicate::In(
                    ids.into_iter()
                        .map(|id| PlanValue::Text(id.to_string()))
                        .collect(),
                )
            }),
            FilterOperator::Eq | FilterOperator::Neq => single_id(&condition.value)
                .map(|id| Predicate::Compare(CompareOp::Eq, PlanValue::Text(id.to_string()))),
            FilterOperator::InGroup if self.fields.record_type.available_hierarchy().is_some() => {
                single_id(&condition.value).and_then(|root| {
                    RecordGroup::of(&self.fields.record_type, root)
                        .map(Predicate::InGroup)
                        .ok_or_else(|| "in_group is not available for this record type".to_owned())
                })
            }
            _ => {
                self.unsupported_operator(condition, path, ID_FIELD);
                return None;
            }
        };
        match result {
            Ok(predicate) => Some(PlanNode::Condition(PlanCondition {
                target: FieldTarget::System {
                    column: ID_FIELD.to_owned(),
                },
                field_type: FieldType::Record,
                predicate,
                negated: matches!(
                    condition.operator,
                    FilterOperator::NotIn | FilterOperator::Neq
                ),
            })),
            Err(message) => {
                self.violation(format!("{path}.value"), message);
                None
            }
        }
    }

    fn expanded_condition(&mut self, condition: &FilterCondition, path: &str) -> Option<PlanNode> {
        if filter_source_field() != Some(condition.field.as_str()) {
            self.violation(
                format!("{path}.field"),
                "must use a contributed saved-filter field",
            );
            return None;
        }
        let expansions = match self.expansions {
            FilterExpansion::Reject => {
                self.violation(
                    format!("{path}.operator"),
                    "saved-filter conditions cannot be used here",
                );
                return None;
            }
            FilterExpansion::Accept => None,
            FilterExpansion::Expand(expansions) => Some(expansions),
        };
        let id = match single_id(&condition.value) {
            Ok(id) => id,
            Err(message) => {
                self.violation(format!("{path}.value"), message);
                return None;
            }
        };
        let expansions = expansions?;
        let Some(filter) = expansions.get(&id) else {
            self.violation(format!("{path}.value"), "saved filter was not found");
            return None;
        };
        let mut nested = Vec::new();
        let node = resolve_filter_with(
            self.fields,
            self.context,
            filter,
            FilterExpansion::Reject,
            &mut nested,
        );
        if !nested.is_empty() {
            self.violation(
                format!("{path}.value"),
                "the saved filter is no longer valid",
            );
            return None;
        }
        node
    }

    fn condition(
        &self,
        field: &ResolvedField,
        operator: FilterOperator,
        value: &Value,
    ) -> Result<(Predicate, bool), String> {
        use FilterOperator::*;
        let field_type = field.definition.field_type;
        match operator.value_kind() {
            OperandKind::None if !value.is_null() => {
                return Err(format!("must be omitted for {}", operator.as_str()));
            }
            _ => {}
        }
        let today = self.context.today();
        let relative = |from: Option<NaiveDate>, until: Option<NaiveDate>| {
            Ok((self.day_range(field_type, from, until), false))
        };
        match operator {
            IsEmpty => Ok((Predicate::Present, true)),
            IsNotEmpty => Ok((Predicate::Present, false)),
            InSavedFilter => Err("in_saved_filter cannot be used with this field".to_owned()),
            InGroup => {
                let root = single_id(value)?;
                let group = hierarchical_target(&field.definition)
                    .and_then(|target| RecordGroup::of(&target, root))
                    .ok_or_else(|| "in_group cannot be used with this field".to_owned())?;
                Ok((Predicate::InGroup(group), false))
            }
            Today => relative(Some(today), Some(add_days(today, 1))),
            Yesterday => relative(Some(add_days(today, -1)), Some(today)),
            Tomorrow => relative(Some(add_days(today, 1)), Some(add_days(today, 2))),
            ThisWeek | LastWeek | NextWeek => {
                let monday = add_days(today, -i64::from(today.weekday().num_days_from_monday()));
                let start = match operator {
                    LastWeek => add_days(monday, -7),
                    NextWeek => add_days(monday, 7),
                    _ => monday,
                };
                relative(Some(start), Some(add_days(start, 7)))
            }
            ThisMonth | LastMonth | NextMonth => {
                let first = today.with_day(1).unwrap_or(today);
                let start = match operator {
                    LastMonth => first.checked_sub_months(Months::new(1)).unwrap_or(first),
                    NextMonth => first.checked_add_months(Months::new(1)).unwrap_or(first),
                    _ => first,
                };
                relative(
                    Some(start),
                    Some(start.checked_add_months(Months::new(1)).unwrap_or(start)),
                )
            }
            ThisYear => {
                let start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
                let end = NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap_or(today);
                relative(Some(start), Some(end))
            }
            WithinLastDays => {
                let days = parse_days(value)?;
                relative(Some(add_days(today, -days)), Some(add_days(today, 1)))
            }
            WithinNextDays => {
                let days = parse_days(value)?;
                relative(Some(today), Some(add_days(today, days + 1)))
            }
            OlderThanDays => {
                let days = parse_days(value)?;
                relative(None, Some(add_days(today, -days)))
            }
            Anniversary => {
                let days = parse_days(value)?;
                let offset_seconds = match field_type.base_type() {
                    FieldType::DateTime => self
                        .context
                        .now
                        .with_timezone(&self.context.timezone)
                        .offset()
                        .fix()
                        .local_minus_utc(),
                    _ => 0,
                };
                Ok((
                    Predicate::Anniversary {
                        from: today,
                        until: add_days(today, days + 1),
                        offset_seconds,
                    },
                    false,
                ))
            }
            Contains | NotContains | StartsWith => {
                let text = non_empty_text(value)?;
                let escaped = escape_like(&text);
                let pattern = if operator == StartsWith {
                    format!("{escaped}%")
                } else {
                    format!("%{escaped}%")
                };
                Ok((Predicate::Like(pattern), operator == NotContains))
            }
            In | NotIn => {
                let Value::Array(items) = value else {
                    return Err("must be a list of values".to_owned());
                };
                if items.is_empty() || items.len() > MAX_LIST_VALUES {
                    return Err(format!("must have between 1 and {MAX_LIST_VALUES} values"));
                }
                let values = items
                    .iter()
                    .map(|item| self.operand(field, item))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((Predicate::In(values), operator == NotIn))
            }
            Between => {
                let Value::Array(items) = value else {
                    return Err("must be a list with two values".to_owned());
                };
                let [low, high] = items.as_slice() else {
                    return Err("must be a list with two values".to_owned());
                };
                if field_type.base_type() == FieldType::DateTime {
                    return match (day_operand(low), day_operand(high)) {
                        (Some(low), Some(high)) => Ok((
                            self.day_range(field_type, Some(low), Some(add_days(high, 1))),
                            false,
                        )),
                        (None, None) => Ok((
                            Predicate::Between(
                                self.operand(field, low)?,
                                self.operand(field, high)?,
                            ),
                            false,
                        )),
                        _ => Err("both values must be dates or both must be date-times".to_owned()),
                    };
                }
                Ok((
                    Predicate::Between(self.operand(field, low)?, self.operand(field, high)?),
                    false,
                ))
            }
            Eq | Neq if field_type.base_type() == FieldType::Checkbox => {
                let checked = match value {
                    Value::Bool(checked) => *checked,
                    Value::String(text) if text.trim() == "true" => true,
                    Value::String(text) if text.trim() == "false" => false,
                    _ => return Err("must be true or false".to_owned()),
                };
                let negated = checked == (operator == Neq);
                Ok((
                    Predicate::Compare(CompareOp::Eq, PlanValue::Bool(true)),
                    negated,
                ))
            }
            Eq | Neq | Gt | Gte | Lt | Lte => {
                if field_type.base_type() == FieldType::DateTime
                    && let Some(day) = day_operand(value)
                {
                    let next = add_days(day, 1);
                    let (from, until) = match operator {
                        Eq | Neq => (Some(day), Some(next)),
                        Gt => (Some(next), None),
                        Gte => (Some(day), None),
                        Lt => (None, Some(day)),
                        _ => (None, Some(next)),
                    };
                    return Ok((self.day_range(field_type, from, until), operator == Neq));
                }
                let compare = match operator {
                    Gt => CompareOp::Gt,
                    Gte => CompareOp::Gte,
                    Lt => CompareOp::Lt,
                    Lte => CompareOp::Lte,
                    _ => CompareOp::Eq,
                };
                Ok((
                    Predicate::Compare(compare, self.operand(field, value)?),
                    operator == Neq,
                ))
            }
        }
    }

    fn day_range(
        &self,
        field_type: FieldType,
        from: Option<NaiveDate>,
        until: Option<NaiveDate>,
    ) -> Predicate {
        let bound = |day: NaiveDate| {
            if field_type.base_type() == FieldType::DateTime {
                PlanValue::DateTime(self.context.start_of(day))
            } else {
                PlanValue::Date(day)
            }
        };
        Predicate::Range {
            from: from.map(bound),
            until: until.map(bound),
        }
    }

    fn operand(&self, field: &ResolvedField, value: &Value) -> Result<PlanValue, String> {
        let definition = &field.definition;
        match definition.field_type.base_type() {
            FieldType::Text | FieldType::LongText | FieldType::Url | FieldType::Email => {
                Ok(PlanValue::Text(non_empty_text(value)?))
            }
            FieldType::Phone => {
                let text = non_empty_text(value)?;
                Ok(PlanValue::Text(
                    normalize_phone(&text, definition.config.phone_country()).unwrap_or(text),
                ))
            }
            FieldType::Number | FieldType::Currency => {
                let raw = raw_from_json(value).ok_or("must be a number")?;
                parse_decimal(&raw)
                    .map(PlanValue::Number)
                    .map_err(|error| error.message())
            }
            FieldType::Date => day_operand(value)
                .map(PlanValue::Date)
                .ok_or_else(|| "must be a date in the format YYYY-MM-DD".to_owned()),
            FieldType::DateTime => value
                .as_str()
                .and_then(|text| DateTime::parse_from_rfc3339(text.trim()).ok())
                .map(|instant| PlanValue::DateTime(instant.with_timezone(&Utc)))
                .ok_or_else(|| {
                    "must be a date (YYYY-MM-DD) or an RFC 3339 date and time".to_owned()
                }),
            FieldType::Checkbox => value
                .as_bool()
                .or_else(|| match value.as_str().map(str::trim) {
                    Some("true") => Some(true),
                    Some("false") => Some(false),
                    _ => None,
                })
                .map(PlanValue::Bool)
                .ok_or_else(|| "must be true or false".to_owned()),
            FieldType::Select | FieldType::MultiSelect => {
                let text = non_empty_text(value)?;
                let option = definition.option_by_value(&text);
                match (field.is_system(), option) {
                    (_, Some(option)) if !field.is_system() => Ok(PlanValue::OptionId(option.id)),
                    (true, Some(_)) => Ok(PlanValue::Text(text)),
                    (true, None) if definition.options.is_empty() => Ok(PlanValue::Text(text)),
                    _ => Err(format!("option {text:?} does not exist")),
                }
            }
            FieldType::User => {
                let text = non_empty_text(value)?;
                if text == CURRENT_USER {
                    return Ok(PlanValue::Text(self.context.user_id.as_str().to_owned()));
                }
                uuid_text(&text)
            }
            FieldType::Record | FieldType::References | FieldType::File | FieldType::Image => {
                uuid_text(&non_empty_text(value)?)
            }
            FieldType::Document => Ok(PlanValue::Text(non_empty_text(value)?)),
            FieldType::Custom { .. } => unreachable!("native field representation"),
        }
    }
}

pub fn target_of(field: &ResolvedField) -> FieldTarget {
    match &field.system {
        Some(system) => match &system.computed {
            Some(computed) => FieldTarget::Computed {
                computed: computed.clone(),
            },
            None => FieldTarget::System {
                column: system.column.clone(),
            },
        },
        None => FieldTarget::Custom {
            field_id: field.definition.id,
            column: field.definition.field_type.value_column(),
        },
    }
}

fn add_days(day: NaiveDate, days: i64) -> NaiveDate {
    if days >= 0 {
        day.checked_add_days(Days::new(days.unsigned_abs()))
    } else {
        day.checked_sub_days(Days::new(days.unsigned_abs()))
    }
    .unwrap_or(day)
}

fn day_operand(value: &Value) -> Option<NaiveDate> {
    value
        .as_str()
        .and_then(|text| NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok())
}

fn parse_days(value: &Value) -> Result<i64, String> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
        .filter(|days| (0..=MAX_RELATIVE_DAYS).contains(days))
        .ok_or_else(|| format!("must be an integer between 0 and {MAX_RELATIVE_DAYS}"))
}

fn non_empty_text(value: &Value) -> Result<String, String> {
    match value.as_str().map(str::trim) {
        Some(text) if !text.is_empty() && text.chars().count() <= 1024 => Ok(text.to_owned()),
        _ => Err("must be a non-empty string".to_owned()),
    }
}

fn single_id(value: &Value) -> Result<Uuid, String> {
    value
        .as_str()
        .and_then(|text| Uuid::parse_str(text.trim()).ok())
        .ok_or_else(|| "must be a valid id".to_owned())
}

fn id_list(value: &Value) -> Result<Vec<Uuid>, String> {
    let Value::Array(items) = value else {
        return Err("must be a list of ids".to_owned());
    };
    if items.is_empty() || items.len() > MAX_LIST_VALUES {
        return Err(format!("must have between 1 and {MAX_LIST_VALUES} values"));
    }
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let id = single_id(item)?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn uuid_text(text: &str) -> Result<PlanValue, String> {
    uuid::Uuid::parse_str(text)
        .map(|id| PlanValue::Text(id.to_string()))
        .map_err(|_| "must be a valid id".to_owned())
}

fn system_option_order(field: &ResolvedField) -> Vec<String> {
    if field.is_system() && field.definition.field_type.base_type() == FieldType::Select {
        let mut options: Vec<_> = field
            .definition
            .options
            .iter()
            .filter(|option| option.is_active)
            .collect();
        options.sort_by_key(|option| option.position);
        options
            .into_iter()
            .map(|option| option.value.clone())
            .collect()
    } else {
        Vec::new()
    }
}

fn resolve_sort(
    fields: &FieldSet,
    sort: &[SortSpec],
    violations: &mut Vec<FieldViolation>,
) -> Vec<PlanSort> {
    if sort.len() > MAX_SORTS {
        violations.push(FieldViolation::new(
            "sort",
            format!("at most {MAX_SORTS} sort fields are allowed"),
        ));
        return Vec::new();
    }
    let mut resolved = Vec::new();
    for (index, spec) in sort.iter().enumerate() {
        let path = format!("sort.{index}.field");
        match fields.find_active(&spec.field) {
            Some(field) if field.sortable() => resolved.push(PlanSort {
                target: target_of(field),
                field_type: field.definition.field_type,
                descending: spec.direction == SortDirection::Desc,
                option_order: system_option_order(field),
            }),
            Some(_) => violations.push(FieldViolation::new(path, "cannot be used to sort")),
            None if spec.field == "id" => resolved.push(PlanSort {
                target: FieldTarget::System {
                    column: "id".to_owned(),
                },
                field_type: FieldType::Text,
                descending: spec.direction == SortDirection::Desc,
                option_order: Vec::new(),
            }),
            None if fields.is_hidden(&spec.field) => {
                violations.push(FieldViolation::new(path, NOT_ACCESSIBLE_MESSAGE))
            }
            None => violations.push(FieldViolation::new(path, "is not a known field")),
        }
    }
    resolved
}

fn resolve_search(
    fields: &FieldSet,
    search: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<SearchPlan> {
    let term = search.map(str::trim).filter(|term| !term.is_empty())?;
    if term.chars().count() > MAX_SEARCH_LENGTH {
        violations.push(FieldViolation::new(
            "search",
            format!("must have at most {MAX_SEARCH_LENGTH} characters"),
        ));
        return None;
    }
    let mut plan = SearchPlan {
        pattern: format!("%{}%", escape_like(term)),
        system_columns: Vec::new(),
        custom_field_ids: Vec::new(),
    };
    for field in fields.active() {
        if !field.definition.is_searchable || !field.definition.field_type.is_textual() {
            continue;
        }
        match &field.system {
            Some(system) => {
                if let Some(column) = system.stored_column() {
                    plan.system_columns.push(column.to_owned());
                }
            }
            None => plan.custom_field_ids.push(field.definition.id),
        }
    }
    Some(plan)
}
