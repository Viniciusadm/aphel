use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use aphel_sdk::configuration::parse_version;
use aphel_sdk::configuration::versioning::{is_package_id, parse_package_version};

pub const CATALOG_FORMAT: &str = "aphel-catalog";
pub const CATALOG_FORMAT_VERSION: u32 = 1;
pub const MAX_CATALOG_ENTRIES: usize = 1000;
const MAX_TEXT_LENGTH: usize = 2000;
const MAX_NAME_LENGTH: usize = 140;
const MAX_URL_LENGTH: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogEntryKind {
    Template,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogPackageRef {
    pub url: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogModuleRef {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub id: String,
    pub kind: CatalogEntryKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub author: String,
    pub version: String,
    pub min_aphel_version: String,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<CatalogPackageRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<CatalogModuleRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedEntry {
    pub position: usize,
    pub id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogIndex {
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub updated_at: Option<String>,
    pub entries: Vec<CatalogEntry>,
    pub rejected: Vec<RejectedEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIndex {
    #[serde(rename = "$schema", default)]
    _schema: Option<String>,
    format: String,
    format_version: u32,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    entries: Vec<Value>,
}

pub fn parse_index(bytes: &[u8], allow_http: bool) -> Result<CatalogIndex, String> {
    let raw: RawIndex = serde_json::from_slice(bytes)
        .map_err(|error| format!("the catalog index is not valid: {error}"))?;
    if raw.format != CATALOG_FORMAT {
        return Err(format!(
            "the catalog index format must be \"{CATALOG_FORMAT}\""
        ));
    }
    if raw.format_version == 0 || raw.format_version > CATALOG_FORMAT_VERSION {
        return Err(format!(
            "this Aphel reads catalog format versions up to {CATALOG_FORMAT_VERSION}"
        ));
    }
    if raw.entries.len() > MAX_CATALOG_ENTRIES {
        return Err(format!(
            "a catalog index can have at most {MAX_CATALOG_ENTRIES} entries"
        ));
    }
    let mut entries = Vec::new();
    let mut rejected = Vec::new();
    let mut seen = BTreeSet::new();
    for (position, value) in raw.entries.into_iter().enumerate() {
        let id = value.get("id").and_then(Value::as_str).map(str::to_owned);
        let entry = serde_json::from_value::<CatalogEntry>(value)
            .map_err(|error| error.to_string())
            .and_then(|entry| validate_entry(entry, allow_http));
        match entry {
            Ok(entry) if !seen.insert(entry.id.clone()) => rejected.push(RejectedEntry {
                position,
                id,
                reason: "the id is declared more than once".to_owned(),
            }),
            Ok(entry) => entries.push(entry),
            Err(reason) => rejected.push(RejectedEntry {
                position,
                id,
                reason,
            }),
        }
    }
    Ok(CatalogIndex {
        name: raw.name,
        description: raw.description,
        homepage: raw.homepage,
        updated_at: raw.updated_at,
        entries,
        rejected,
    })
}

pub fn is_relative_path(path: &str) -> bool {
    path.len() <= MAX_URL_LENGTH
        && !path.is_empty()
        && !path.starts_with('/')
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with('.')
                && segment.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
                })
        })
}

pub fn resolve_package_url(source: &str, url: &str) -> String {
    if !is_relative_path(url) {
        return url.to_owned();
    }
    let base_end = source
        .rfind('/')
        .filter(|position| !source[..*position].ends_with('/'))
        .or_else(|| source.find(':'))
        .map_or(0, |position| position + 1);
    format!("{}{url}", &source[..base_end])
}

pub fn is_allowed_url(url: &str, allow_http: bool) -> bool {
    url.len() <= MAX_URL_LENGTH
        && (url.starts_with("https://") || (allow_http && url.starts_with("http://")))
        && url.split("://").nth(1).is_some_and(|rest| {
            !rest.is_empty() && !rest.starts_with('/') && !rest.contains(char::is_whitespace)
        })
}

fn validate_entry(entry: CatalogEntry, allow_http: bool) -> Result<CatalogEntry, String> {
    if !is_package_id(&entry.id) {
        return Err(
            "id must use lowercase letters or digits separated by dots, dashes or underscores"
                .to_owned(),
        );
    }
    let length = |value: &str, max: usize| !value.trim().is_empty() && value.chars().count() <= max;
    if !length(&entry.name, MAX_NAME_LENGTH) {
        return Err(format!(
            "name must have between 1 and {MAX_NAME_LENGTH} characters"
        ));
    }
    if !length(&entry.author, MAX_NAME_LENGTH) {
        return Err(format!(
            "author must have between 1 and {MAX_NAME_LENGTH} characters"
        ));
    }
    if !length(&entry.license, MAX_NAME_LENGTH) {
        return Err("license must be an SPDX identifier such as AGPL-3.0-or-later".to_owned());
    }
    if entry
        .description
        .as_ref()
        .is_some_and(|description| description.chars().count() > MAX_TEXT_LENGTH)
    {
        return Err(format!(
            "description must have at most {MAX_TEXT_LENGTH} characters"
        ));
    }
    if parse_package_version(&entry.version).is_none() {
        return Err("version must be a semantic version such as 1.0.0".to_owned());
    }
    if parse_version(&entry.min_aphel_version).is_none() {
        return Err("min_aphel_version must be a version such as 0.1.0".to_owned());
    }
    for url in [&entry.homepage, &entry.documentation_url]
        .into_iter()
        .flatten()
    {
        if !is_allowed_url(url, allow_http) {
            return Err(format!("{url} must be an https URL"));
        }
    }
    match entry.kind {
        CatalogEntryKind::Template => {
            let Some(package) = &entry.package else {
                return Err("a template needs package.url and package.sha256".to_owned());
            };
            if entry.module.is_some() {
                return Err("a template cannot declare module".to_owned());
            }
            if !is_allowed_url(&package.url, allow_http) && !is_relative_path(&package.url) {
                return Err(
                    "package.url must be an https URL or a path relative to the index".to_owned(),
                );
            }
            if package.sha256.len() != 64
                || !package
                    .sha256
                    .chars()
                    .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
            {
                return Err("package.sha256 must be 64 lowercase hexadecimal characters".to_owned());
            }
        }
        CatalogEntryKind::Module => {
            let Some(module) = &entry.module else {
                return Err("a module entry needs module.key".to_owned());
            };
            if entry.package.is_some() {
                return Err("modules are never downloaded: remove package".to_owned());
            }
            let valid_key = !module.key.is_empty()
                && module.key.len() <= 64
                && module.key.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                });
            if !valid_key {
                return Err("module.key must be snake_case".to_owned());
            }
            if module
                .repository
                .as_ref()
                .is_some_and(|url| !is_allowed_url(url, allow_http))
            {
                return Err("module.repository must be an https URL".to_owned());
            }
            if module
                .install_notes
                .as_ref()
                .is_some_and(|notes| notes.chars().count() > MAX_TEXT_LENGTH)
            {
                return Err(format!(
                    "module.install_notes must have at most {MAX_TEXT_LENGTH} characters"
                ));
            }
        }
    }
    Ok(entry)
}
