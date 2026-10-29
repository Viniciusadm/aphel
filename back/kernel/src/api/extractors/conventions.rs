use std::collections::BTreeSet;

use axum::extract::FromRequestParts;
use axum::http::HeaderName;
use axum::http::request::Parts;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::api::error::ApiError;
use crate::errors::{AppError, FieldViolation};

pub const IF_MATCH: HeaderName = HeaderName::from_static("if-match");

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IncludeQuery {
    #[param(example = "company,owner")]
    pub include: Option<String>,
}

impl IncludeQuery {
    pub fn parse(&self, allowed: &[&str]) -> Result<Includes, AppError> {
        Includes::parse(self.include.as_deref(), allowed)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Includes(BTreeSet<String>);

impl Includes {
    pub fn parse(raw: Option<&str>, allowed: &[&str]) -> Result<Self, AppError> {
        let mut names = BTreeSet::new();
        for name in raw
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            if !allowed.contains(&name) {
                return Err(AppError::invalid_fields(
                    "invalid include",
                    vec![FieldViolation::new(
                        "include",
                        format!("{name} is not available; allowed: {}", allowed.join(", ")),
                    )],
                ));
            }
            names.insert(name.to_owned());
        }
        Ok(Self(names))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IfMatch(pub Option<DateTime<Utc>>);

impl<S> FromRequestParts<S> for IfMatch
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(value) = parts.headers.get(IF_MATCH) else {
            return Ok(Self(None));
        };
        let parsed = value
            .to_str()
            .ok()
            .map(|text| text.trim().trim_start_matches("W/").trim_matches('"'))
            .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
            .map(|instant| instant.with_timezone(&Utc));
        match parsed {
            Some(instant) => Ok(Self(Some(instant))),
            None => Err(AppError::invalid_fields(
                "invalid If-Match header",
                vec![FieldViolation::new(
                    "If-Match",
                    "must be the updated_at of the resource in ISO-8601",
                )],
            )
            .into()),
        }
    }
}
