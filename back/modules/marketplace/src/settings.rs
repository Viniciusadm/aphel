use crate::application::{BUILTIN_CATALOG_SOURCE, CatalogSettings};
use aphel_kernel::application::organizations::{OrganizationConfig, OrganizationConfigExtension};
use aphel_kernel::errors::FieldViolation;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatalogMode {
    #[default]
    Builtin,
    Remote,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatalogSource {
    pub mode: CatalogMode,
    pub index_url: Option<String>,
    pub allow_http: bool,
    pub timeout_secs: u64,
}

impl Default for CatalogSource {
    fn default() -> Self {
        Self {
            mode: CatalogMode::Builtin,
            index_url: None,
            allow_http: false,
            timeout_secs: 10,
        }
    }
}

pub fn catalog_settings(config: &OrganizationConfig) -> CatalogSettings {
    let catalog: CatalogSource = config
        .extensions
        .get("catalog")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    CatalogSettings {
        index_url: match catalog.mode {
            CatalogMode::Builtin => Some(BUILTIN_CATALOG_SOURCE.to_owned()),
            CatalogMode::Remote => catalog.index_url,
            CatalogMode::Off => None,
        },
        allow_http: catalog.allow_http,
        timeout: Duration::from_secs(catalog.timeout_secs),
    }
}

pub struct SourceConfiguration;
impl OrganizationConfigExtension for SourceConfiguration {
    fn defaults(&self) -> (String, Value) {
        (
            "catalog".to_owned(),
            serde_json::to_value(CatalogSource::default()).expect("catalog settings"),
        )
    }
    fn validate(&self, values: &std::collections::BTreeMap<String, Value>) -> Vec<FieldViolation> {
        let Some(value) = values.get("catalog") else {
            return Vec::new();
        };
        let Ok(source) = serde_json::from_value::<CatalogSource>(value.clone()) else {
            return vec![FieldViolation::new(
                "catalog",
                "invalid catalog configuration",
            )];
        };
        let mut errors = Vec::new();
        if !(1..=120).contains(&source.timeout_secs) {
            errors.push(FieldViolation::new(
                "catalog.timeout_secs",
                "must be between 1 and 120",
            ));
        }
        if source.mode == CatalogMode::Remote
            && !source
                .index_url
                .as_deref()
                .is_some_and(|url| crate::application::is_allowed_url(url, source.allow_http))
        {
            errors.push(FieldViolation::new(
                "catalog.index_url",
                "must be an allowed catalog URL",
            ));
        }
        errors
    }
}
