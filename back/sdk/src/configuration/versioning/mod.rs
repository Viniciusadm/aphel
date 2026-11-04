pub mod fragments;
use crate::configuration::{PackageIssue, PlanAction, PlanChange, PlanItem, PlanSummary};
pub use aphel_contract_configuration::versioning::*;
use aphel_kernel::domain::UserId;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledVersion {
    pub id: Uuid,
    pub package_id: String,
    pub version: String,
    pub name: String,
    pub install_id: Uuid,
    pub origin: Option<String>,
    pub package_sha256: String,
    pub applied_package: Value,
    pub installed_by: Option<UserId>,
    pub installed_by_name: Option<String>,
    pub installed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageStatus {
    Added,
    Changed,
    Removed,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionRelation {
    Install,
    Upgrade,
    Reinstall,
    Downgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    KeepLocal,
    AcceptPackage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VersionItemComparison {
    pub id: String,
    pub section: String,
    pub kind: String,
    pub key: String,
    pub label: String,
    pub package_status: PackageStatus,
    pub package_changes: Vec<PlanChange>,
    pub locally_modified: bool,
    pub conflict: bool,
    pub plan: Option<PlanItem>,
}

impl VersionItemComparison {
    pub fn plan_conflict(&self) -> bool {
        self.plan.as_ref().is_some_and(|plan| {
            plan.action == PlanAction::SkipConflict
                && plan
                    .conflict
                    .as_ref()
                    .is_some_and(|conflict| conflict.resolvable)
        })
    }

    pub fn needs_resolution(&self) -> bool {
        self.conflict || self.plan_conflict()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ComparisonSummary {
    pub added: u64,
    pub changed: u64,
    pub removed: u64,
    pub unchanged: u64,
    pub locally_modified: u64,
    pub conflicts: u64,
}

impl ComparisonSummary {
    pub fn of(items: &[VersionItemComparison]) -> Self {
        let mut summary = Self::default();
        for item in items {
            match item.package_status {
                PackageStatus::Added => summary.added += 1,
                PackageStatus::Changed => summary.changed += 1,
                PackageStatus::Removed => summary.removed += 1,
                PackageStatus::Unchanged => summary.unchanged += 1,
            }
            if item.locally_modified {
                summary.locally_modified += 1;
            }
            if item.conflict {
                summary.conflicts += 1;
            }
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionComparison {
    pub package: Option<PackageIdentity>,
    pub installed: Option<InstalledVersion>,
    pub relation: Option<VersionRelation>,
    pub valid: bool,
    pub issues: Vec<PackageIssue>,
    pub sections: Vec<String>,
    pub items: Vec<VersionItemComparison>,
    pub summary: ComparisonSummary,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplyVersionCommand {
    pub package: Value,
    pub origin: Option<String>,
    pub checksum: Option<String>,
    pub resolutions: HashMap<String, Resolution>,
    pub fingerprint: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionApplyResult {
    pub version: InstalledVersion,
    pub previous_version: Option<String>,
    pub kept_local: Vec<String>,
    pub summary: PlanSummary,
}

#[async_trait::async_trait]
pub trait ConfigurationVersions: Send + Sync {
    async fn compare(
        &self,
        ctx: &crate::Ctx<'_>,
        package: &Value,
    ) -> Result<VersionComparison, crate::AppError>;
    async fn apply(
        &self,
        ctx: &crate::Ctx<'_>,
        command: ApplyVersionCommand,
    ) -> Result<VersionApplyResult, crate::AppError>;
}
pub fn configuration_versions(
    ctx: &crate::Ctx<'_>,
) -> Result<std::sync::Arc<dyn ConfigurationVersions>, crate::AppError> {
    ctx.services()
        .get::<dyn ConfigurationVersions>()
        .ok_or_else(|| {
            crate::AppError::unavailable_code(
                "configuration_versions_unavailable",
                "configuration versioning is not enabled",
            )
        })
}
