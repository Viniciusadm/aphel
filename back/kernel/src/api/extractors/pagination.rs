use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::api::error::ApiError;
use crate::errors::{AppError, FieldViolation};

pub const DEFAULT_PAGE: u32 = 1;
pub const DEFAULT_PER_PAGE: u32 = 25;
pub const MAX_PER_PAGE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationQuery {
    #[param(value_type = Option<u32>, minimum = 1, default = 1)]
    pub page: Option<String>,
    #[param(value_type = Option<u32>, minimum = 1, maximum = 100, default = 25)]
    pub per_page: Option<String>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: DEFAULT_PAGE,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl Pagination {
    pub fn parse(page: Option<&str>, per_page: Option<&str>) -> Result<Self, AppError> {
        let mut violations = Vec::new();

        let page = parse_bounded(page, DEFAULT_PAGE, 1, u32::MAX).unwrap_or_else(|| {
            violations.push(FieldViolation::new(
                "page",
                "must be an integer greater than or equal to 1",
            ));
            DEFAULT_PAGE
        });
        let per_page =
            parse_bounded(per_page, DEFAULT_PER_PAGE, 1, MAX_PER_PAGE).unwrap_or_else(|| {
                violations.push(FieldViolation::new(
                    "per_page",
                    format!("must be an integer between 1 and {MAX_PER_PAGE}"),
                ));
                DEFAULT_PER_PAGE
            });

        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid pagination parameters",
                violations,
            ));
        }

        Ok(Self { page, per_page })
    }

    pub fn offset(&self) -> u64 {
        u64::from(self.page - 1) * u64::from(self.per_page)
    }

    pub fn limit(&self) -> u64 {
        u64::from(self.per_page)
    }
}

fn parse_bounded(raw: Option<&str>, default: u32, min: u32, max: u32) -> Option<u32> {
    match raw {
        None => Some(default),
        Some(value) => value
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|parsed| (min..=max).contains(parsed)),
    }
}

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<PaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| AppError::Validation(rejection.body_text()))?;

        Ok(Self::parse(
            query.page.as_deref(),
            query.per_page.as_deref(),
        )?)
    }
}
