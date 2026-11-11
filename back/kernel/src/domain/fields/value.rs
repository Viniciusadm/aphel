use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use regex::RegexBuilder;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::UserId;
use crate::domain::fields::{
    FieldDefinition, FieldType, MAX_TEXT_LENGTH, PATTERN_SIZE_LIMIT, TextMask,
};

pub const MAX_NUMBER_ABS: i64 = 1_000_000_000_000_000_000;
pub const MAX_EMAIL_LENGTH: usize = 254;
pub const MAX_MULTI_SELECT_VALUES: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum RawValue {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<RawValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Text(String),
    LongText(String),
    Number(Decimal),
    Date(NaiveDate),
    DateTime(DateTime<Utc>),
    Bool(bool),
    Option(Uuid),
    Options(Vec<Uuid>),
    User(UserId),
    Record(Uuid),
    References(Vec<Uuid>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValueError {
    Required,
    ExpectedText,
    ExpectedNumber,
    ExpectedBool,
    ExpectedList,
    TooLong(u32),
    PatternMismatch,
    InvalidEmail,
    InvalidUrl,
    InvalidPhone,
    InvalidMasked(TextMask),
    InvalidDate,
    InvalidDateTime,
    BelowMinimum(Decimal),
    AboveMaximum(Decimal),
    TooManyDecimals(u32),
    OutOfRange,
    UnknownOption(String),
    InactiveOption(String),
    TooManyOptions(usize),
    InvalidReference,
    ReferenceNotFound,
    TypeUnavailable,
    NotUnique,
}

impl FieldValueError {
    pub fn message(&self) -> String {
        match self {
            Self::Required => "is required".to_owned(),
            Self::ExpectedText => "must be a string".to_owned(),
            Self::ExpectedNumber => "must be a number".to_owned(),
            Self::ExpectedBool => "must be true or false".to_owned(),
            Self::ExpectedList => "must be a list of option values".to_owned(),
            Self::TooLong(limit) => format!("must have at most {limit} characters"),
            Self::PatternMismatch => "does not match the expected format".to_owned(),
            Self::InvalidEmail => "must be a valid email address".to_owned(),
            Self::InvalidUrl => "must be a valid http or https URL".to_owned(),
            Self::InvalidPhone => "must be a valid phone number".to_owned(),
            Self::InvalidMasked(mask) => format!("must be a valid {} value", mask.as_str()),
            Self::InvalidDate => "must be a date in the format YYYY-MM-DD".to_owned(),
            Self::InvalidDateTime => "must be an RFC 3339 date and time".to_owned(),
            Self::BelowMinimum(min) => format!("must be greater than or equal to {min}"),
            Self::AboveMaximum(max) => format!("must be less than or equal to {max}"),
            Self::TooManyDecimals(decimals) => {
                format!("must have at most {decimals} decimal places")
            }
            Self::OutOfRange => "is out of the supported range".to_owned(),
            Self::UnknownOption(value) => format!("option {value:?} does not exist"),
            Self::InactiveOption(value) => format!("option {value:?} is inactive"),
            Self::TooManyOptions(limit) => format!("must have at most {limit} options"),
            Self::InvalidReference => "must be a valid id".to_owned(),
            Self::ReferenceNotFound => {
                "references a record that does not exist or that you cannot read".to_owned()
            }
            Self::TypeUnavailable => "field type provider is unavailable".to_owned(),
            Self::NotUnique => "is already used by another record".to_owned(),
        }
    }
}

impl RawValue {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Text(text) => text.trim().is_empty(),
            Self::List(items) => items.is_empty(),
            Self::Bool(_) | Self::Number(_) => false,
        }
    }
}

pub fn validate_value(
    definition: &FieldDefinition,
    raw: &RawValue,
) -> Result<Option<FieldValue>, FieldValueError> {
    if raw.is_empty() {
        return Ok(None);
    }
    if matches!(definition.field_type, FieldType::Custom { .. }) {
        let kind = crate::application::fields::field_types::find(definition.field_type.as_str())
            .ok_or(FieldValueError::TypeUnavailable)?;
        (kind.validate)(definition, raw)?;
    }
    validate_native_value(definition, raw)
}

pub fn validate_native_value(
    definition: &FieldDefinition,
    raw: &RawValue,
) -> Result<Option<FieldValue>, FieldValueError> {
    if raw.is_empty() {
        return Ok(None);
    }
    let config = &definition.config;
    let field_type = definition.field_type.base_type();
    let value = match field_type.base_type() {
        FieldType::Text | FieldType::LongText => {
            let text = expect_text(raw)?.trim().to_owned();
            let text = match config.mask {
                Some(mask) => mask
                    .normalize(&text)
                    .map_err(|_| FieldValueError::InvalidMasked(mask))?,
                None => text,
            };
            let limit = config.max_text_length(field_type);
            if text.chars().count() > limit as usize {
                return Err(FieldValueError::TooLong(limit));
            }
            if let Some(pattern) = &config.pattern {
                let regex = RegexBuilder::new(pattern)
                    .size_limit(PATTERN_SIZE_LIMIT)
                    .build()
                    .map_err(|_| FieldValueError::PatternMismatch)?;
                if !regex.is_match(&text) {
                    return Err(FieldValueError::PatternMismatch);
                }
            }
            if field_type.base_type() == FieldType::LongText {
                FieldValue::LongText(text)
            } else {
                FieldValue::Text(text)
            }
        }
        FieldType::Number | FieldType::Currency => {
            let number = parse_decimal(raw)?;
            if let Some(min) = config.min
                && number < min
            {
                return Err(FieldValueError::BelowMinimum(min));
            }
            if let Some(max) = config.max
                && number > max
            {
                return Err(FieldValueError::AboveMaximum(max));
            }
            let decimals = config.allowed_decimals(field_type);
            if number.normalize().scale() > decimals {
                return Err(FieldValueError::TooManyDecimals(decimals));
            }
            FieldValue::Number(number.normalize())
        }
        FieldType::Date => {
            let text = expect_text(raw)?.trim();
            FieldValue::Date(
                NaiveDate::parse_from_str(text, "%Y-%m-%d")
                    .map_err(|_| FieldValueError::InvalidDate)?,
            )
        }
        FieldType::DateTime => {
            let text = expect_text(raw)?.trim();
            FieldValue::DateTime(
                DateTime::parse_from_rfc3339(text)
                    .map_err(|_| FieldValueError::InvalidDateTime)?
                    .with_timezone(&Utc),
            )
        }
        FieldType::Checkbox => match raw {
            RawValue::Bool(value) => FieldValue::Bool(*value),
            _ => return Err(FieldValueError::ExpectedBool),
        },
        FieldType::Select => {
            let value = expect_text(raw)?.trim();
            FieldValue::Option(active_option(definition, value)?)
        }
        FieldType::MultiSelect => {
            let RawValue::List(items) = raw else {
                return Err(FieldValueError::ExpectedList);
            };
            if items.len() > MAX_MULTI_SELECT_VALUES {
                return Err(FieldValueError::TooManyOptions(MAX_MULTI_SELECT_VALUES));
            }
            let mut ids = Vec::with_capacity(items.len());
            for item in items {
                let id = active_option(definition, expect_text(item)?.trim())?;
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            FieldValue::Options(ids)
        }
        FieldType::Url => FieldValue::Text(normalize_url(expect_text(raw)?)?),
        FieldType::Email => FieldValue::Text(normalize_email(expect_text(raw)?)?),
        FieldType::Phone => {
            FieldValue::Text(normalize_phone(expect_text(raw)?, config.phone_country())?)
        }
        FieldType::User => {
            let id = parse_uuid(expect_text(raw)?)?;
            FieldValue::User(
                UserId::new(id.to_string()).map_err(|_| FieldValueError::InvalidReference)?,
            )
        }
        FieldType::Record => FieldValue::Record(parse_uuid(expect_text(raw)?)?),
        FieldType::References => {
            let RawValue::List(items) = raw else {
                return Err(FieldValueError::ExpectedList);
            };
            if items.len() > MAX_MULTI_SELECT_VALUES {
                return Err(FieldValueError::TooManyOptions(MAX_MULTI_SELECT_VALUES));
            }
            let mut ids = Vec::with_capacity(items.len());
            for item in items {
                let id = parse_uuid(expect_text(item)?)?;
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            FieldValue::References(ids)
        }
        FieldType::Custom { .. } => unreachable!("native field representation"),
    };
    Ok(Some(value))
}

fn expect_text(raw: &RawValue) -> Result<&str, FieldValueError> {
    match raw {
        RawValue::Text(text) => Ok(text),
        _ => Err(FieldValueError::ExpectedText),
    }
}

fn active_option(definition: &FieldDefinition, value: &str) -> Result<Uuid, FieldValueError> {
    let option = definition
        .option_by_value(value)
        .ok_or_else(|| FieldValueError::UnknownOption(value.to_owned()))?;
    if !option.is_active {
        return Err(FieldValueError::InactiveOption(value.to_owned()));
    }
    Ok(option.id)
}

fn parse_uuid(text: &str) -> Result<Uuid, FieldValueError> {
    Uuid::parse_str(text.trim()).map_err(|_| FieldValueError::InvalidReference)
}

pub fn parse_decimal(raw: &RawValue) -> Result<Decimal, FieldValueError> {
    let text = match raw {
        RawValue::Number(text) | RawValue::Text(text) => text.trim(),
        _ => return Err(FieldValueError::ExpectedNumber),
    };
    let number = Decimal::from_str(text)
        .or_else(|_| Decimal::from_scientific(text))
        .map_err(|_| FieldValueError::ExpectedNumber)?;
    if number.abs() >= Decimal::from(MAX_NUMBER_ABS) {
        return Err(FieldValueError::OutOfRange);
    }
    Ok(number)
}

pub fn normalize_email(text: &str) -> Result<String, FieldValueError> {
    let email = text.trim().to_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(FieldValueError::InvalidEmail);
    };
    let valid = email.len() <= MAX_EMAIL_LENGTH
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
        && !email.chars().any(|c| c.is_whitespace() || c.is_control());
    if !valid {
        return Err(FieldValueError::InvalidEmail);
    }
    Ok(email)
}

pub fn normalize_url(text: &str) -> Result<String, FieldValueError> {
    let trimmed = text.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let url = if lowered.starts_with("http://") || lowered.starts_with("https://") {
        trimmed.to_owned()
    } else if lowered.contains("://") {
        return Err(FieldValueError::InvalidUrl);
    } else {
        format!("https://{trimmed}")
    };
    let rest = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_default();
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    let host_name = host.split(':').next().unwrap_or_default();
    let valid = url.chars().count() <= MAX_TEXT_LENGTH as usize
        && !host_name.is_empty()
        && (host_name.contains('.') || host_name == "localhost")
        && !host_name.starts_with('.')
        && !host_name.ends_with('.')
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    if !valid {
        return Err(FieldValueError::InvalidUrl);
    }
    Ok(url)
}

pub fn normalize_phone(text: &str, default_country: &str) -> Result<String, FieldValueError> {
    let country = default_country.parse::<phonenumber::country::Id>().ok();
    let number =
        phonenumber::parse(country, text.trim()).map_err(|_| FieldValueError::InvalidPhone)?;
    if !phonenumber::is_valid(&number) {
        return Err(FieldValueError::InvalidPhone);
    }
    Ok(number.format().mode(phonenumber::Mode::E164).to_string())
}
