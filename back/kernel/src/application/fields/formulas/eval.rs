use std::cmp::Ordering;
use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::application::fields::formulas::{
    AggregateFunction, AggregateRef, BinaryOp, Expr, ExprKind, FormulaValue, MAX_TEXT_VALUE_LENGTH,
    UnaryOp,
};
use crate::domain::fields::MAX_NUMBER_ABS;

const MAX_ROUND_DECIMALS: u32 = 6;
const MAX_DAY_OFFSET: i64 = 3_650_000;

pub struct EvalEnv<'a> {
    pub values: &'a HashMap<String, FormulaValue>,
    pub aggregates: &'a HashMap<AggregateRef, FormulaValue>,
    pub today: NaiveDate,
}

pub fn evaluate(expression: &Expr, env: &EvalEnv) -> FormulaValue {
    match &expression.kind {
        ExprKind::Number(number) => FormulaValue::Number(*number),
        ExprKind::Text(text) => FormulaValue::Text(text.clone()),
        ExprKind::Bool(value) => FormulaValue::Bool(*value),
        ExprKind::Null | ExprKind::Related { .. } => FormulaValue::Null,
        ExprKind::Field(key) => env.values.get(key).cloned().unwrap_or(FormulaValue::Null),
        ExprKind::Unary(UnaryOp::Negate, operand) => match evaluate(operand, env) {
            FormulaValue::Number(number) => FormulaValue::Number(-number),
            _ => FormulaValue::Null,
        },
        ExprKind::Unary(UnaryOp::Not, operand) => match evaluate(operand, env) {
            FormulaValue::Bool(value) => FormulaValue::Bool(!value),
            _ => FormulaValue::Null,
        },
        ExprKind::Binary(operator, left, right) => binary(*operator, left, right, env),
        ExprKind::Call { name, args } => call(name, args, env),
    }
}

fn bounded(number: Option<Decimal>) -> FormulaValue {
    match number {
        Some(number) if number.abs() < Decimal::from(MAX_NUMBER_ABS) => {
            FormulaValue::Number(number)
        }
        _ => FormulaValue::Null,
    }
}

fn shift(date: NaiveDate, days: Decimal) -> FormulaValue {
    let Some(days) = days
        .trunc()
        .to_i64()
        .filter(|days| days.abs() <= MAX_DAY_OFFSET)
    else {
        return FormulaValue::Null;
    };
    date.checked_add_signed(Duration::days(days))
        .map_or(FormulaValue::Null, FormulaValue::Date)
}

fn days(from: NaiveDate, to: NaiveDate) -> FormulaValue {
    FormulaValue::Number(Decimal::from((to - from).num_days()))
}

fn compare(left: &FormulaValue, right: &FormulaValue) -> Option<Ordering> {
    match (left, right) {
        (FormulaValue::Number(left), FormulaValue::Number(right)) => Some(left.cmp(right)),
        (FormulaValue::Text(left), FormulaValue::Text(right)) => {
            Some(left.to_lowercase().cmp(&right.to_lowercase()))
        }
        (FormulaValue::Date(left), FormulaValue::Date(right)) => Some(left.cmp(right)),
        (FormulaValue::Bool(left), FormulaValue::Bool(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn binary(operator: BinaryOp, left: &Expr, right: &Expr, env: &EvalEnv) -> FormulaValue {
    if operator == BinaryOp::And {
        let left = evaluate(left, env);
        if !left.truthy() {
            return FormulaValue::Bool(false);
        }
        return FormulaValue::Bool(evaluate(right, env).truthy());
    }
    if operator == BinaryOp::Or {
        let left = evaluate(left, env);
        if left.truthy() {
            return FormulaValue::Bool(true);
        }
        return FormulaValue::Bool(evaluate(right, env).truthy());
    }
    let left = evaluate(left, env);
    let right = evaluate(right, env);
    use FormulaValue::*;
    match operator {
        BinaryOp::Equal | BinaryOp::NotEqual => {
            let equal = match (left.is_empty(), right.is_empty()) {
                (true, true) => true,
                (true, false) | (false, true) => false,
                (false, false) => compare(&left, &right) == Some(Ordering::Equal),
            };
            Bool(equal == (operator == BinaryOp::Equal))
        }
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            let Some(ordering) = compare(&left, &right) else {
                return Null;
            };
            Bool(match operator {
                BinaryOp::Less => ordering == Ordering::Less,
                BinaryOp::LessEqual => ordering != Ordering::Greater,
                BinaryOp::Greater => ordering == Ordering::Greater,
                _ => ordering != Ordering::Less,
            })
        }
        BinaryOp::Add => match (left, right) {
            (Number(left), Number(right)) => bounded(left.checked_add(right)),
            (Date(date), Number(days)) | (Number(days), Date(date)) => shift(date, days),
            _ => Null,
        },
        BinaryOp::Subtract => match (left, right) {
            (Number(left), Number(right)) => bounded(left.checked_sub(right)),
            (Date(left), Date(right)) => days(right, left),
            (Date(date), Number(offset)) => shift(date, -offset),
            _ => Null,
        },
        BinaryOp::Multiply => match (left, right) {
            (Number(left), Number(right)) => bounded(left.checked_mul(right)),
            _ => Null,
        },
        BinaryOp::Divide => match (left, right) {
            (Number(_), Number(right)) if right.is_zero() => Null,
            (Number(left), Number(right)) => bounded(left.checked_div(right)),
            _ => Null,
        },
        BinaryOp::And | BinaryOp::Or => Null,
    }
}

fn number_arg(args: &[Expr], index: usize, env: &EvalEnv) -> Option<Decimal> {
    match args.get(index).map(|arg| evaluate(arg, env)) {
        Some(FormulaValue::Number(number)) => Some(number),
        _ => None,
    }
}

fn date_arg(args: &[Expr], index: usize, env: &EvalEnv) -> Option<NaiveDate> {
    match args.get(index).map(|arg| evaluate(arg, env)) {
        Some(FormulaValue::Date(date)) => Some(date),
        _ => None,
    }
}

fn text_arg(args: &[Expr], index: usize, env: &EvalEnv) -> Option<String> {
    match args.get(index).map(|arg| evaluate(arg, env)) {
        Some(FormulaValue::Text(text)) => Some(text),
        _ => None,
    }
}

fn truncated(mut text: String) -> FormulaValue {
    if text.chars().count() > MAX_TEXT_VALUE_LENGTH {
        text = text.chars().take(MAX_TEXT_VALUE_LENGTH).collect();
    }
    FormulaValue::Text(text)
}

fn aggregate(name: &str, args: &[Expr], env: &EvalEnv) -> FormulaValue {
    let function = match name {
        "count" => AggregateFunction::Count,
        "sum" => AggregateFunction::Sum,
        "avg" => AggregateFunction::Avg,
        "min" => AggregateFunction::Min,
        _ => AggregateFunction::Max,
    };
    let (relation, field) = match args.first().map(|arg| &arg.kind) {
        Some(ExprKind::Field(relation)) => (relation.clone(), None),
        Some(ExprKind::Related { relation, field }) => (relation.clone(), Some(field.clone())),
        _ => return FormulaValue::Null,
    };
    let key = AggregateRef {
        function,
        relation,
        field: if function == AggregateFunction::Count {
            None
        } else {
            field
        },
    };
    env.aggregates.get(&key).cloned().unwrap_or(match function {
        AggregateFunction::Count | AggregateFunction::Sum => FormulaValue::Number(Decimal::ZERO),
        _ => FormulaValue::Null,
    })
}

fn extreme(args: &[Expr], env: &EvalEnv, wanted: Ordering) -> FormulaValue {
    let mut best = FormulaValue::Null;
    for arg in args {
        let value = evaluate(arg, env);
        if value.is_null() {
            continue;
        }
        if best.is_null() || compare(&value, &best) == Some(wanted) {
            best = value;
        }
    }
    best
}

fn call(name: &str, args: &[Expr], env: &EvalEnv) -> FormulaValue {
    use FormulaValue::*;
    let is_aggregate = matches!(name, "count" | "sum" | "avg")
        || (matches!(name, "min" | "max") && args.len() == 1);
    if is_aggregate {
        return aggregate(name, args, env);
    }
    match name {
        "today" => Date(env.today),
        "days_between" => match (date_arg(args, 0, env), date_arg(args, 1, env)) {
            (Some(start), Some(end)) => days(start, end),
            _ => Null,
        },
        "days_until" => date_arg(args, 0, env).map_or(Null, |date| days(env.today, date)),
        "days_since" => date_arg(args, 0, env).map_or(Null, |date| days(date, env.today)),
        "add_days" => match (date_arg(args, 0, env), number_arg(args, 1, env)) {
            (Some(date), Some(offset)) => shift(date, offset),
            _ => Null,
        },
        "if" => {
            let condition = args.first().map(|arg| evaluate(arg, env));
            let branch = if condition.is_some_and(|value| value.truthy()) {
                1
            } else {
                2
            };
            args.get(branch).map_or(Null, |arg| evaluate(arg, env))
        }
        "coalesce" => args
            .iter()
            .map(|arg| evaluate(arg, env))
            .find(|value| !value.is_empty())
            .unwrap_or(Null),
        "is_empty" => Bool(args.first().is_none_or(|arg| evaluate(arg, env).is_empty())),
        "concat" => {
            let mut text = String::new();
            for arg in args {
                text.push_str(&evaluate(arg, env).as_text());
                if text.len() > MAX_TEXT_VALUE_LENGTH * 4 {
                    break;
                }
            }
            truncated(text)
        }
        "upper" => text_arg(args, 0, env).map_or(Null, |text| Text(text.to_uppercase())),
        "lower" => text_arg(args, 0, env).map_or(Null, |text| Text(text.to_lowercase())),
        "len" => text_arg(args, 0, env).map_or(Null, |text| {
            Number(Decimal::from(text.chars().count() as u64))
        }),
        "round" => {
            let Some(number) = number_arg(args, 0, env) else {
                return Null;
            };
            let decimals = match args.len() {
                1 => 0,
                _ => match number_arg(args, 1, env).and_then(|value| value.trunc().to_u32()) {
                    Some(decimals) => decimals.min(MAX_ROUND_DECIMALS),
                    None => return Null,
                },
            };
            Number(number.round_dp_with_strategy(decimals, RoundingStrategy::MidpointAwayFromZero))
        }
        "abs" => number_arg(args, 0, env).map_or(Null, |number| Number(number.abs())),
        "floor" => number_arg(args, 0, env).map_or(Null, |number| Number(number.floor())),
        "ceil" => number_arg(args, 0, env).map_or(Null, |number| Number(number.ceil())),
        "min" => extreme(args, env, Ordering::Less),
        "max" => extreme(args, env, Ordering::Greater),
        _ => Null,
    }
}
