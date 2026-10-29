use chrono_tz::Tz;

use crate::errors::FieldViolation;

pub const MAX_ORGANIZATION_NAME_LENGTH: usize = 140;
pub const SUPPORTED_LOCALES: [&str; 2] = ["pt-BR", "en"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationPreferences {
    pub locale: String,
    pub timezone: String,
    pub currency: String,
}

pub fn validate_organization_name(
    value: &str,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > MAX_ORGANIZATION_NAME_LENGTH {
        violations.push(FieldViolation::new(
            field,
            format!("must have between 1 and {MAX_ORGANIZATION_NAME_LENGTH} characters"),
        ));
        return None;
    }
    Some(name.to_owned())
}

pub fn validate_preferences(
    locale: &str,
    timezone: &str,
    currency: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<OrganizationPreferences> {
    let before = violations.len();
    let locale = locale.trim();
    if !SUPPORTED_LOCALES.contains(&locale) {
        violations.push(FieldViolation::new("locale", "is not supported"));
    }
    let timezone = timezone.trim();
    if timezone.len() > 64 || timezone.parse::<Tz>().is_err() {
        violations.push(FieldViolation::new("timezone", "must be an IANA time zone"));
    }
    let currency = currency.trim().to_ascii_uppercase();
    if currency.len() != 3
        || !currency
            .chars()
            .all(|character| character.is_ascii_uppercase())
    {
        violations.push(FieldViolation::new("currency", "must be an ISO 4217 code"));
    }
    (violations.len() == before).then(|| OrganizationPreferences {
        locale: locale.to_owned(),
        timezone: timezone.to_owned(),
        currency,
    })
}

pub fn normalize_primary_color(
    value: Option<String>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let color = value
        .map(|color| color.trim().to_ascii_lowercase())
        .filter(|color| !color.is_empty())?;
    if !is_hex_color(&color) {
        violations.push(FieldViolation::new(
            "primary_color",
            "must be a color like #4f46e5",
        ));
        return None;
    }
    Some(color)
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}
