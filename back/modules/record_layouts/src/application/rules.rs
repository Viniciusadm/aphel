use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value};

use crate::application::{
    ConditionContext, FormLayout, FormLayoutRepository, LayoutContext, LayoutField,
    condition_matches, has_value,
};
use aphel_kernel::application::fields::{ConditionalRules, FieldSet, WriteMode, custom_field_path};
use aphel_kernel::application::security::Clock;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::FieldValueError;
use aphel_kernel::errors::{AppError, FieldViolation};

pub fn rule_contexts(mode: WriteMode) -> &'static [LayoutContext] {
    match mode {
        WriteMode::Create => &[LayoutContext::Create],
        WriteMode::Update => &[LayoutContext::Edit, LayoutContext::Create],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleState {
    pub visible: bool,
    pub required: bool,
}

pub fn rule_state(
    field: &LayoutField,
    values: &Map<String, Value>,
    context: &ConditionContext,
) -> RuleState {
    let visible = field
        .visible_when
        .as_ref()
        .is_none_or(|filter| condition_matches(filter, values, context));
    let required = visible
        && field
            .required_when
            .as_ref()
            .is_some_and(|filter| condition_matches(filter, values, context));
    RuleState { visible, required }
}

pub fn required_violations(
    layout: &FormLayout,
    fields: &FieldSet,
    values: &Map<String, Value>,
    context: &ConditionContext,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    for placed in layout.fields() {
        if placed.required_when.is_none() {
            continue;
        }
        let Some(field) = fields.find_active(&placed.key) else {
            continue;
        };
        if field.definition.is_required || has_value(values.get(field.key())) {
            continue;
        }
        if !rule_state(placed, values, context).required {
            continue;
        }
        let path = if field.is_system() {
            field.key().to_owned()
        } else {
            custom_field_path(field.key())
        };
        violations.push(FieldViolation::new(
            path,
            FieldValueError::Required.message(),
        ));
    }
    violations
}

pub struct LayoutConditionalRules<R, C> {
    layouts: R,
    clock: C,
}

impl<R, C> LayoutConditionalRules<R, C> {
    pub fn new(layouts: R, clock: C) -> Self {
        Self { layouts, clock }
    }
}

#[async_trait]
impl<R, C> ConditionalRules for LayoutConditionalRules<R, C>
where
    R: FormLayoutRepository,
    C: Clock,
{
    async fn violations(
        &self,
        scope: &AccessScope,
        fields: &FieldSet,
        values: &Map<String, Value>,
        mode: WriteMode,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let source = self
            .layouts
            .rules_source(&fields.record_type.record_type, rule_contexts(mode))
            .await?;
        let Some(layout) = source.layout.filter(FormLayout::has_required_rules) else {
            return Ok(Vec::new());
        };
        let timezone = source
            .timezone
            .and_then(|zone| zone.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::UTC);
        let now = self.clock.now();
        let context =
            ConditionContext::new(now, timezone, scope.user_id().as_str()).with_fields(fields);
        let values = completed_values(fields, values, mode, now);
        Ok(required_violations(&layout, fields, &values, &context))
    }
}

const NAME_FIELD: &str = "name";
const NAME_PARTS: [&str; 2] = ["first_name", "last_name"];
const CREATED_AT_FIELD: &str = "created_at";
const UPDATED_AT_FIELD: &str = "updated_at";

pub fn completed_values(
    fields: &FieldSet,
    values: &Map<String, Value>,
    mode: WriteMode,
    now: DateTime<Utc>,
) -> Map<String, Value> {
    let mut completed = values.clone();
    let missing = |completed: &Map<String, Value>, key: &str| {
        fields.find_active(key).is_some() && !has_value(completed.get(key))
    };
    if missing(&completed, NAME_FIELD) && NAME_PARTS.iter().any(|part| values.contains_key(*part)) {
        let name = NAME_PARTS
            .iter()
            .filter_map(|part| values.get(*part).and_then(Value::as_str))
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !name.is_empty() {
            completed.insert(NAME_FIELD.to_owned(), Value::String(name));
        }
    }
    let timestamp = Value::String(now.to_rfc3339_opts(SecondsFormat::Secs, true));
    if mode == WriteMode::Create && missing(&completed, CREATED_AT_FIELD) {
        completed.insert(CREATED_AT_FIELD.to_owned(), timestamp.clone());
    }
    if missing(&completed, UPDATED_AT_FIELD) {
        completed.insert(UPDATED_AT_FIELD.to_owned(), timestamp);
    }
    completed
}
