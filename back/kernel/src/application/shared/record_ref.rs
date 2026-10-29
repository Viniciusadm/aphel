use std::borrow::Cow;
use std::fmt;

use uuid::Uuid;

use crate::errors::AppError;

const MAX_RECORD_TYPE_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordType(Cow<'static, str>);

impl RecordType {
    pub const CONTACT: Self = Self::from_static("contact");
    pub const COMPANY: Self = Self::from_static("company");
    pub const DEAL: Self = Self::from_static("deal");
    pub const ACTIVITY: Self = Self::from_static("activity");
    pub const PRODUCT: Self = Self::from_static("product");
    pub const NOTE: Self = Self::from_static("note");
    pub const PIPELINE: Self = Self::from_static("pipeline");
    pub const PIPELINE_STAGE: Self = Self::from_static("pipeline_stage");
    pub const LOST_REASON: Self = Self::from_static("lost_reason");
    pub const ACTIVITY_TYPE: Self = Self::from_static("activity_type");
    pub const FILE: Self = Self::from_static("file");
    pub const MEMBER: Self = Self::from_static("member");
    pub const ROLE: Self = Self::from_static("role");
    pub const TEAM: Self = Self::from_static("team");
    pub const ORGANIZATION: Self = Self::from_static("organization");

    pub const fn from_static(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, AppError> {
        let value = value.into();
        let mut characters = value.chars();
        let valid = value.len() <= MAX_RECORD_TYPE_LENGTH
            && characters
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
            && characters.all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            });
        if !valid {
            return Err(AppError::Validation(format!(
                "record type must be lowercase letters, digits or underscores and start with a letter (max {MAX_RECORD_TYPE_LENGTH})"
            )));
        }
        Ok(Self(Cow::Owned(value)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordRef {
    pub record_type: RecordType,
    pub record_id: Uuid,
}

impl RecordRef {
    pub fn new(record_type: RecordType, record_id: Uuid) -> Self {
        Self {
            record_type,
            record_id,
        }
    }

    pub fn organization() -> Self {
        Self::new(RecordType::ORGANIZATION, Uuid::nil())
    }

    pub fn parse(record_type: &str, record_id: &str) -> Result<Self, AppError> {
        let record_id = Uuid::parse_str(record_id)
            .map_err(|_| AppError::Validation("record id must be a UUID".to_owned()))?;
        Ok(Self::new(RecordType::parse(record_type)?, record_id))
    }
}
