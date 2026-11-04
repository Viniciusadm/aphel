use serde::Serialize;
use utoipa::ToSchema;

use crate::application::{
    CatalogEntryKind, CatalogEntryView, CatalogListing, CatalogModuleRef, CatalogPackageRef,
    RejectedEntry,
};

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatalogEntryKindResponse {
    Template,
    Theme,
    Module,
}

impl From<CatalogEntryKind> for CatalogEntryKindResponse {
    fn from(kind: CatalogEntryKind) -> Self {
        match kind {
            CatalogEntryKind::Template => Self::Template,
            CatalogEntryKind::Theme => Self::Theme,
            CatalogEntryKind::Module => Self::Module,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogPackageResponse {
    pub url: String,
    pub sha256: String,
    pub size: Option<u64>,
}

impl From<CatalogPackageRef> for CatalogPackageResponse {
    fn from(package: CatalogPackageRef) -> Self {
        Self {
            url: package.url,
            sha256: package.sha256,
            size: package.size,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogModuleResponse {
    #[schema(example = "forms")]
    pub key: String,
    pub repository: Option<String>,
    pub install_notes: Option<String>,
}

impl From<CatalogModuleRef> for CatalogModuleResponse {
    fn from(module: CatalogModuleRef) -> Self {
        Self {
            key: module.key,
            repository: module.repository,
            install_notes: module.install_notes,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogEntryResponse {
    #[schema(example = "aphel.b2b")]
    pub id: String,
    pub kind: CatalogEntryKindResponse,
    pub name: String,
    pub description: Option<String>,
    pub author: String,
    #[schema(example = "1.0.0")]
    pub version: String,
    #[schema(example = "0.1.0")]
    pub min_aphel_version: String,
    #[schema(example = "AGPL-3.0-or-later")]
    pub license: String,
    pub homepage: Option<String>,
    pub documentation_url: Option<String>,
    pub tags: Vec<String>,
    pub locale: Option<String>,
    pub package: Option<CatalogPackageResponse>,
    pub module: Option<CatalogModuleResponse>,
    pub compatible: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
}

impl From<CatalogEntryView> for CatalogEntryResponse {
    fn from(view: CatalogEntryView) -> Self {
        let entry = view.entry;
        Self {
            id: entry.id,
            kind: entry.kind.into(),
            name: entry.name,
            description: entry.description,
            author: entry.author,
            version: entry.version,
            min_aphel_version: entry.min_aphel_version,
            license: entry.license,
            homepage: entry.homepage,
            documentation_url: entry.documentation_url,
            tags: entry.tags,
            locale: entry.locale,
            package: entry.package.map(Into::into),
            module: entry.module.map(Into::into),
            compatible: view.compatible,
            installed_version: view.installed_version,
            update_available: view.update_available,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RejectedCatalogEntryResponse {
    pub position: usize,
    pub id: Option<String>,
    pub reason: String,
}

impl From<RejectedEntry> for RejectedCatalogEntryResponse {
    fn from(rejected: RejectedEntry) -> Self {
        Self {
            position: rejected.position,
            id: rejected.id,
            reason: rejected.reason,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogResponse {
    pub enabled: bool,
    pub source: Option<String>,
    #[schema(example = "0.1.0")]
    pub aphel_version: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub updated_at: Option<String>,
    pub entries: Vec<CatalogEntryResponse>,
    pub rejected: Vec<RejectedCatalogEntryResponse>,
}

impl From<CatalogListing> for CatalogResponse {
    fn from(listing: CatalogListing) -> Self {
        Self {
            enabled: listing.enabled,
            source: listing.source,
            aphel_version: listing.aphel_version,
            name: listing.name,
            description: listing.description,
            homepage: listing.homepage,
            updated_at: listing.updated_at,
            entries: listing.entries.into_iter().map(Into::into).collect(),
            rejected: listing.rejected.into_iter().map(Into::into).collect(),
        }
    }
}
