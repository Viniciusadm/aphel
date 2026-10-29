use std::collections::BTreeSet;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::{INVALID_SUBJECT_TERM, SubjectKind};
use aphel_kernel::application::shared::REDACTED;
use aphel_kernel::domain::fields::{normalize_email, normalize_phone};
use aphel_kernel::errors::AppError;

pub(crate) const MIN_TERM_LENGTH: usize = 3;
pub(crate) const MAX_TERM_LENGTH: usize = 255;
pub(crate) const DEFAULT_PHONE_COUNTRY: &str = "BR";
const MIN_REDACTED_VALUE_LENGTH: usize = 3;
const MIN_IDENTIFIER_DIGITS: usize = 6;
const MIN_NAME_WORD_LENGTH: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectMatcher {
    pub(crate) kind: SubjectKind,
    pub(crate) email: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) digits: Option<String>,
    pub(crate) text: Option<String>,
}

impl SubjectMatcher {
    pub(crate) fn canonical(&self) -> String {
        self.email
            .clone()
            .or_else(|| self.phone.clone())
            .or_else(|| self.digits.clone())
            .or_else(|| self.text.as_ref().map(|text| text.to_lowercase()))
            .unwrap_or_default()
    }

    pub(crate) fn label(&self) -> String {
        match self.kind {
            SubjectKind::Email => mask_email(self.email.as_deref().unwrap_or_default()),
            SubjectKind::Phone => mask_digits(
                self.phone
                    .as_deref()
                    .or(self.digits.as_deref())
                    .unwrap_or_default(),
            ),
            SubjectKind::Document => mask_digits(self.digits.as_deref().unwrap_or_default()),
            SubjectKind::Text => mask_text(self.text.as_deref().unwrap_or_default()),
        }
    }

    pub(crate) fn hash(&self) -> String {
        let digest =
            Sha256::digest(format!("{}:{}", self.kind.as_str(), self.canonical()).as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

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

fn find_whole(text: &str, value: &str, from: usize) -> Option<usize> {
    let guard_start = value.starts_with(char::is_alphanumeric);
    let guard_end = value.ends_with(char::is_alphanumeric);
    text[from..]
        .match_indices(value)
        .map(|(offset, _)| from + offset)
        .find(|&start| {
            let end = start + value.len();
            (!guard_start
                || text[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|character| !character.is_alphanumeric()))
                && (!guard_end
                    || text[end..]
                        .chars()
                        .next()
                        .is_none_or(|character| !character.is_alphanumeric()))
        })
}

fn mask_part(value: &str) -> String {
    let first: String = value.chars().take(1).collect();
    format!("{first}***")
}

pub fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return mask_text(email);
    };
    let (host, suffix) = match domain.rsplit_once('.') {
        Some((host, suffix)) => (host, format!(".{suffix}")),
        None => (domain, String::new()),
    };
    format!("{}@{}{suffix}", mask_part(local), mask_part(host))
}

pub fn mask_digits(value: &str) -> String {
    let digits: Vec<char> = value.chars().filter(char::is_ascii_digit).collect();
    let visible = digits.len().saturating_sub(digits.len().min(4));
    let tail: String = digits[visible..].iter().collect();
    format!("***{tail}")
}

pub fn mask_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(mask_part)
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Redaction {
    values: BTreeSet<String>,
    fields: BTreeSet<String>,
    mentions: BTreeSet<String>,
}

fn is_identifier(value: &str) -> bool {
    let digits = value.chars().filter(char::is_ascii_digit).count();
    digits >= MIN_IDENTIFIER_DIGITS && is_phone_like(value)
}

fn is_email_like(value: &str) -> bool {
    value
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
        && !value.contains(char::is_whitespace)
}

fn json_texts(value: &Value) -> Vec<&str> {
    match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().flat_map(json_texts).collect(),
        Value::Object(map) => map.values().flat_map(json_texts).collect(),
        _ => Vec::new(),
    }
}

impl Redaction {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_value(&mut self, value: &str) {
        let value = value.trim().to_lowercase();
        if value.chars().count() >= MIN_REDACTED_VALUE_LENGTH {
            self.values.insert(value);
        }
    }

    pub(crate) fn add_json(&mut self, value: &Value) {
        match value {
            Value::String(text) => self.add_value(text),
            Value::Array(items) => items.iter().for_each(|item| self.add_json(item)),
            Value::Object(map) => map.values().for_each(|item| self.add_json(item)),
            _ => {}
        }
    }

    pub(crate) fn add_field(&mut self, field: impl Into<String>) {
        self.fields.insert(field.into());
    }

    pub(crate) fn add_strong_json(&mut self, value: &Value) {
        for text in json_texts(value) {
            let text = text.trim().to_lowercase();
            if text.chars().count() >= MIN_REDACTED_VALUE_LENGTH {
                self.mentions.insert(text.clone());
            }
            let digits: String = text.chars().filter(char::is_ascii_digit).collect();
            if digits.len() >= MIN_IDENTIFIER_DIGITS && is_phone_like(&text) {
                self.mentions.insert(digits);
            }
        }
    }

    pub(crate) fn add_identifiers_json(&mut self, value: &Value) {
        for text in json_texts(value) {
            let text = text.trim();
            if is_identifier(text) || is_email_like(text) {
                self.add_strong_json(&Value::String(text.to_owned()));
            }
        }
    }

    pub(crate) fn add_full_name(&mut self, value: &str) {
        let words: Vec<&str> = value.split_whitespace().collect();
        if words.len() >= 2
            && words
                .iter()
                .all(|word| word.chars().count() >= MIN_NAME_WORD_LENGTH)
        {
            self.mentions.insert(words.join(" ").to_lowercase());
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty() && self.fields.is_empty()
    }

    pub(crate) fn mentions_only(&self) -> Self {
        Self {
            values: self.mentions.clone(),
            fields: BTreeSet::new(),
            mentions: BTreeSet::new(),
        }
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &str> {
        self.values.iter().map(String::as_str)
    }

    pub(crate) fn apply_text(&self, text: &str) -> Option<String> {
        if let Ok(mut parsed) = serde_json::from_str::<Value>(text) {
            return self.apply(&mut parsed).then(|| parsed.to_string());
        }
        let mut result = text.to_owned();
        let mut changed = false;
        for value in &self.values {
            let mut cursor = 0;
            loop {
                let lowered = result.to_lowercase();
                if lowered.len() != result.len() {
                    return Some(REDACTED.to_owned());
                }
                let Some(start) = find_whole(&lowered, value, cursor) else {
                    break;
                };
                result.replace_range(start..start + value.len(), REDACTED);
                cursor = start + REDACTED.len();
                changed = true;
            }
        }
        changed.then_some(result)
    }

    fn matches_text(&self, text: &str) -> bool {
        if text == REDACTED {
            return false;
        }
        let lowered = text.to_lowercase();
        self.values
            .iter()
            .any(|value| find_whole(&lowered, value, 0).is_some())
    }

    pub(crate) fn apply(&self, value: &mut Value) -> bool {
        match value {
            Value::String(text) if self.matches_text(text) => {
                *text = REDACTED.to_owned();
                true
            }
            Value::Array(items) => {
                let mut changed = false;
                for item in items.iter_mut() {
                    changed |= self.apply(item);
                }
                changed
            }
            Value::Object(map) => {
                let field_redacted = map
                    .get("field")
                    .and_then(Value::as_str)
                    .is_some_and(|field| self.fields.contains(field));
                let mut changed = false;
                for (key, item) in map.iter_mut() {
                    let redact_whole = (field_redacted && (key == "old" || key == "new"))
                        || self.fields.contains(key.as_str());
                    if redact_whole && !item.is_null() {
                        if item.as_str() != Some(REDACTED) {
                            *item = Value::String(REDACTED.to_owned());
                            changed = true;
                        }
                    } else {
                        changed = self.apply(item) || changed;
                    }
                }
                changed
            }
            _ => false,
        }
    }
}
