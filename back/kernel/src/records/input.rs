use serde_json::{Map, Value};
use uuid::Uuid;

use crate::application::fields::permissions::provided_key;
use crate::domain::UserId;
use crate::domain::fields::{normalize_email, normalize_phone, normalize_url};
use crate::errors::{AppError, FieldViolation};

pub type Patch<T> = Option<Option<T>>;

pub fn value_patch(values: &Map<String, Value>, key: &str) -> Patch<String> {
    values.get(key).map(|value| match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    })
}

pub fn bool_patch(values: &Map<String, Value>, key: &str) -> Patch<bool> {
    values.get(key).map(|value| match value {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    })
}

pub const DEFAULT_PHONE_COUNTRY: &str = "BR";
pub const MAX_LONG_TEXT_LENGTH: usize = 10_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddressInput {
    pub street: Patch<String>,
    pub number: Patch<String>,
    pub complement: Patch<String>,
    pub district: Patch<String>,
    pub city: Patch<String>,
    pub state: Patch<String>,
    pub postal_code: Patch<String>,
    pub country: Patch<String>,
}

impl AddressInput {
    pub fn from_values(values: &Map<String, Value>) -> Self {
        Self {
            street: value_patch(values, "address_street"),
            number: value_patch(values, "address_number"),
            complement: value_patch(values, "address_complement"),
            district: value_patch(values, "address_district"),
            city: value_patch(values, "address_city"),
            state: value_patch(values, "address_state"),
            postal_code: value_patch(values, "address_postal_code"),
            country: value_patch(values, "address_country"),
        }
    }
}

#[derive(Debug, Default)]
pub struct InputValidator {
    violations: Vec<FieldViolation>,
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

impl InputValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn violation(&mut self, field: &str, message: impl Into<String>) {
        self.violations.push(FieldViolation::new(field, message));
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    pub fn finish(mut self, message: &str) -> Result<(), AppError> {
        if self.violations.is_empty() {
            return Ok(());
        }
        self.violations
            .sort_by(|left, right| left.field.cmp(&right.field));
        Err(AppError::invalid_fields(message, self.violations))
    }

    pub fn into_violations(self) -> Vec<FieldViolation> {
        self.violations
    }

    pub fn required_text(&mut self, field: &str, value: Option<String>, max: usize) -> String {
        match blank_to_none(value) {
            None => {
                self.violation(field, "is required");
                String::new()
            }
            Some(text) => self.bounded(field, text, max).unwrap_or_default(),
        }
    }

    pub fn text(&mut self, field: &str, value: Option<String>, max: usize) -> Option<String> {
        blank_to_none(value).and_then(|text| self.bounded(field, text, max))
    }

    fn bounded(&mut self, field: &str, text: String, max: usize) -> Option<String> {
        if text.chars().count() > max {
            self.violation(field, format!("must have at most {max} characters"));
            return None;
        }
        Some(text)
    }

    pub fn email(&mut self, field: &str, value: Option<String>) -> Option<String> {
        let text = blank_to_none(value)?;
        normalize_email(&text)
            .map_err(|error| self.violation(field, error.message()))
            .ok()
    }

    pub fn phone(&mut self, field: &str, value: Option<String>) -> Option<String> {
        let text = blank_to_none(value)?;
        normalize_phone(&text, DEFAULT_PHONE_COUNTRY)
            .map_err(|error| self.violation(field, error.message()))
            .ok()
    }

    pub fn url(&mut self, field: &str, value: Option<String>) -> Option<String> {
        let text = blank_to_none(value)?;
        normalize_url(&text)
            .map_err(|error| self.violation(field, error.message()))
            .ok()
    }

    pub fn domain(&mut self, field: &str, value: Option<String>) -> Option<String> {
        let text = blank_to_none(value)?;
        let domain = normalize_domain(&text);
        if domain.is_none() {
            self.violation(field, "must be a valid domain, such as acme.com");
        }
        domain
    }

    pub fn country(&mut self, field: &str, value: Option<String>) -> Option<String> {
        let text = blank_to_none(value)?;
        let country = normalize_country(&text);
        if country.is_none() {
            self.violation(field, "must be a two-letter ISO country code");
        }
        country
    }

    pub fn user(&mut self, field: &str, value: Option<String>) -> Option<UserId> {
        let text = blank_to_none(value)?;
        match Uuid::parse_str(&text) {
            Ok(_) => UserId::new(text).ok(),
            Err(_) => {
                self.violation(field, "must be a user id");
                None
            }
        }
    }

    pub fn record_id(&mut self, field: &str, value: Option<String>) -> Option<Uuid> {
        let text = blank_to_none(value)?;
        Uuid::parse_str(&text)
            .map_err(|_| self.violation(field, "must be a record id"))
            .ok()
    }

    pub fn address(&mut self, target: &mut Address, input: AddressInput) {
        if let Some(value) = input.street {
            target.street = self.text("address_street", value, 255);
        }
        if let Some(value) = input.number {
            target.number = self.text("address_number", value, 32);
        }
        if let Some(value) = input.complement {
            target.complement = self.text("address_complement", value, 140);
        }
        if let Some(value) = input.district {
            target.district = self.text("address_district", value, 140);
        }
        if let Some(value) = input.city {
            target.city = self.text("address_city", value, 140);
        }
        if let Some(value) = input.state {
            target.state = self.text("address_state", value, 140);
        }
        if let Some(value) = input.postal_code {
            target.postal_code = self.text("address_postal_code", value, 32);
        }
        if let Some(value) = input.country {
            target.country = self.country("address_country", value);
        }
    }
}

impl AddressInput {
    pub fn provided_keys(&self, keys: &mut Vec<&'static str>) {
        provided_key(keys, "address_street", self.street.is_some());
        provided_key(keys, "address_number", self.number.is_some());
        provided_key(keys, "address_complement", self.complement.is_some());
        provided_key(keys, "address_district", self.district.is_some());
        provided_key(keys, "address_city", self.city.is_some());
        provided_key(keys, "address_state", self.state.is_some());
        provided_key(keys, "address_postal_code", self.postal_code.is_some());
        provided_key(keys, "address_country", self.country.is_some());
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    pub street: Option<String>,
    pub number: Option<String>,
    pub complement: Option<String>,
    pub district: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

pub fn normalize_country(text: &str) -> Option<String> {
    let country = text.trim().to_ascii_uppercase();
    (country.len() == 2 && country.chars().all(|c| c.is_ascii_uppercase())).then_some(country)
}

const MAX_DOMAIN_LENGTH: usize = 253;

pub fn normalize_domain(text: &str) -> Option<String> {
    let lowered = text.trim().to_ascii_lowercase();
    let without_scheme = lowered
        .split_once("://")
        .map_or(lowered.as_str(), |(_, rest)| rest);
    let authority = without_scheme.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    let host = host.trim_end_matches('.');
    let host = host.strip_prefix("www.").unwrap_or(host);
    let valid = !host.is_empty()
        && host.len() <= MAX_DOMAIN_LENGTH
        && host.contains('.')
        && !host.contains("..")
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_alphanumeric() || c == '-')
        });
    valid.then(|| host.to_owned())
}

const MIN_PHONE_DIGITS: usize = 4;
const MIN_FULL_PHONE_DIGITS: usize = 10;

pub fn normalize_search_term(term: &str) -> String {
    let trimmed = term.trim();
    let digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();
    let phone_like = digits.len() >= MIN_PHONE_DIGITS
        && trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '+' | '-' | '(' | ')' | '.'));
    if !phone_like {
        return trimmed.to_owned();
    }
    if digits.len() >= MIN_FULL_PHONE_DIGITS
        && let Ok(number) = normalize_phone(trimmed, DEFAULT_PHONE_COUNTRY)
    {
        return number;
    }
    digits
}

pub fn insert_text(map: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    map.insert(
        key.to_owned(),
        value.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
}

pub fn insert_address(map: &mut Map<String, Value>, address: &Address) {
    insert_text(map, "address_street", address.street.as_deref());
    insert_text(map, "address_number", address.number.as_deref());
    insert_text(map, "address_complement", address.complement.as_deref());
    insert_text(map, "address_district", address.district.as_deref());
    insert_text(map, "address_city", address.city.as_deref());
    insert_text(map, "address_state", address.state.as_deref());
    insert_text(map, "address_postal_code", address.postal_code.as_deref());
    insert_text(map, "address_country", address.country.as_deref());
}
