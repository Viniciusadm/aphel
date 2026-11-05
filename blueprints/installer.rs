use serde::{Deserialize, Serialize};

use aphel_kernel::errors::FieldViolation;

pub const OFFERINGS: [&str; 3] = ["catalog_items.item", "service", "recurring"];
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

static CATALOG: std::sync::OnceLock<aphel_blueprint::Catalog> = std::sync::OnceLock::new();

fn catalog() -> &'static aphel_blueprint::Catalog {
    CATALOG.get_or_init(|| aphel_blueprint::embedded().expect("valid embedded blueprints"))
}

pub fn installer_profiles() -> Vec<aphel_kernel::application::installer::modules::InstallerProfile>
{
    catalog()
        .resolve_all()
        .expect("valid embedded blueprints")
        .into_iter()
        .map(
            |blueprint| aphel_kernel::application::installer::modules::InstallerProfile {
                key: blueprint.key.clone(),
                required: blueprint.required.clone(),
                modules: blueprint.plugins().map(str::to_owned).collect(),
            },
        )
        .collect()
}

struct BlueprintInstaller;
impl aphel_kernel::application::installer::InstallerBlueprintSource for BlueprintInstaller {
    fn profiles(&self) -> Vec<aphel_kernel::application::installer::modules::InstallerProfile> {
        installer_profiles()
    }
    fn packages(&self, business: Option<&serde_json::Value>) -> Vec<String> {
        let Some(key) = business
            .and_then(|business| business.get("business_type"))
            .and_then(serde_json::Value::as_str)
        else {
            return Vec::new();
        };
        catalog()
            .resolve(key)
            .map(|blueprint| blueprint.configuration.packages)
            .unwrap_or_default()
    }
    fn identity(
        &self,
        key: &str,
    ) -> Option<aphel_kernel::application::installer::modules::InstallerIdentity> {
        let blueprint = catalog().resolve(key).ok()?;
        Some(
            aphel_kernel::application::installer::modules::InstallerIdentity {
                key: blueprint.key,
                name: blueprint.name,
                home: blueprint.navigation.home,
                sections: blueprint
                    .navigation
                    .sections
                    .into_iter()
                    .map(|section| {
                        aphel_kernel::application::installer::modules::InstallerSection {
                            key: section.key,
                            label: serde_json::to_value(section.label).unwrap_or_default(),
                            items: section.items,
                        }
                    })
                    .collect(),
            },
        )
    }
    fn theme(&self, business: Option<&serde_json::Value>) -> Option<serde_json::Value> {
        let key = business
            .and_then(|business| business.get("business_type"))
            .and_then(serde_json::Value::as_str)?;
        let path = catalog().resolve(key).ok()?.configuration.theme?;
        serde_json::from_str(&aphel_blueprint::embedded_file(&path)?).ok()
    }
    fn roles(
        &self,
        business: Option<&serde_json::Value>,
    ) -> Vec<aphel_kernel::application::installer::modules::InstallerRole> {
        let Some(key) = business
            .and_then(|business| business.get("business_type"))
            .and_then(serde_json::Value::as_str)
        else {
            return Vec::new();
        };
        catalog()
            .resolve(key)
            .map(|blueprint| {
                blueprint
                    .roles
                    .into_iter()
                    .map(
                        |role| aphel_kernel::application::installer::modules::InstallerRole {
                            key: role.key,
                            label: role.label,
                            permissions: role.permissions,
                        },
                    )
                    .collect()
            })
            .unwrap_or_default()
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
