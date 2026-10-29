use std::collections::HashMap;

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::api::dto::config_packages::{
    InstalledByResponse, PackageIssueResponse, PlanChangeResponse, PlanItemResponse,
    PlanSummaryResponse,
};
use crate::application::config_packages::versioning::{
    ComparisonSummary, InstalledVersion, PackageIdentity, PackageStatus, Resolution,
    VersionApplyResult, VersionComparison, VersionItemComparison, VersionRelation,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct PackageIdentityResponse {
    #[schema(example = "aphel.b2b")]
    pub id: String,
    #[schema(example = "1.1.0")]
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    #[schema(example = "pt-BR")]
    pub locale: String,
    #[schema(example = "0.1.0")]
    pub min_aphel_version: String,
}

impl From<PackageIdentity> for PackageIdentityResponse {
    fn from(identity: PackageIdentity) -> Self {
        Self {
            id: identity.id,
            version: identity.version,
            name: identity.name,
            description: identity.description,
            author: identity.author,
            locale: identity.locale,
            min_aphel_version: identity.min_aphel_version,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstalledVersionResponse {
    pub id: String,
    #[schema(example = "aphel.b2b")]
    pub package_id: String,
    #[schema(example = "1.0.0")]
    pub version: String,
    pub name: String,
    pub install_id: String,
    pub origin: Option<String>,
    pub package_sha256: String,
    pub installed_by: Option<InstalledByResponse>,
    pub installed_at: String,
}

impl From<InstalledVersion> for InstalledVersionResponse {
    fn from(version: InstalledVersion) -> Self {
        Self {
            id: version.id.to_string(),
            package_id: version.package_id,
            version: version.version,
            name: version.name,
            install_id: version.install_id.to_string(),
            origin: version.origin,
            package_sha256: version.package_sha256,
            installed_by: version.installed_by.map(|id| InstalledByResponse {
                id: id.into_inner(),
                name: version.installed_by_name,
            }),
            installed_at: version
                .installed_at
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PackageStatusResponse {
    Added,
    Changed,
    Removed,
    Unchanged,
}

impl From<PackageStatus> for PackageStatusResponse {
    fn from(status: PackageStatus) -> Self {
        match status {
            PackageStatus::Added => Self::Added,
            PackageStatus::Changed => Self::Changed,
            PackageStatus::Removed => Self::Removed,
            PackageStatus::Unchanged => Self::Unchanged,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VersionRelationResponse {
    Install,
    Upgrade,
    Reinstall,
    Downgrade,
}

impl From<VersionRelation> for VersionRelationResponse {
    fn from(relation: VersionRelation) -> Self {
        match relation {
            VersionRelation::Install => Self::Install,
            VersionRelation::Upgrade => Self::Upgrade,
            VersionRelation::Reinstall => Self::Reinstall,
            VersionRelation::Downgrade => Self::Downgrade,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionBody {
    KeepLocal,
    AcceptPackage,
}

impl From<ResolutionBody> for Resolution {
    fn from(resolution: ResolutionBody) -> Self {
        match resolution {
            ResolutionBody::KeepLocal => Self::KeepLocal,
            ResolutionBody::AcceptPackage => Self::AcceptPackage,
        }
    }
}

pub fn resolutions(body: HashMap<String, ResolutionBody>) -> HashMap<String, Resolution> {
    body.into_iter()
        .map(|(id, resolution)| (id, resolution.into()))
        .collect()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VersionItemResponse {
    #[schema(example = "fields:field:company.segment")]
    pub id: String,
    pub section: String,
    pub kind: String,
    pub key: String,
    pub label: String,
    pub package_status: PackageStatusResponse,
    pub package_changes: Vec<PlanChangeResponse>,
    pub locally_modified: bool,
    pub conflict: bool,
    pub needs_resolution: bool,
    pub plan: Option<PlanItemResponse>,
}

impl From<VersionItemComparison> for VersionItemResponse {
    fn from(item: VersionItemComparison) -> Self {
        let needs_resolution = item.needs_resolution();
        Self {
            id: item.id,
            section: item.section,
            kind: item.kind,
            key: item.key,
            label: item.label,
            package_status: item.package_status.into(),
            package_changes: item.package_changes.into_iter().map(Into::into).collect(),
            locally_modified: item.locally_modified,
            conflict: item.conflict,
            needs_resolution,
            plan: item.plan.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ComparisonSummaryResponse {
    pub added: u64,
    pub changed: u64,
    pub removed: u64,
    pub unchanged: u64,
    pub locally_modified: u64,
    pub conflicts: u64,
}

impl From<ComparisonSummary> for ComparisonSummaryResponse {
    fn from(summary: ComparisonSummary) -> Self {
        Self {
            added: summary.added,
            changed: summary.changed,
            removed: summary.removed,
            unchanged: summary.unchanged,
            locally_modified: summary.locally_modified,
            conflicts: summary.conflicts,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VersionComparisonResponse {
    pub valid: bool,
    pub package: Option<PackageIdentityResponse>,
    pub installed: Option<InstalledVersionResponse>,
    pub relation: Option<VersionRelationResponse>,
    pub issues: Vec<PackageIssueResponse>,
    pub sections: Vec<String>,
    pub items: Vec<VersionItemResponse>,
    pub summary: ComparisonSummaryResponse,
    pub fingerprint: String,
}

impl From<VersionComparison> for VersionComparisonResponse {
    fn from(comparison: VersionComparison) -> Self {
        Self {
            valid: comparison.valid,
            package: comparison.package.map(Into::into),
            installed: comparison.installed.map(Into::into),
            relation: comparison.relation.map(Into::into),
            issues: comparison.issues.into_iter().map(Into::into).collect(),
            sections: comparison.sections,
            items: comparison.items.into_iter().map(Into::into).collect(),
            summary: comparison.summary.into(),
            fingerprint: comparison.fingerprint,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompareVersionRequest {
    #[schema(value_type = Object)]
    pub package: Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplyVersionRequest {
    #[schema(value_type = Object)]
    pub package: Value,
    pub fingerprint: String,
    #[serde(default)]
    pub resolutions: HashMap<String, ResolutionBody>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplyCatalogVersionRequest {
    pub fingerprint: String,
    #[serde(default)]
    pub resolutions: HashMap<String, ResolutionBody>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VersionApplyResponse {
    pub version: InstalledVersionResponse,
    pub previous_version: Option<String>,
    pub kept_local: Vec<String>,
    pub summary: PlanSummaryResponse,
}

impl From<VersionApplyResult> for VersionApplyResponse {
    fn from(result: VersionApplyResult) -> Self {
        Self {
            version: result.version.into(),
            previous_version: result.previous_version,
            kept_local: result.kept_local,
            summary: result.summary.into(),
        }
    }
}
