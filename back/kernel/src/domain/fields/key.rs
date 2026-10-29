use std::fmt;

use crate::domain::DomainValidationError;

pub const MAX_FIELD_KEY_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldKey(String);

impl FieldKey {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if !is_identifier(&value, MAX_FIELD_KEY_LENGTH) {
            return Err(DomainValidationError::Invalid("field_key"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FieldKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn is_identifier(value: &str, max_length: usize) -> bool {
    let mut characters = value.chars();
    value.len() <= max_length
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        && !value.ends_with('_')
        && !value.contains("__")
}
