use aphel_kernel::errors::FieldViolation;
use aphel_sdk::platform::application::organization_profile::{
    OrganizationConfig, OrganizationConfigExtension,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SETTINGS_KEY: &str = "site";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteSettings {
    pub indexing: bool,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self { indexing: true }
    }
}

pub(crate) fn site_settings(config: &OrganizationConfig) -> SiteSettings {
    config
        .extensions
        .get(SETTINGS_KEY)
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub(crate) struct SiteConfiguration;

impl OrganizationConfigExtension for SiteConfiguration {
    fn defaults(&self) -> (String, Value) {
        (
            SETTINGS_KEY.to_owned(),
            serde_json::to_value(SiteSettings::default()).unwrap_or(Value::Null),
        )
    }

    fn validate(&self, values: &std::collections::BTreeMap<String, Value>) -> Vec<FieldViolation> {
        match values.get(SETTINGS_KEY) {
            Some(value) if serde_json::from_value::<SiteSettings>(value.clone()).is_err() => {
                vec![FieldViolation::new(
                    SETTINGS_KEY,
                    "invalid site configuration",
                )]
            }
            _ => Vec::new(),
        }
    }
}
