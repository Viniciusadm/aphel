use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::application::PackageIssue;
use aphel_kernel::application::organizations::SUPPORTED_LOCALES;

pub const PACKAGE_FORMAT: &str = "aphel-config";
pub const FORMAT_VERSION: u32 = 1;
pub const APHEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAX_PACKAGE_NAME_LENGTH: usize = 140;
pub const MAX_PACKAGE_DESCRIPTION_LENGTH: usize = 2000;
pub const MAX_PACKAGE_AUTHOR_LENGTH: usize = 140;
pub const MAX_PACKAGE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigPackage {
    pub format: String,
    pub format_version: u32,
    pub min_aphel_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub locale: String,
    #[serde(default)]
    pub sections: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageMetadata {
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub locale: String,
}

impl ConfigPackage {
    pub fn new(metadata: PackageMetadata) -> Self {
        Self {
            format: PACKAGE_FORMAT.to_owned(),
            format_version: FORMAT_VERSION,
            min_aphel_version: APHEL_VERSION.to_owned(),
            id: None,
            version: None,
            name: metadata.name,
            description: metadata.description,
            author: metadata.author,
            locale: metadata.locale,
            sections: BTreeMap::new(),
        }
    }

    pub fn parse(value: &Value) -> Result<Self, Vec<PackageIssue>> {
        if value.to_string().len() > MAX_PACKAGE_BYTES {
            return Err(vec![PackageIssue::new(
                "",
                "package_too_large",
                format!("a package can have at most {MAX_PACKAGE_BYTES} bytes"),
            )]);
        }
        let Value::Object(object) = value else {
            return Err(vec![PackageIssue::new(
                "",
                "invalid_package",
                "must be a JSON object",
            )]);
        };
        if object.get("format").and_then(Value::as_str) != Some(PACKAGE_FORMAT) {
            return Err(vec![PackageIssue::new(
                "format",
                "invalid_format",
                format!("must be \"{PACKAGE_FORMAT}\""),
            )]);
        }
        serde_json::from_value::<Self>(value.clone())
            .map_err(|error| vec![PackageIssue::new("", "invalid_package", error.to_string())])
    }

    pub fn section(&self, key: &str) -> Option<&Value> {
        self.sections.get(key).filter(|value| !value.is_null())
    }

    pub fn header_issues(&self) -> Vec<PackageIssue> {
        let mut issues = Vec::new();
        if self.format_version == 0 || self.format_version > FORMAT_VERSION {
            issues.push(PackageIssue::new(
                "format_version",
                "unsupported_format_version",
                format!("this Aphel reads format versions up to {FORMAT_VERSION}"),
            ));
        }
        match (
            parse_version(&self.min_aphel_version),
            parse_version(APHEL_VERSION),
        ) {
            (None, _) => issues.push(PackageIssue::new(
                "min_aphel_version",
                "invalid_version",
                "must be a version such as 0.1.0",
            )),
            (Some(required), Some(current)) if required > current => {
                issues.push(PackageIssue::new(
                    "min_aphel_version",
                    "aphel_too_old",
                    format!(
                        "the package requires Aphel {} or newer (this is {APHEL_VERSION})",
                        self.min_aphel_version
                    ),
                ))
            }
            _ => {}
        }
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > MAX_PACKAGE_NAME_LENGTH {
            issues.push(PackageIssue::new(
                "name",
                "invalid_name",
                format!("must have between 1 and {MAX_PACKAGE_NAME_LENGTH} characters"),
            ));
        }
        if self
            .description
            .as_ref()
            .is_some_and(|description| description.chars().count() > MAX_PACKAGE_DESCRIPTION_LENGTH)
        {
            issues.push(PackageIssue::new(
                "description",
                "invalid_description",
                format!("must have at most {MAX_PACKAGE_DESCRIPTION_LENGTH} characters"),
            ));
        }
        if self
            .author
            .as_ref()
            .is_some_and(|author| author.chars().count() > MAX_PACKAGE_AUTHOR_LENGTH)
        {
            issues.push(PackageIssue::new(
                "author",
                "invalid_author",
                format!("must have at most {MAX_PACKAGE_AUTHOR_LENGTH} characters"),
            ));
        }
        if !SUPPORTED_LOCALES.contains(&self.locale.as_str()) {
            issues.push(PackageIssue::new(
                "locale",
                "unsupported_locale",
                format!("must be one of {}", SUPPORTED_LOCALES.join(", ")),
            ));
        }
        issues
    }
}

pub fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((major, minor, patch))
}
