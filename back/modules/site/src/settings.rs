use aphel_kernel::errors::FieldViolation;
use aphel_sdk::platform::application::organization_profile::{
    OrganizationConfig, OrganizationConfigExtension,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SETTINGS_KEY: &str = "site";
const MAX_TAGLINE_CHARS: usize = 160;
const MAX_MENU_ENTRIES: usize = 100;
const MAX_LANGUAGES: usize = 5;
pub(crate) const SITE_LANGUAGES: &[&str] = &["pt-BR", "en", "es", "fr", "de", "it"];

pub(crate) fn language_slug(language: &str) -> String {
    language.to_ascii_lowercase()
}

pub(crate) fn field_suffix(language: &str) -> String {
    language.to_ascii_lowercase().replace('-', "_")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteSettings {
    pub indexing: bool,
    pub accounts: bool,
    pub theme: Option<String>,
    pub tagline: Option<String>,
    pub menu: Vec<MenuEntry>,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MenuEntry {
    pub item: String,
    #[serde(default)]
    pub hidden: bool,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            indexing: true,
            accounts: false,
            theme: None,
            tagline: None,
            menu: Vec::new(),
            languages: Vec::new(),
        }
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
        let Some(value) = values.get(SETTINGS_KEY) else {
            return Vec::new();
        };
        let Ok(settings) = serde_json::from_value::<SiteSettings>(value.clone()) else {
            return vec![FieldViolation::new(
                SETTINGS_KEY,
                "invalid site configuration",
            )];
        };
        let mut violations = Vec::new();
        if settings
            .tagline
            .as_deref()
            .is_some_and(|tagline| tagline.chars().count() > MAX_TAGLINE_CHARS)
        {
            violations.push(FieldViolation::new(
                "site.tagline",
                format!("must have at most {MAX_TAGLINE_CHARS} characters"),
            ));
        }
        if settings.languages.len() > MAX_LANGUAGES
            || settings
                .languages
                .iter()
                .any(|language| !SITE_LANGUAGES.contains(&language.as_str()))
        {
            violations.push(FieldViolation::new(
                "site.languages",
                format!(
                    "must have at most {MAX_LANGUAGES} of {}",
                    SITE_LANGUAGES.join(", ")
                ),
            ));
        }
        if settings.menu.len() > MAX_MENU_ENTRIES {
            violations.push(FieldViolation::new(
                "site.menu",
                format!("must have at most {MAX_MENU_ENTRIES} entries"),
            ));
        }
        violations
    }
}
