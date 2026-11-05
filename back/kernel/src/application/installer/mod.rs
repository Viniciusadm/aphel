pub(crate) mod commands;
pub(crate) mod ports;
pub(crate) mod use_cases;

pub use commands::*;
pub use ports::*;
pub use use_cases::*;

pub mod modules;

pub trait InstallerBlueprintSource: Send + Sync {
    fn profiles(&self) -> Vec<modules::InstallerProfile>;
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
