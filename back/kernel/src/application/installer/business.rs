use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::FieldViolation;

pub const OFFERINGS: [&str; 3] = ["product", "service", "recurring"];
pub const TEAM_SIZES: [&str; 4] = ["solo", "small", "medium", "large"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct BusinessAnswers {
    #[schema(example = "agency")]
    pub business_type: Option<String>,
    #[serde(default)]
    #[schema(example = json!(["service", "recurring"]))]
    pub offering: Vec<String>,
    #[schema(example = "small")]
    pub team_size: Option<String>,
}

impl BusinessAnswers {
    pub fn is_empty(&self) -> bool {
        self.business_type.is_none() && self.offering.is_empty() && self.team_size.is_none()
    }
}

pub fn validate_business(
    answers: Option<BusinessAnswers>,
    violations: &mut Vec<FieldViolation>,
) -> Option<BusinessAnswers> {
    let answers = answers?;
    let business_type = answers
        .business_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(value) = business_type.as_deref()
        && !super::modules::profiles()
            .iter()
            .any(|profile| profile.key == value)
    {
        violations.push(FieldViolation::new(
            "business.business_type",
            "business type was not found",
        ));
    }
    let mut offering: Vec<String> = answers
        .offering
        .iter()
        .map(|value| value.trim().to_owned())
        .collect();
    offering.sort();
    offering.dedup();
    if offering
        .iter()
        .any(|value| !OFFERINGS.contains(&value.as_str()))
    {
        violations.push(FieldViolation::new(
            "business.offering",
            format!("must contain only {}", OFFERINGS.join(", ")),
        ));
    }
    let team_size = answers
        .team_size
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if team_size
        .as_deref()
        .is_some_and(|value| !TEAM_SIZES.contains(&value))
    {
        violations.push(FieldViolation::new(
            "business.team_size",
            format!("must be one of {}", TEAM_SIZES.join(", ")),
        ));
    }
    let answers = BusinessAnswers {
        business_type,
        offering,
        team_size,
    };
    (!answers.is_empty()).then_some(answers)
}
