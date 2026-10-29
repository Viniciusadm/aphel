use std::cmp::Ordering;
use std::collections::HashMap;

use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value};

use crate::application::fields::FieldSet;
use crate::application::query::{CURRENT_USER, Filter, FilterCondition, LogicalOperator};
use crate::domain::fields::FieldType;

#[derive(Debug, Clone)]
pub struct ConditionContext {
    pub now: DateTime<Utc>,
    pub timezone: Tz,
    pub user_id: String,
    pub field_types: HashMap<String, FieldType>,
}

impl ConditionContext {
    pub fn new(now: DateTime<Utc>, timezone: Tz, user_id: impl Into<String>) -> Self {
        Self {
            now,
            timezone,
            user_id: user_id.into(),
            field_types: HashMap::new(),
        }
    }

    pub fn with_fields(mut self, fields: &FieldSet) -> Self {
        self.field_types = fields
            .active()
            .map(|field| (field.key().to_owned(), field.definition.field_type))
            .collect();
        self
    }

    pub fn with_field_types(mut self, field_types: HashMap<String, FieldType>) -> Self {
        self.field_types = field_types;
        self
    }

    fn today(&self) -> NaiveDate {
        self.now.with_timezone(&self.timezone).date_naive()
    }
}

pub fn condition_matches(
    filter: &Filter,
    values: &Map<String, Value>,
    context: &ConditionContext,
) -> bool {
    match filter {
        Filter::Group(group) => match group.op {
            LogicalOperator::And => group
                .conditions
                .iter()
                .all(|child| condition_matches(child, values, context)),
            LogicalOperator::Or => {
                group.conditions.is_empty()
                    || group
                        .conditions
                        .iter()
                        .any(|child| condition_matches(child, values, context))
            }
        },
        Filter::Condition(condition) => leaf_matches(condition, values, context),
        Filter::Related(_) => false,
    }
}

pub fn has_value(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(text)) => !text.trim().is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

fn leaf_matches(
    condition: &FilterCondition,
    values: &Map<String, Value>,
    context: &ConditionContext,
) -> bool {
    let (Some(field_type), Some(value)) = (
        context.field_types.get(&condition.field),
        values.get(&condition.field),
    ) else {
        return false;
    };
    let operator = condition.operator.as_str();
    if *field_type == FieldType::Checkbox && matches!(operator, "eq" | "neq") {
        let Value::Bool(expected) = condition.value else {
            return false;
        };
        let checked = *value == Value::Bool(true);
        return (checked == expected) == (operator == "eq");
    }
    let (positive, negated) = match operator {
        "neq" => ("eq", true),
        "not_contains" => ("contains", true),
        "not_in" => ("in", true),
        "is_empty" => ("is_not_empty", true),
        other => (other, false),
    };
    let matched = positive_matches(positive, *field_type, value, &condition.value, context);
    matched != negated
}

fn positive_matches(
    operator: &str,
    field_type: FieldType,
    value: &Value,
    operand: &Value,
    context: &ConditionContext,
) -> bool {
    if !has_value(Some(value)) {
        return false;
    }
    match operator {
        "is_not_empty" => true,
        "eq" => equals(field_type, value, operand, context),
        "in" => match operand {
            Value::Array(items) => items
                .iter()
                .any(|item| equals(field_type, value, item, context)),
            _ => false,
        },
        "contains" => text_test(field_type, value, operand, |text, needle| {
            text.contains(needle)
        }),
        "starts_with" => text_test(field_type, value, operand, |text, needle| {
            text.starts_with(needle)
        }),
        "gt" => compare(field_type, value, operand, context) == Some(Ordering::Greater),
        "gte" => matches!(
            compare(field_type, value, operand, context),
            Some(Ordering::Greater | Ordering::Equal)
        ),
        "lt" => compare(field_type, value, operand, context) == Some(Ordering::Less),
        "lte" => matches!(
            compare(field_type, value, operand, context),
            Some(Ordering::Less | Ordering::Equal)
        ),
        "between" => match operand.as_array().map(Vec::as_slice) {
            Some([low, high]) => {
                matches!(
                    compare(field_type, value, low, context),
                    Some(Ordering::Greater | Ordering::Equal)
                ) && matches!(
                    compare(field_type, value, high, context),
                    Some(Ordering::Less | Ordering::Equal)
                )
            }
            _ => false,
        },
        relative => match relative_range(relative, operand, context) {
            Some((from, until)) => local_day(field_type, value, context).is_some_and(|day| {
                from.is_none_or(|from| day >= from) && until.is_none_or(|until| day < until)
            }),
            None => false,
        },
    }
}

fn relative_range(
    operator: &str,
    operand: &Value,
    context: &ConditionContext,
) -> Option<(Option<NaiveDate>, Option<NaiveDate>)> {
    let today = context.today();
    let range = |from: NaiveDate, until: NaiveDate| Some((Some(from), Some(until)));
    let monday = add_days(today, -i64::from(today.weekday().num_days_from_monday()));
    let first = today.with_day(1).unwrap_or(today);
    let next_month = |day: NaiveDate| day.checked_add_months(Months::new(1)).unwrap_or(day);
    let days = || operand.as_i64().filter(|days| *days >= 0);
    match operator {
        "today" => range(today, add_days(today, 1)),
        "yesterday" => range(add_days(today, -1), today),
        "tomorrow" => range(add_days(today, 1), add_days(today, 2)),
        "this_week" => range(monday, add_days(monday, 7)),
        "last_week" => range(add_days(monday, -7), monday),
        "next_week" => range(add_days(monday, 7), add_days(monday, 14)),
        "this_month" => range(first, next_month(first)),
        "last_month" => {
            let start = first.checked_sub_months(Months::new(1)).unwrap_or(first);
            range(start, first)
        }
        "next_month" => {
            let start = next_month(first);
            range(start, next_month(start))
        }
        "this_year" => {
            let start = NaiveDate::from_ymd_opt(today.year(), 1, 1)?;
            let end = NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)?;
            range(start, end)
        }
        "within_last_days" => range(add_days(today, -days()?), add_days(today, 1)),
        "within_next_days" => range(today, add_days(today, days()? + 1)),
        "older_than_days" => Some((None, Some(add_days(today, -days()?)))),
        _ => None,
    }
}

fn equals(
    field_type: FieldType,
    value: &Value,
    operand: &Value,
    context: &ConditionContext,
) -> bool {
    if let Value::Array(items) = value {
        return items
            .iter()
            .any(|item| equals(field_type, item, operand, context));
    }
    match field_type {
        FieldType::Number | FieldType::Currency | FieldType::Date | FieldType::DateTime => {
            compare(field_type, value, operand, context) == Some(Ordering::Equal)
        }
        FieldType::User if operand.as_str().map(str::trim) == Some(CURRENT_USER) => {
            text_of(value).is_some_and(|text| text == context.user_id.to_lowercase())
        }
        FieldType::Phone => match (digits_of(value), digits_of(operand)) {
            (Some(left), Some(right)) => same_phone(&left, &right),
            _ => false,
        },
        _ => match (text_of(value), text_of(operand)) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        },
    }
}

const MIN_PHONE_SUFFIX_DIGITS: usize = 8;

fn same_phone(left: &str, right: &str) -> bool {
    let (shorter, longer) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    shorter == longer || (shorter.len() >= MIN_PHONE_SUFFIX_DIGITS && longer.ends_with(shorter))
}

fn digits_of(value: &Value) -> Option<String> {
    let digits: String = match value {
        Value::String(text) => text.chars().filter(char::is_ascii_digit).collect(),
        Value::Number(number) => number.to_string(),
        _ => return None,
    };
    (!digits.is_empty()).then_some(digits)
}

fn compare(
    field_type: FieldType,
    value: &Value,
    operand: &Value,
    context: &ConditionContext,
) -> Option<Ordering> {
    match field_type {
        FieldType::Number | FieldType::Currency => {
            number_of(value)?.partial_cmp(&number_of(operand)?)
        }
        FieldType::Date => Some(date_of(value)?.cmp(&date_of(operand)?)),
        FieldType::DateTime => match date_of(operand) {
            Some(day) => Some(local_day(field_type, value, context)?.cmp(&day)),
            None => Some(instant_of(value)?.cmp(&instant_of(operand)?)),
        },
        _ => None,
    }
}

fn local_day(
    field_type: FieldType,
    value: &Value,
    context: &ConditionContext,
) -> Option<NaiveDate> {
    match field_type {
        FieldType::Date => date_of(value),
        FieldType::DateTime => instant_of(value)
            .map(|instant| instant.with_timezone(&context.timezone).date_naive())
            .or_else(|| date_of(value)),
        _ => None,
    }
}

fn text_test(
    field_type: FieldType,
    value: &Value,
    operand: &Value,
    test: impl Fn(&str, &str) -> bool,
) -> bool {
    let comparable = |value: &Value| match field_type {
        FieldType::Phone => digits_of(value),
        _ => text_of(value),
    };
    let Some(needle) = comparable(operand).filter(|needle| !needle.is_empty()) else {
        return false;
    };
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(comparable)
            .any(|text| test(&text, &needle)),
        other => comparable(other).is_some_and(|text| test(&text, &needle)),
    }
}

fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        other => other,
    }
}

fn folded(text: &str) -> String {
    text.trim()
        .chars()
        .map(fold_accent)
        .flat_map(char::to_lowercase)
        .collect()
}

fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(folded(text)),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|number| number.is_finite())
}

fn date_of(value: &Value) -> Option<NaiveDate> {
    value
        .as_str()
        .and_then(|text| NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok())
}

fn instant_of(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_str()
        .and_then(|text| DateTime::parse_from_rfc3339(text.trim()).ok())
        .map(|instant| instant.with_timezone(&Utc))
}

fn add_days(day: NaiveDate, days: i64) -> NaiveDate {
    if days >= 0 {
        day.checked_add_days(Days::new(days.unsigned_abs()))
    } else {
        day.checked_sub_days(Days::new(days.unsigned_abs()))
    }
    .unwrap_or(day)
}
