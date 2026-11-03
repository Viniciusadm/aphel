use crate::domain::INVALID_SUBJECT_TERM;
use aphel_kernel::{
    domain::fields::{normalize_email, normalize_phone},
    errors::AppError,
};
use aphel_sdk::privacy::SubjectKind;
pub use aphel_sdk::privacy::{Redaction, SubjectMatcher, mask_digits, mask_email, mask_text};
const MIN_TERM_LENGTH: usize = 3;
const MAX_TERM_LENGTH: usize = 255;
const DEFAULT_PHONE_COUNTRY: &str = "BR";
fn invalid_term(message: &str) -> AppError {
    AppError::validation_code(INVALID_SUBJECT_TERM, message.to_owned())
}
fn is_phone_like(term: &str) -> bool {
    term.chars()
        .all(|character| character.is_ascii_digit() || " +()-./".contains(character))
}

pub fn parse_subject(term: &str) -> Result<SubjectMatcher, AppError> {
    let term = term.trim();
    let length = term.chars().count();
    if length < MIN_TERM_LENGTH {
        return Err(invalid_term(
            "the search term must have at least 3 characters",
        ));
    }
    if length > MAX_TERM_LENGTH {
        return Err(invalid_term(
            "the search term must have at most 255 characters",
        ));
    }
    if term.contains('@') {
        let email = normalize_email(term).unwrap_or_else(|_| term.to_lowercase());
        return Ok(SubjectMatcher {
            kind: SubjectKind::Email,
            email: Some(email),
            phone: None,
            digits: None,
            text: None,
        });
    }
    let digits: String = term.chars().filter(char::is_ascii_digit).collect();
    if is_phone_like(term) && (6..=15).contains(&digits.len()) {
        let phone = normalize_phone(term, DEFAULT_PHONE_COUNTRY).ok();
        let looks_like_document =
            !term.starts_with('+') && (term.contains('.') || term.contains('/'));
        let kind = if phone.is_some() && !looks_like_document {
            SubjectKind::Phone
        } else {
            SubjectKind::Document
        };
        return Ok(SubjectMatcher {
            kind,
            email: None,
            phone,
            digits: Some(digits),
            text: None,
        });
    }
    Ok(SubjectMatcher {
        kind: SubjectKind::Text,
        email: None,
        phone: None,
        digits: None,
        text: Some(term.to_owned()),
    })
}
