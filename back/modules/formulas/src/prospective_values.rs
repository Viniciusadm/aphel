use std::collections::{BTreeSet, HashMap};
use std::str::FromStr;

use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use serde_json::Value;

use aphel_sdk::formulas::PredicateInput;
const RECORD_PATH: &str = "record";
use crate::application::{
    AggregateFunction, AggregateRef, CompiledFormula, EvalEnv, FormulaContext, FormulaError,
    FormulaPreview, FormulaValue, INCOMPATIBLE_RESULT, Span, TypeFormulas, ValueKind,
    compile_formula, evaluate, formula_field_value,
};
use aphel_sdk::platform::domain::fields::{FieldDefinition, FieldType, FieldValue};

pub fn compile_rule(
    expression: &str,
    context: &FormulaContext,
) -> Result<CompiledFormula, Vec<FormulaError>> {
    let compiled = compile_formula(expression, context)?;
    if compiled.kind != Some(ValueKind::Bool) {
        return Err(vec![FormulaError::new(
            INCOMPATIBLE_RESULT,
            format!(
                "the condition results in {} and must result in a boolean",
                compiled.kind.map_or("null", |kind| kind.as_str())
            ),
            Span::new(0, expression.chars().count()),
        )]);
    }
    Ok(compiled)
}

pub fn violation_path(context: &FormulaContext, field: Option<&str>) -> String {
    let Some(key) = field else {
        return RECORD_PATH.to_owned();
    };
    match context.field(key) {
        Some(field) if field.field.is_system() => key.to_owned(),
        Some(_) => format!("custom_fields.{key}"),
        None => RECORD_PATH.to_owned(),
    }
}

pub fn stored_formula_value(
    definition: &FieldDefinition,
    value: &FieldValue,
    timezone: Tz,
) -> FormulaValue {
    match value {
        FieldValue::Text(text) if definition.field_type.base_type() == FieldType::Select => {
            definition.option_by_value(text).map_or_else(
                || FormulaValue::Text(text.clone()),
                |option| FormulaValue::Text(option.label.clone()),
            )
        }
        FieldValue::Text(text) | FieldValue::LongText(text) => FormulaValue::Text(text.clone()),
        FieldValue::Number(number) => FormulaValue::Number(*number),
        FieldValue::Date(date) => FormulaValue::Date(*date),
        FieldValue::DateTime(instant) => {
            FormulaValue::Date(instant.with_timezone(&timezone).date_naive())
        }
        FieldValue::Bool(value) => FormulaValue::Bool(*value),
        FieldValue::Option(id) => definition
            .option_by_id(*id)
            .map_or(FormulaValue::Null, |option| {
                FormulaValue::Text(option.label.clone())
            }),
        FieldValue::Options(_)
        | FieldValue::User(_)
        | FieldValue::Record(_)
        | FieldValue::References(_) => FormulaValue::Null,
    }
}

fn json_decimal(value: &Value) -> Option<Decimal> {
    match value {
        Value::Number(number) => Decimal::from_str(&number.to_string())
            .or_else(|_| Decimal::from_scientific(&number.to_string()))
            .ok(),
        Value::String(text) => Decimal::from_str(text.trim()).ok(),
        _ => None,
    }
}

fn json_date(value: &Value, timezone: Tz) -> Option<NaiveDate> {
    let text = value.as_str()?.trim();
    if let Ok(instant) = DateTime::parse_from_rfc3339(text) {
        return Some(instant.with_timezone(&timezone).date_naive());
    }
    NaiveDate::parse_from_str(text.get(..10)?, "%Y-%m-%d").ok()
}

pub fn json_formula_value(
    definition: &FieldDefinition,
    value: &Value,
    timezone: Tz,
) -> FormulaValue {
    if value.is_null() {
        return FormulaValue::Null;
    }
    match definition.field_type.base_type() {
        FieldType::Number | FieldType::Currency => {
            json_decimal(value).map_or(FormulaValue::Null, FormulaValue::Number)
        }
        FieldType::Date | FieldType::DateTime => {
            json_date(value, timezone).map_or(FormulaValue::Null, FormulaValue::Date)
        }
        FieldType::Checkbox => value
            .as_bool()
            .map_or(FormulaValue::Null, FormulaValue::Bool),
        FieldType::Select => match value.as_str() {
            Some(text) => {
                stored_formula_value(definition, &FieldValue::Text(text.to_owned()), timezone)
            }
            None => FormulaValue::Null,
        },
        FieldType::Text
        | FieldType::LongText
        | FieldType::Url
        | FieldType::Email
        | FieldType::Phone => match value {
            Value::String(text) => FormulaValue::Text(text.clone()),
            Value::Number(number) => FormulaValue::Text(number.to_string()),
            Value::Bool(flag) => FormulaValue::Text(flag.to_string()),
            _ => FormulaValue::Null,
        },
        FieldType::MultiSelect
        | FieldType::User
        | FieldType::Record
        | FieldType::References
        | FieldType::File
        | FieldType::Image
        | FieldType::Document => FormulaValue::Null,
        FieldType::Custom { .. } => unreachable!("native field representation"),
    }
}

fn formula_result_value(value: &FieldValue) -> FormulaValue {
    match value {
        FieldValue::Number(number) => FormulaValue::Number(*number),
        FieldValue::Date(date) => FormulaValue::Date(*date),
        FieldValue::Bool(value) => FormulaValue::Bool(*value),
        FieldValue::Text(text) | FieldValue::LongText(text) => FormulaValue::Text(text.clone()),
        _ => FormulaValue::Null,
    }
}

pub struct CompiledRule<'a> {
    pub rule: &'a PredicateInput,
    pub compiled: CompiledFormula,
}

pub fn rule_dependencies(
    formulas: &TypeFormulas,
    rules: &[CompiledRule<'_>],
) -> (BTreeSet<String>, BTreeSet<AggregateRef>) {
    let mut fields = BTreeSet::new();
    let mut aggregates = BTreeSet::new();
    let compiled = formulas
        .ordered
        .iter()
        .filter_map(|(_, compiled)| compiled.as_ref())
        .chain(rules.iter().map(|rule| &rule.compiled));
    for formula in compiled {
        fields.extend(formula.dependencies.fields.iter().cloned());
        aggregates.extend(formula.dependencies.aggregates.iter().cloned());
    }
    (fields, aggregates)
}

pub fn aggregate_value(aggregate: &AggregateRef, value: Option<Decimal>) -> FormulaValue {
    match value {
        Some(number) => FormulaValue::Number(number),
        None if matches!(
            aggregate.function,
            AggregateFunction::Count | AggregateFunction::Sum
        ) =>
        {
            FormulaValue::Number(Decimal::ZERO)
        }
        None => FormulaValue::Null,
    }
}

pub struct ProposedRecord {
    pub values: HashMap<String, FormulaValue>,
    pub aggregates: HashMap<AggregateRef, FormulaValue>,
    pub today: NaiveDate,
}

impl ProposedRecord {
    pub fn compute_formulas(&mut self, formulas: &TypeFormulas) {
        for (definition, compiled) in &formulas.ordered {
            let value = compiled.as_ref().and_then(|compiled| {
                let env = EvalEnv {
                    values: &self.values,
                    aggregates: &self.aggregates,
                    today: self.today,
                };
                formula_field_value(definition, &evaluate(&compiled.expression, &env))
            });
            self.values.insert(
                definition.key.as_str().to_owned(),
                value
                    .as_ref()
                    .map_or(FormulaValue::Null, formula_result_value),
            );
        }
    }

    pub fn fails(&self, compiled: &CompiledFormula) -> bool {
        let env = EvalEnv {
            values: &self.values,
            aggregates: &self.aggregates,
            today: self.today,
        };
        evaluate(&compiled.expression, &env).truthy()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BooleanPreview {
    pub formula: FormulaPreview,
    pub fails: Option<bool>,
}

pub fn rule_preview(expression: &str, mut formula: FormulaPreview) -> BooleanPreview {
    if formula.errors.is_empty() && formula.kind != Some(ValueKind::Bool) {
        formula.errors.push(FormulaError::new(
            INCOMPATIBLE_RESULT,
            format!(
                "the condition results in {} and must result in a boolean",
                formula.kind.map_or("null", |kind| kind.as_str())
            ),
            Span::new(0, expression.chars().count()),
        ));
    }
    let fails = (formula.evaluated && formula.errors.is_empty())
        .then(|| formula.value == Some(Value::Bool(true)));
    BooleanPreview { formula, fails }
}
