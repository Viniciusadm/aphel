use serde::{Deserialize, Serialize};

use aphel_kernel::errors::FieldViolation;

pub const OFFERINGS: [&str; 3] = ["products.product", "service", "recurring"];
pub const TEAM_SIZES: [&str; 4] = ["solo", "small", "medium", "large"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct BusinessAnswers {
    pub business_type: Option<String>,
    #[serde(default)]
    pub offering: Vec<String>,
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
        && !aphel_kernel::application::installer::modules::profiles()
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

struct BlueprintInstaller;
impl aphel_kernel::application::installer::InstallerBlueprintSource for BlueprintInstaller {
    fn profiles(&self) -> Vec<aphel_kernel::application::installer::modules::InstallerProfile> {
        serde_json::from_str(include_str!("profiles.json")).expect("valid blueprint profiles")
    }
    fn validate(
        &self,
        input: Option<serde_json::Value>,
        violations: &mut Vec<FieldViolation>,
    ) -> Option<serde_json::Value> {
        let input = input?;
        match serde_json::from_value(input) {
            Ok(answers) => validate_business(Some(answers), violations)
                .and_then(|value| serde_json::to_value(value).ok()),
            Err(error) => {
                violations.push(FieldViolation::new("business", error.to_string()));
                None
            }
        }
    }
}
pub fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.contribute::<dyn aphel_kernel::application::installer::InstallerBlueprintSource>(
        std::sync::Arc::new(BlueprintInstaller),
    );
}
