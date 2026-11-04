use std::cmp::Ordering;

use semver::Version;

use crate::{ConfigPackage, PackageIssue};

pub const MAX_PACKAGE_ID_LENGTH: usize = 128;
pub const MAX_PACKAGE_VERSION_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIdentity {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub locale: String,
    pub min_aphel_version: String,
}

impl PackageIdentity {
    pub fn of(package: &ConfigPackage) -> Option<Self> {
        Some(Self {
            id: package.id.clone()?,
            version: package.version.clone()?,
            name: package.name.clone(),
            description: package.description.clone(),
            author: package.author.clone(),
            locale: package.locale.clone(),
            min_aphel_version: package.min_aphel_version.clone(),
        })
    }
}

pub fn is_package_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PACKAGE_ID_LENGTH
        && value.split(['.', '-', '_']).all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
}

pub fn parse_package_version(value: &str) -> Option<Version> {
    (value.len() <= MAX_PACKAGE_VERSION_LENGTH)
        .then(|| Version::parse(value.trim()).ok())
        .flatten()
}

pub fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    Some(parse_package_version(left)?.cmp(&parse_package_version(right)?))
}

pub fn identity_issues(package: &ConfigPackage) -> Vec<PackageIssue> {
    let mut issues = Vec::new();
    match package.id.as_deref() {
        None => issues.push(PackageIssue::new(
            "id",
            "package_not_versioned",
            "a versioned package needs a stable id such as aphel.b2b",
        )),
        Some(id) if !is_package_id(id) => issues.push(PackageIssue::new(
            "id",
            "invalid_package_id",
            format!(
                "must have up to {MAX_PACKAGE_ID_LENGTH} lowercase letters or digits, separated by single dots, dashes or underscores"
            ),
        )),
        Some(_) => {}
    }
    match package.version.as_deref() {
        None => issues.push(PackageIssue::new(
            "version",
            "package_not_versioned",
            "a versioned package needs a semantic version such as 1.0.0",
        )),
        Some(version) if parse_package_version(version).is_none() => {
            issues.push(PackageIssue::new(
                "version",
                "invalid_version",
                "must be a semantic version such as 1.2.0",
            ))
        }
        Some(_) => {}
    }
    issues
}
