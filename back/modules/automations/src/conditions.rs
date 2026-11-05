use std::collections::HashMap;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value};

use aphel_contract_automation::application::context::lookup;
use aphel_contract_automation::application::templates::TRIGGER_PATHS;
use aphel_kernel::errors::FieldViolation;
use aphel_sdk::conditions::{ConditionContext, condition_matches};
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::query::{
    Filter, FilterCondition, FilterGroup, LogicalOperator,
};
use aphel_sdk::platform::domain::fields::FieldType;

pub(crate) const TRIGGER_FIELD_PREFIX: &str = "trigger.";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Partial {
    Constant(bool),
    Record(Filter),
}

pub(crate) struct TriggerEvaluation<'a> {
    pub(crate) trigger: &'a Value,
    pub(crate) fields: &'a FieldSet,
    pub(crate) now: DateTime<Utc>,
    pub(crate) timezone: Tz,
    pub(crate) user_id: &'a str,
}

fn trigger_path(condition: &FilterCondition) -> Option<&str> {
    condition.field.strip_prefix(TRIGGER_FIELD_PREFIX)
}

impl TriggerEvaluation<'_> {
    pub(crate) fn partial(&self, filter: &Filter) -> Partial {
        match filter {
            Filter::Condition(condition) => match trigger_path(condition) {
                Some(path) => Partial::Constant(self.condition(condition, path)),
                None => Partial::Record(filter.clone()),
            },
            Filter::Related(_) => Partial::Record(filter.clone()),
            Filter::Group(group) => self.group(group),
        }
    }

    fn group(&self, group: &FilterGroup) -> Partial {
        let decisive = group.op == LogicalOperator::Or;
        let mut remaining = Vec::new();
        for child in &group.conditions {
            match self.partial(child) {
                Partial::Constant(value) if value == decisive => {
                    return Partial::Constant(decisive);
                }
                Partial::Constant(_) => {}
                Partial::Record(filter) => remaining.push(filter),
            }
        }
        match remaining.len() {
            0 => Partial::Constant(!decisive || group.conditions.is_empty()),
            1 => Partial::Record(remaining.remove(0)),
            _ => Partial::Record(Filter::Group(FilterGroup {
                op: group.op,
                conditions: remaining,
            })),
        }
    }

    fn condition(&self, condition: &FilterCondition, path: &str) -> bool {
        let segments: Vec<&str> = path.split('.').collect();
        let value = lookup(self.trigger, &segments)
            .cloned()
            .unwrap_or(Value::Null);
        let field_type = match segments.as_slice() {
            ["previous", key] => self
                .fields
                .find(key)
                .map(|field| field.definition.field_type)
                .unwrap_or(FieldType::Text),
            ["occurred_at"] => FieldType::DateTime,
            _ => FieldType::Text,
        };
        let mut values = Map::new();
        values.insert(condition.field.clone(), value);
        let context = ConditionContext::new(self.now, self.timezone, self.user_id)
            .with_field_types(HashMap::from([(condition.field.clone(), field_type)]));
        condition_matches(&Filter::Condition(condition.clone()), &values, &context)
    }
}

pub(crate) fn record_part(filter: &Filter) -> Option<Filter> {
    match filter {
        Filter::Condition(condition) if trigger_path(condition).is_some() => None,
        Filter::Condition(_) | Filter::Related(_) => Some(filter.clone()),
        Filter::Group(group) => {
            let conditions: Vec<Filter> = group.conditions.iter().filter_map(record_part).collect();
            (!conditions.is_empty()).then_some(Filter::Group(FilterGroup {
                op: group.op,
                conditions,
            }))
        }
    }
}

pub(crate) fn trigger_violations(filter: &Filter, path: &str) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    visit(filter, path, &mut violations);
    violations
}

fn visit(filter: &Filter, path: &str, violations: &mut Vec<FieldViolation>) {
    match filter {
        Filter::Group(group) => {
            for (index, child) in group.conditions.iter().enumerate() {
                visit(child, &format!("{path}.conditions.{index}"), violations);
            }
        }
        Filter::Related(_) => {}
        Filter::Condition(condition) => {
            let Some(rest) = trigger_path(condition) else {
                return;
            };
            let head = rest.split('.').next().unwrap_or_default();
            if !TRIGGER_PATHS.contains(&head) {
                violations.push(FieldViolation::new(
                    format!("{path}.field"),
                    format!(
                        "trigger.{head} is not available (use {})",
                        TRIGGER_PATHS.join(", ")
                    ),
                ));
            }
        }
    }
}
