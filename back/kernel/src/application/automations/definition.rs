use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::application::query::Filter;
use crate::domain::automations::StepKind;
use crate::errors::FieldViolation;

pub const MAX_STEPS: usize = 50;
pub const MAX_BRANCH_CASES: usize = 10;
pub const MAX_STEP_ID_LENGTH: usize = 64;
pub const MAX_DEFINITION_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerSpec {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub options: Map<String, Value>,
}

impl TriggerSpec {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            options: Map::new(),
        }
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        self.options
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn texts(&self, key: &str) -> Vec<String> {
        self.options
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationOptions {
    #[serde(default = "enabled")]
    pub cancel_waiting_on_pause: bool,
    #[serde(default)]
    pub cancel_waiting_on_publish: bool,
}

fn enabled() -> bool {
    true
}

impl Default for AutomationOptions {
    fn default() -> Self {
        Self {
            cancel_waiting_on_pause: true,
            cancel_waiting_on_publish: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AutomationDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default)]
    pub steps: Vec<StepSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<AutomationOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSpec {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(flatten)]
    pub body: StepBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepBody {
    Condition {
        filter: Filter,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next: Option<String>,
    },
    Branch {
        cases: Vec<BranchCase>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },
    Action {
        action: String,
        #[serde(default)]
        config: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next: Option<String>,
    },
    Wait {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until_field: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recheck: Option<Filter>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next: Option<String>,
    },
    WaitUntil {
        filter: Filter,
        timeout: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_next: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchCase {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub filter: Filter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl StepBody {
    pub fn kind(&self) -> StepKind {
        match self {
            Self::Condition { .. } => StepKind::Condition,
            Self::Branch { .. } => StepKind::Branch,
            Self::Action { .. } => StepKind::Action,
            Self::Wait { .. } => StepKind::Wait,
            Self::WaitUntil { .. } => StepKind::WaitUntil,
        }
    }

    pub fn successors(&self) -> Vec<&str> {
        match self {
            Self::Condition { next, .. } | Self::Action { next, .. } | Self::Wait { next, .. } => {
                next.as_deref().into_iter().collect()
            }
            Self::WaitUntil {
                next, timeout_next, ..
            } => next
                .as_deref()
                .into_iter()
                .chain(timeout_next.as_deref())
                .collect(),
            Self::Branch { cases, default } => cases
                .iter()
                .filter_map(|case| case.next.as_deref())
                .chain(default.as_deref())
                .collect(),
        }
    }
}

impl AutomationDefinition {
    pub fn options(&self) -> AutomationOptions {
        self.options.unwrap_or_default()
    }

    pub fn step(&self, id: &str) -> Option<&StepSpec> {
        self.steps.iter().find(|step| step.id == id)
    }

    pub fn step_index(&self, id: &str) -> Option<usize> {
        self.steps.iter().position(|step| step.id == id)
    }

    pub fn actions(&self) -> impl Iterator<Item = (usize, &StepSpec, &str, &Value)> {
        self.steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| match &step.body {
                StepBody::Action { action, config, .. } => {
                    Some((index, step, action.as_str(), config))
                }
                _ => None,
            })
    }

    pub fn filters(&self) -> Vec<(String, &Filter)> {
        let mut filters = Vec::new();
        for (index, step) in self.steps.iter().enumerate() {
            match &step.body {
                StepBody::Condition { filter, .. } => {
                    filters.push((format!("definition.steps.{index}.filter"), filter))
                }
                StepBody::Branch { cases, .. } => {
                    for (case_index, case) in cases.iter().enumerate() {
                        filters.push((
                            format!("definition.steps.{index}.cases.{case_index}.filter"),
                            &case.filter,
                        ));
                    }
                }
                StepBody::Wait {
                    recheck: Some(filter),
                    ..
                } => filters.push((format!("definition.steps.{index}.recheck"), filter)),
                StepBody::WaitUntil { filter, .. } => {
                    filters.push((format!("definition.steps.{index}.filter"), filter))
                }
                StepBody::Action { .. } | StepBody::Wait { .. } => {}
            }
        }
        filters
    }

    pub fn structure_violations(&self) -> Vec<FieldViolation> {
        let mut violations = Vec::new();
        if self.steps.len() > MAX_STEPS {
            violations.push(FieldViolation::new(
                "definition.steps",
                format!("an automation can have at most {MAX_STEPS} steps"),
            ));
            return violations;
        }
        let mut ids = BTreeMap::new();
        for (index, step) in self.steps.iter().enumerate() {
            let path = format!("definition.steps.{index}.id");
            if !valid_step_id(&step.id) {
                violations.push(FieldViolation::new(
                    path,
                    "must have 1 to 64 letters, digits, '_' or '-'",
                ));
            } else if ids.insert(step.id.as_str(), index).is_some() {
                violations.push(FieldViolation::new(path, "is used by another step"));
            }
            if let StepBody::Branch { cases, .. } = &step.body {
                if cases.is_empty() || cases.len() > MAX_BRANCH_CASES {
                    violations.push(FieldViolation::new(
                        format!("definition.steps.{index}.cases"),
                        format!("must have 1 to {MAX_BRANCH_CASES} cases"),
                    ));
                }
                let mut keys = BTreeSet::new();
                for (case_index, case) in cases.iter().enumerate() {
                    if !valid_step_id(&case.key) || !keys.insert(case.key.as_str()) {
                        violations.push(FieldViolation::new(
                            format!("definition.steps.{index}.cases.{case_index}.key"),
                            "must be unique and have 1 to 64 letters, digits, '_' or '-'",
                        ));
                    }
                }
            }
        }
        if !violations.is_empty() {
            return violations;
        }
        for (index, step) in self.steps.iter().enumerate() {
            for successor in step.body.successors() {
                if !ids.contains_key(successor) {
                    violations.push(FieldViolation::new(
                        format!("definition.steps.{index}"),
                        format!("points to the unknown step {successor}"),
                    ));
                } else if successor == step.id {
                    violations.push(FieldViolation::new(
                        format!("definition.steps.{index}"),
                        "cannot point to itself",
                    ));
                }
            }
        }
        if !violations.is_empty() {
            return violations;
        }
        let Some(start) = self.start.as_deref() else {
            if !self.steps.is_empty() {
                violations.push(FieldViolation::new(
                    "definition.start",
                    "is required when there are steps",
                ));
            }
            return violations;
        };
        if !ids.contains_key(start) {
            violations.push(FieldViolation::new(
                "definition.start",
                "is not one of the steps",
            ));
            return violations;
        }
        if self.has_cycle(start) {
            violations.push(FieldViolation::new(
                "definition.steps",
                "the steps form a cycle; every path must end",
            ));
            return violations;
        }
        let reachable = self.reachable(start);
        for (index, step) in self.steps.iter().enumerate() {
            if !reachable.contains(step.id.as_str()) {
                violations.push(FieldViolation::new(
                    format!("definition.steps.{index}"),
                    "is not reachable from the start step",
                ));
            }
        }
        violations
    }

    fn reachable<'a>(&'a self, start: &'a str) -> BTreeSet<&'a str> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(step) = self.step(id) {
                pending.extend(step.body.successors());
            }
        }
        seen
    }

    fn has_cycle(&self, start: &str) -> bool {
        fn visit<'a>(
            definition: &'a AutomationDefinition,
            id: &'a str,
            visiting: &mut BTreeSet<&'a str>,
            done: &mut BTreeSet<&'a str>,
        ) -> bool {
            if done.contains(id) {
                return false;
            }
            if !visiting.insert(id) {
                return true;
            }
            if let Some(step) = definition.step(id) {
                for successor in step.body.successors() {
                    if visit(definition, successor, visiting, done) {
                        return true;
                    }
                }
            }
            visiting.remove(id);
            done.insert(id);
            false
        }
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        if visit(self, start, &mut visiting, &mut done) {
            return true;
        }
        self.steps.iter().any(|step| {
            let mut visiting = BTreeSet::new();
            visit(self, &step.id, &mut visiting, &mut done)
        })
    }
}

pub fn valid_step_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_STEP_ID_LENGTH
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

pub fn parse_definition(value: &Value) -> Result<AutomationDefinition, FieldViolation> {
    serde_json::from_value(value.clone())
        .map_err(|error| FieldViolation::new("definition", format!("is invalid: {error}")))
}

pub fn parse_trigger(value: &Value) -> Result<TriggerSpec, FieldViolation> {
    serde_json::from_value(value.clone())
        .map_err(|error| FieldViolation::new("trigger", format!("is invalid: {error}")))
}
