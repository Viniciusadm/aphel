use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const APPEARANCE_ENTITY: &str = "site.appearance";
pub const APPEARANCE_EDIT: &str = "site.appearance.edit";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceProperty {
    pub key: String,
    pub group: String,
    pub kind: String,
    pub default: Value,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppearanceValues(pub BTreeMap<String, Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearancePreset {
    pub key: String,
    pub values: AppearanceValues,
}

pub fn appearance_properties() -> Vec<AppearanceProperty> {
    let property = |key: &str, group: &str, kind: &str, default: Value| AppearanceProperty {
        key: key.into(),
        group: group.into(),
        kind: kind.into(),
        default,
        options: vec![],
    };
    let mut properties = vec![
        property("sticky_header", "frame", "boolean", false.into()),
        property("name", "identity", "text", Value::Null),
        property("tagline", "identity", "text", Value::Null),
        property("logo", "identity", "file", Value::Null),
        property("dark_logo", "identity", "file", Value::Null),
        property("icon", "identity", "file", Value::Null),
        property("variation", "identity", "variation", "default".into()),
        property(
            "styles",
            "styles",
            "styles",
            Value::Object(Default::default()),
        ),
        property("theme", "identity", "theme", "".into()),
    ];
    for key in ["header", "footer", "translations"] {
        properties.push(property(key, "frame", "frame", Value::Null));
    }
    properties
}

pub fn appearance_presets() -> Vec<AppearancePreset> {
    crate::styles::style_variations()
        .into_iter()
        .map(|(key, _)| AppearancePreset {
            key: key.into(),
            values: AppearanceValues([("variation".to_owned(), Value::from(key))].into()),
        })
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeAppearance {
    #[serde(default)]
    pub defaults: AppearanceValues,
    #[serde(default)]
    pub properties: Vec<String>,
}
