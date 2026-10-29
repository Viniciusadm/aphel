use rust_decimal::Decimal;
use uuid::Uuid;

use crate::application::fields::permissions::FIELD_NOT_ACCESSIBLE;
use crate::errors::{AppError, CodedKind};

pub fn parse_uuid(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value.trim()).ok()
}

pub fn money_text(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

pub fn normalize_currency_code(value: &str) -> Option<String> {
    let code = value.trim().to_ascii_uppercase();
    (code.len() == 3 && code.chars().all(|character| character.is_ascii_uppercase()))
        .then_some(code)
}

pub fn is_access_denied(error: &AppError) -> bool {
    match error {
        AppError::Forbidden(_) => true,
        AppError::Coded { kind, .. } => *kind == CodedKind::Forbidden,
        AppError::CodedFields { code, .. } => *code == FIELD_NOT_ACCESSIBLE,
        _ => false,
    }
}
