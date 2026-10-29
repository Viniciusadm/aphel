use serde::Serialize;

use crate::application::fields::formulas::Span;

pub const UNEXPECTED_CHARACTER: &str = "unexpected_character";
pub const UNTERMINATED_STRING: &str = "unterminated_string";
pub const INVALID_NUMBER: &str = "invalid_number";
pub const UNEXPECTED_TOKEN: &str = "unexpected_token";
pub const UNEXPECTED_END: &str = "unexpected_end";
pub const EXPRESSION_TOO_LONG: &str = "expression_too_long";
pub const EXPRESSION_TOO_COMPLEX: &str = "expression_too_complex";
pub const EMPTY_EXPRESSION: &str = "empty_expression";
pub const UNKNOWN_FIELD: &str = "unknown_field";
pub const UNKNOWN_RELATION: &str = "unknown_relation";
pub const UNKNOWN_FUNCTION: &str = "unknown_function";
pub const UNSUPPORTED_FIELD: &str = "unsupported_field";
pub const WRONG_ARGUMENTS: &str = "wrong_arguments";
pub const TYPE_MISMATCH: &str = "type_mismatch";
pub const RELATED_OUTSIDE_AGGREGATE: &str = "related_outside_aggregate";
pub const INCOMPATIBLE_RESULT: &str = "incompatible_result";
pub const FORMULA_CYCLE: &str = "formula_cycle";
pub const FIELD_NOT_ACCESSIBLE: &str = "field_not_accessible";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormulaError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl FormulaError {
    pub fn new(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }

    pub fn describe(&self) -> String {
        format!("at position {}: {}", self.span.start + 1, self.message)
    }
}
