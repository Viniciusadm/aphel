pub(crate) mod commands;
pub(crate) mod ports;
pub(crate) mod use_cases;

pub use commands::*;
pub use ports::*;
pub use use_cases::*;

pub mod modules;

pub trait InstallerBlueprintSource: Send + Sync {
    fn profiles(&self) -> Vec<modules::InstallerProfile>;
    fn packages(&self, business: Option<&serde_json::Value>) -> Vec<String>;
    fn identity(&self, key: &str) -> Option<modules::InstallerIdentity>;
    fn roles(&self, business: Option<&serde_json::Value>) -> Vec<modules::InstallerRole>;
    fn theme(&self, business: Option<&serde_json::Value>) -> Option<serde_json::Value>;
    fn validate(
        &self,
        input: Option<serde_json::Value>,
        violations: &mut Vec<crate::errors::FieldViolation>,
    ) -> Option<serde_json::Value>;
}
pub fn blueprint_input(
    mut input: Option<serde_json::Value>,
    violations: &mut Vec<crate::errors::FieldViolation>,
) -> Option<serde_json::Value> {
    let sources = crate::modules::app().contributions::<dyn InstallerBlueprintSource>();
    if sources.is_empty() && input.is_some() {
        violations.push(crate::errors::FieldViolation::new(
            "business",
            "this build has no installer blueprint",
        ));
        return None;
    }
    for source in sources {
        input = source.validate(input, violations);
    }
    input
}

#[async_trait::async_trait]
pub trait BlueprintConfigurator: Send + Sync {
    async fn apply_template(
        &self,
        ctx: &crate::Ctx<'_>,
        template: &str,
    ) -> Result<(), crate::errors::AppError>;
}

pub async fn apply_blueprint_configuration(
    ctx: &crate::Ctx<'_>,
    business: Option<&serde_json::Value>,
) -> Result<(), crate::errors::AppError> {
    let app = crate::modules::app();
    let configurators = app.contributions::<dyn BlueprintConfigurator>();
    let themes = app.contributions::<dyn BlueprintThemeInstaller>();
    for source in app.contributions::<dyn InstallerBlueprintSource>() {
        for template in source.packages(business) {
            for configurator in &configurators {
                configurator.apply_template(ctx, &template).await?;
            }
        }
        if let Some(theme) = source.theme(business) {
            for installer in &themes {
                installer.apply_theme(ctx, theme.clone()).await?;
            }
        }
    }
    Ok(())
}

#[async_trait::async_trait]
pub trait BlueprintThemeInstaller: Send + Sync {
    async fn apply_theme(
        &self,
        ctx: &crate::Ctx<'_>,
        package: serde_json::Value,
    ) -> Result<(), crate::errors::AppError>;
}
