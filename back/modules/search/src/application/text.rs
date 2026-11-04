use aphel_kernel::errors::{AppError, FieldViolation};

pub const MIN_QUERY_LENGTH: usize = 2;
pub const MAX_QUERY_LENGTH: usize = 100;
pub const MIN_TERM_LENGTH: usize = 2;
pub const MAX_TERMS: usize = 8;
pub const MAX_VALUE_LENGTH: usize = 1000;
pub const MAX_CONTENT_LENGTH: usize = 4000;
const MIN_PHONE_DIGITS: usize = 6;
const MAX_PHONE_DIGITS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTerms {
    pub query: String,
    pub terms: Vec<String>,
}

pub fn is_phone_like(value: &str) -> bool {
    let digits = digits_of(value);
    value
        .chars()
        .all(|character| character.is_ascii_digit() || " +()-./".contains(character))
        && (MIN_PHONE_DIGITS..=MAX_PHONE_DIGITS).contains(&digits.len())
}

pub fn digits_of(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

pub fn normalize_content(value: &str) -> String {
    let mut content = normalize_name(value);
    if is_phone_like(value.trim()) {
        let digits = digits_of(value);
        if digits != content {
            content.push(' ');
            content.push_str(&digits);
        }
    }
    truncate(&content, MAX_CONTENT_LENGTH)
}

pub fn phone_content(value: &str) -> String {
    digits_of(value)
}

pub fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

pub fn parse_terms(query: &str) -> Result<SearchTerms, AppError> {
    let query = query.trim();
    let length = query.chars().count();
    if !(MIN_QUERY_LENGTH..=MAX_QUERY_LENGTH).contains(&length) {
        return Err(AppError::invalid_fields(
            "invalid search",
            vec![FieldViolation::new(
                "q",
                format!("must have between {MIN_QUERY_LENGTH} and {MAX_QUERY_LENGTH} characters"),
            )],
        ));
    }
    let terms = if is_phone_like(query) {
        vec![digits_of(query)]
    } else {
        let mut terms: Vec<String> = Vec::new();
        for term in normalize_name(query).split_whitespace() {
            if term.chars().count() >= MIN_TERM_LENGTH && !terms.iter().any(|known| known == term) {
                terms.push(term.to_owned());
            }
        }
        terms.truncate(MAX_TERMS);
        terms
    };
    Ok(SearchTerms {
        query: query.to_owned(),
        terms,
    })
}

pub use aphel_sdk::text::normalize_name;
