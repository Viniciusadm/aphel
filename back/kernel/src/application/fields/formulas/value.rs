use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::Value;

use crate::application::fields::decimal_json;
use crate::domain::fields::FormulaResultType;

pub const MAX_TEXT_VALUE_LENGTH: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ValueKind {
    Number,
    Text,
    Date,
    Bool,
}

impl ValueKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Text => "text",
            Self::Date => "date",
            Self::Bool => "boolean",
        }
    }

    pub fn fits(kind: Option<Self>, result_type: FormulaResultType) -> bool {
        match (kind, result_type) {
            (None, _) | (Some(_), FormulaResultType::Text) => true,
            (Some(kind), FormulaResultType::Number | FormulaResultType::Currency) => {
                kind == Self::Number
            }
            (Some(kind), FormulaResultType::Date) => kind == Self::Date,
            (Some(kind), FormulaResultType::Boolean) => kind == Self::Bool,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaValue {
    Null,
    Number(Decimal),
    Text(String),
    Date(NaiveDate),
    Bool(bool),
}

impl FormulaValue {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Text(text) => text.trim().is_empty(),
            _ => false,
        }
    }

    pub fn as_text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Number(number) => number.normalize().to_string(),
            Self::Text(text) => text.clone(),
            Self::Date(date) => date.format("%Y-%m-%d").to_string(),
            Self::Bool(value) => value.to_string(),
        }
    }

    pub fn truthy(&self) -> bool {
        matches!(self, Self::Bool(true))
    }

    pub fn to_json(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Number(number) => decimal_json(*number),
            Self::Text(text) => Value::String(text.clone()),
            Self::Date(date) => Value::String(date.format("%Y-%m-%d").to_string()),
            Self::Bool(value) => Value::Bool(*value),
        }
    }
}
