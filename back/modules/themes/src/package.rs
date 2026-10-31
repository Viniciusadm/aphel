use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

pub(crate) const FORMAT: &str = "aphel-theme";
pub(crate) const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ThemePackage {
    pub(crate) format: String,
    pub(crate) format_version: u32,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) author: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    pub(crate) license: String,
    #[serde(default)]
    pub(crate) min_aphel_version: Option<String>,
    #[serde(default)]
    pub(crate) tokens: TokenSets,
    #[serde(default)]
    pub(crate) fonts: Vec<FontFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TokenSets {
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) shared: BTreeMap<String, Value>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) light: BTreeMap<String, Value>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) dark: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FontFile {
    pub(crate) family: String,
    pub(crate) weight: u16,
    #[serde(default = "normal_style")]
    pub(crate) style: String,
    #[serde(default = "woff2_format")]
    pub(crate) format: String,
    pub(crate) data: String,
}

fn normal_style() -> String {
    "normal".to_owned()
}

fn woff2_format() -> String {
    "woff2".to_owned()
}
