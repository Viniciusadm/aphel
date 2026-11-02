#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionSpec {
    pub name: &'static str,
    pub signature: &'static str,
    pub category: &'static str,
}

const fn spec(name: &'static str, signature: &'static str, category: &'static str) -> FunctionSpec {
    FunctionSpec {
        name,
        signature,
        category,
    }
}

pub const FUNCTIONS: [FunctionSpec; 21] = [
    spec("today", "today()", "date"),
    spec("days_between", "days_between(start, end)", "date"),
    spec("days_until", "days_until(date)", "date"),
    spec("days_since", "days_since(date)", "date"),
    spec("add_days", "add_days(date, days)", "date"),
    spec("if", "if(condition, then, else)", "logic"),
    spec("coalesce", "coalesce(value, fallback, ...)", "logic"),
    spec("is_empty", "is_empty(value)", "logic"),
    spec("concat", "concat(value, ...)", "text"),
    spec("upper", "upper(text)", "text"),
    spec("lower", "lower(text)", "text"),
    spec("len", "len(text)", "text"),
    spec("round", "round(number, decimals)", "number"),
    spec("abs", "abs(number)", "number"),
    spec("floor", "floor(number)", "number"),
    spec("ceil", "ceil(number)", "number"),
    spec(
        "min",
        "min(value, value, ...) | min(relation.field)",
        "number",
    ),
    spec(
        "max",
        "max(value, value, ...) | max(relation.field)",
        "number",
    ),
    spec("count", "count(relation)", "aggregate"),
    spec("sum", "sum(relation.field)", "aggregate"),
    spec("avg", "avg(relation.field)", "aggregate"),
];

pub fn is_known_function(name: &str) -> bool {
    FUNCTIONS.iter().any(|function| function.name == name)
}
