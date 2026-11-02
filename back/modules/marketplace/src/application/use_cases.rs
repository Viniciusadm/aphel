use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::Value;

use crate::application::{
    CatalogEntry, CatalogEntryKind, CatalogFetchError, CatalogFetcher, CatalogPorts,
    MAX_CATALOG_INDEX_BYTES, RejectedEntry, parse_index, resolve_package_url,
};
use ::config_packages::application::versioning::{
    ConfigVersionRepository, compare_versions, sha256_hex,
};
use ::config_packages::application::{APHEL_VERSION, MAX_PACKAGE_BYTES, parse_version};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;

pub const CATALOG_DISABLED: &str = "catalog_disabled";
pub const CATALOG_UNAVAILABLE: &str = "catalog_unavailable";
pub const CATALOG_ENTRY_NOT_FOUND: &str = "catalog_entry_not_found";
pub const CATALOG_ENTRY_NOT_INSTALLABLE: &str = "catalog_entry_not_installable";
pub const CATALOG_APHEL_TOO_OLD: &str = "aphel_too_old";
pub const CATALOG_CHECKSUM_MISMATCH: &str = "checksum_mismatch";
pub const CATALOG_PACKAGE_MISMATCH: &str = "catalog_package_mismatch";
pub const CATALOG_INVALID_PACKAGE: &str = "invalid_package";
pub const MAX_THEME_PACKAGE_BYTES: usize = 3 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntryView {
    pub entry: CatalogEntry,
    pub compatible: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogListing {
    pub enabled: bool,
    pub source: Option<String>,
    pub aphel_version: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub updated_at: Option<String>,
    pub entries: Vec<CatalogEntryView>,
    pub rejected: Vec<RejectedEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadedTemplate {
    pub entry: CatalogEntry,
    pub checksum: String,
    pub package: Value,
    pub origin: String,
}

pub struct CatalogUseCases<P> {
    ports: P,
}

impl<P> CatalogUseCases<P>
where
    P: CatalogPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list(&self, scope: &AccessScope) -> Result<CatalogListing, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let Some(source) = self.ports.settings().index_url.clone() else {
            return Ok(CatalogListing {
                enabled: false,
                source: None,
                aphel_version: APHEL_VERSION.to_owned(),
                name: None,
                description: None,
                homepage: None,
                updated_at: None,
                entries: Vec::new(),
                rejected: Vec::new(),
            });
        };
        let index = self.index(&source).await?;
        let installed = self.installed_versions().await?;
        let entries = index
            .entries
            .into_iter()
            .map(|entry| {
                let installed_version = (entry.kind == CatalogEntryKind::Template)
                    .then(|| installed.get(&entry.id).cloned())
                    .flatten();
                let update_available = installed_version.as_ref().is_some_and(|installed| {
                    compare_versions(&entry.version, installed) == Some(Ordering::Greater)
                });
                CatalogEntryView {
                    compatible: is_compatible(&entry),
                    installed_version,
                    update_available,
                    entry,
                }
            })
            .collect();
        Ok(CatalogListing {
            enabled: true,
            source: Some(source),
            aphel_version: APHEL_VERSION.to_owned(),
            name: Some(index.name),
            description: index.description,
            homepage: index.homepage,
            updated_at: index.updated_at,
            entries,
            rejected: index.rejected,
        })
    }

    pub async fn download_template(
        &self,
        scope: &AccessScope,
        entry_id: &str,
    ) -> Result<DownloadedTemplate, AppError> {
        self.download_package(scope, entry_id, CatalogEntryKind::Template)
            .await
    }

    pub async fn download_package(
        &self,
        scope: &AccessScope,
        entry_id: &str,
        expected: CatalogEntryKind,
    ) -> Result<DownloadedTemplate, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let source = self.ports.settings().index_url.clone().ok_or_else(|| {
            AppError::not_found_code(
                CATALOG_DISABLED,
                "the catalog is disabled in this installation",
            )
        })?;
        let index = self.index(&source).await?;
        let entry = index
            .entries
            .into_iter()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| {
                AppError::not_found_code(
                    CATALOG_ENTRY_NOT_FOUND,
                    format!("the catalog has no entry `{entry_id}`"),
                )
            })?;
        let package_ref = match (&entry.kind, &entry.package) {
            (kind, Some(package)) if *kind == expected => package.clone(),
            _ => {
                return Err(AppError::validation_code(
                    CATALOG_ENTRY_NOT_INSTALLABLE,
                    "modules are installed by adding them to aphel.modules.toml and compiling Aphel; see the instructions",
                ));
            }
        };
        if !is_compatible(&entry) {
            return Err(AppError::validation_code(
                CATALOG_APHEL_TOO_OLD,
                format!(
                    "{} {} requires Aphel {} or newer (this is {APHEL_VERSION})",
                    entry.name, entry.version, entry.min_aphel_version
                ),
            ));
        }
        let package_url = resolve_package_url(&source, &package_ref.url);
        let bytes = self
            .ports
            .fetcher()
            .fetch(&package_url, self.ports.settings(), package_limit(expected))
            .await
            .map_err(|error| unavailable("could not download the package", &error))?;
        let checksum = sha256_hex(&bytes);
        if checksum != package_ref.sha256 {
            return Err(AppError::validation_code(
                CATALOG_CHECKSUM_MISMATCH,
                format!(
                    "the downloaded package has SHA-256 {checksum}, but the catalog declares {}; it was not installed",
                    package_ref.sha256
                ),
            ));
        }
        let package: Value = serde_json::from_slice(&bytes).map_err(|error| {
            AppError::validation_code(
                CATALOG_INVALID_PACKAGE,
                format!("the downloaded package is not JSON: {error}"),
            )
        })?;
        let declared_id = package.get("id").and_then(Value::as_str);
        let declared_version = package.get("version").and_then(Value::as_str);
        if declared_id != Some(entry.id.as_str())
            || declared_version != Some(entry.version.as_str())
        {
            return Err(AppError::validation_code(
                CATALOG_PACKAGE_MISMATCH,
                format!(
                    "the package declares {} {}, but the catalog entry is {} {}",
                    declared_id.unwrap_or("(no id)"),
                    declared_version.unwrap_or("(no version)"),
                    entry.id,
                    entry.version
                ),
            ));
        }
        Ok(DownloadedTemplate {
            checksum,
            origin: package_url,
            entry,
            package,
        })
    }

    async fn index(&self, source: &str) -> Result<crate::application::CatalogIndex, AppError> {
        let settings = self.ports.settings();
        let bytes = self
            .ports
            .fetcher()
            .fetch(source, settings, MAX_CATALOG_INDEX_BYTES)
            .await
            .map_err(|error| unavailable("could not read the catalog index", &error))?;
        parse_index(&bytes, settings.allow_http)
            .map_err(|message| AppError::validation_code(CATALOG_UNAVAILABLE, message))
    }

    async fn installed_versions(&self) -> Result<HashMap<String, String>, AppError> {
        let mut installed = HashMap::new();
        for version in self.ports.versions().list().await? {
            installed
                .entry(version.package_id)
                .or_insert(version.version);
        }
        Ok(installed)
    }
}

fn package_limit(kind: CatalogEntryKind) -> usize {
    match kind {
        CatalogEntryKind::Theme => MAX_THEME_PACKAGE_BYTES,
        _ => MAX_PACKAGE_BYTES,
    }
}

pub fn is_compatible(entry: &CatalogEntry) -> bool {
    match (
        parse_version(&entry.min_aphel_version),
        parse_version(APHEL_VERSION),
    ) {
        (Some(required), Some(current)) => required <= current,
        _ => false,
    }
}

fn unavailable(context: &str, error: &CatalogFetchError) -> AppError {
    AppError::validation_code(CATALOG_UNAVAILABLE, format!("{context}: {error}"))
}
