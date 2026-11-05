use std::collections::HashMap;

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::configuration::{
    ConfigPackage, ConfigPackageInstall, ConflictDecision, ExportCommand, InstallResult,
    PackageIssue, PackageMetadata, PackagePlan, PackageSource, PlanAction, PlanChange,
    PlanConflict, PlanItem, PlanSummary, TemplateSummary,
};
use aphel_kernel::errors::{AppError, FieldViolation};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfigPackageDocument {
    #[schema(example = "aphel-config")]
    pub format: String,
    #[schema(example = 1)]
    pub format_version: u32,
    #[schema(example = "0.1.0")]
    pub min_aphel_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "aphel.b2b")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "1.0.0")]
    pub version: Option<String>,
    #[schema(example = "B2B sales")]
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    #[schema(example = "pt-BR")]
    pub locale: String,
    #[schema(value_type = Object)]
    pub sections: HashMap<String, Value>,
}

impl From<ConfigPackage> for ConfigPackageDocument {
    fn from(package: ConfigPackage) -> Self {
        Self {
            format: package.format,
            format_version: package.format_version,
            min_aphel_version: package.min_aphel_version,
            id: package.id,
            version: package.version,
            name: package.name,
            description: package.description,
            author: package.author,
            locale: package.locale,
            sections: package.sections.into_iter().collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigSectionsResponse {
    pub sections: Vec<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct ExportPackageRequest {
    pub sections: Option<Vec<String>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    #[schema(example = "pt-BR")]
    pub locale: Option<String>,
}

impl From<ExportPackageRequest> for ExportCommand {
    fn from(request: ExportPackageRequest) -> Self {
        Self {
            sections: request.sections,
            name: request.name,
            description: request.description,
            author: request.author,
            locale: request.locale,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PackageTemplateResponse {
    #[schema(example = "sales")]
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub locale: String,
    pub sections: Vec<String>,
}

impl From<TemplateSummary> for PackageTemplateResponse {
    fn from(template: TemplateSummary) -> Self {
        Self {
            key: template.key,
            name: template.name,
            description: template.description,
            author: template.author,
            locale: template.locale,
            sections: template.sections,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PlanPackageRequest {
    #[schema(value_type = Option<Object>)]
    pub package: Option<Value>,
    #[schema(example = "sales")]
    pub template: Option<String>,
}

pub fn package_source(
    package: Option<Value>,
    template: Option<String>,
) -> Result<PackageSource, AppError> {
    match (package, template) {
        (Some(package), None) => Ok(PackageSource::Package(package)),
        (None, Some(template)) => Ok(PackageSource::Template(template)),
        _ => Err(AppError::invalid_fields(
            "invalid package source",
            vec![FieldViolation::new(
                "package",
                "send either package or template",
            )],
        )),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanActionResponse {
    Create,
    Update,
    SkipConflict,
    Unchanged,
}

impl From<PlanAction> for PlanActionResponse {
    fn from(action: PlanAction) -> Self {
        match action {
            PlanAction::Create => Self::Create,
            PlanAction::Update => Self::Update,
            PlanAction::SkipConflict => Self::SkipConflict,
            PlanAction::Unchanged => Self::Unchanged,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictDecisionBody {
    Apply,
    Skip,
}

impl From<ConflictDecisionBody> for ConflictDecision {
    fn from(decision: ConflictDecisionBody) -> Self {
        match decision {
            ConflictDecisionBody::Apply => Self::Apply,
            ConflictDecisionBody::Skip => Self::Skip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PackageIssueResponse {
    #[schema(example = "sections.fields.0.type")]
    pub path: String,
    #[schema(example = "invalid_field_type")]
    pub code: String,
    pub message: String,
}

impl From<PackageIssue> for PackageIssueResponse {
    fn from(issue: PackageIssue) -> Self {
        Self {
            path: issue.path,
            code: issue.code,
            message: issue.message,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlanChangeResponse {
    #[schema(example = "label")]
    pub property: String,
    #[schema(value_type = Object)]
    pub current: Value,
    #[schema(value_type = Object)]
    pub proposed: Value,
}

impl From<PlanChange> for PlanChangeResponse {
    fn from(change: PlanChange) -> Self {
        Self {
            property: change.property,
            current: change.current,
            proposed: change.proposed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlanConflictResponse {
    #[schema(example = "field_type_mismatch")]
    pub code: String,
    pub message: String,
    pub resolvable: bool,
}

impl From<PlanConflict> for PlanConflictResponse {
    fn from(conflict: PlanConflict) -> Self {
        Self {
            code: conflict.code,
            message: conflict.message,
            resolvable: conflict.resolvable,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlanItemResponse {
    #[schema(example = "fields:field:company.segment")]
    pub id: String,
    #[schema(example = "fields")]
    pub section: String,
    #[schema(example = "field")]
    pub kind: String,
    #[schema(example = "company.segment")]
    pub key: String,
    pub label: String,
    pub action: PlanActionResponse,
    pub changes: Vec<PlanChangeResponse>,
    pub conflict: Option<PlanConflictResponse>,
}

impl From<PlanItem> for PlanItemResponse {
    fn from(item: PlanItem) -> Self {
        Self {
            id: item.id,
            section: item.section,
            kind: item.kind,
            key: item.key,
            label: item.label,
            action: item.action.into(),
            changes: item.changes.into_iter().map(Into::into).collect(),
            conflict: item.conflict.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlanSummaryResponse {
    pub create: u64,
    pub update: u64,
    pub skip_conflict: u64,
    pub unchanged: u64,
}

impl From<PlanSummary> for PlanSummaryResponse {
    fn from(summary: PlanSummary) -> Self {
        Self {
            create: summary.create,
            update: summary.update,
            skip_conflict: summary.skip_conflict,
            unchanged: summary.unchanged,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PackageMetadataResponse {
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub locale: String,
}

impl From<PackageMetadata> for PackageMetadataResponse {
    fn from(metadata: PackageMetadata) -> Self {
        Self {
            name: metadata.name,
            description: metadata.description,
            author: metadata.author,
            locale: metadata.locale,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PackagePlanResponse {
    pub valid: bool,
    pub package: Option<PackageMetadataResponse>,
    pub template_key: Option<String>,
    pub sections: Vec<String>,
    pub skipped_sections: Vec<String>,
    #[schema(example = json!(["deal.mrr"]))]
    pub skipped_fields: Vec<String>,
    pub issues: Vec<PackageIssueResponse>,
    pub items: Vec<PlanItemResponse>,
    pub fingerprint: String,
    pub summary: PlanSummaryResponse,
}

impl From<PackagePlan> for PackagePlanResponse {
    fn from(plan: PackagePlan) -> Self {
        Self {
            valid: plan.valid,
            package: plan.package.map(Into::into),
            template_key: plan.template_key,
            sections: plan.sections,
            skipped_sections: plan.skipped_sections,
            skipped_fields: plan.skipped_fields,
            issues: plan.issues.into_iter().map(Into::into).collect(),
            items: plan.items.into_iter().map(Into::into).collect(),
            fingerprint: plan.fingerprint,
            summary: plan.summary.into(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplyPackageRequest {
    #[schema(value_type = Option<Object>)]
    pub package: Option<Value>,
    #[schema(example = "sales")]
    pub template: Option<String>,
    pub fingerprint: String,
    #[serde(default)]
    pub decisions: HashMap<String, ConflictDecisionBody>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstalledByResponse {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstallSummaryResponse {
    pub create: u64,
    pub update: u64,
    pub skip_conflict: u64,
    pub unchanged: u64,
    pub no_op: bool,
    pub sections: Vec<String>,
}

impl From<&Value> for InstallSummaryResponse {
    fn from(summary: &Value) -> Self {
        let count = |key: &str| summary.get(key).and_then(Value::as_u64).unwrap_or(0);
        Self {
            create: count("create"),
            update: count("update"),
            skip_conflict: count("skip_conflict"),
            unchanged: count("unchanged"),
            no_op: summary
                .get("no_op")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            sections: summary
                .get("sections")
                .and_then(Value::as_array)
                .map(|sections| {
                    sections
                        .iter()
                        .filter_map(|section| section.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigPackageInstallResponse {
    pub id: String,
    pub name: String,
    pub format_version: u32,
    pub template_key: Option<String>,
    pub summary: InstallSummaryResponse,
    pub installed_by: Option<InstalledByResponse>,
    pub installed_at: String,
}

impl From<ConfigPackageInstall> for ConfigPackageInstallResponse {
    fn from(install: ConfigPackageInstall) -> Self {
        Self {
            id: install.id.to_string(),
            name: install.name,
            format_version: install.format_version,
            template_key: install.template_key,
            summary: InstallSummaryResponse::from(&install.summary),
            installed_by: install.installed_by.map(|id| InstalledByResponse {
                id: id.into_inner(),
                name: install.installed_by_name,
            }),
            installed_at: install
                .installed_at
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigPackageInstallDetailResponse {
    #[serde(flatten)]
    pub install: ConfigPackageInstallResponse,
    #[schema(value_type = Object)]
    pub package: Value,
}

impl From<ConfigPackageInstall> for ConfigPackageInstallDetailResponse {
    fn from(mut install: ConfigPackageInstall) -> Self {
        let package = std::mem::take(&mut install.package);
        Self {
            install: install.into(),
            package,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplyPackageResponse {
    pub install: ConfigPackageInstallResponse,
    pub summary: PlanSummaryResponse,
    pub applied: Vec<PlanItemResponse>,
}

impl From<InstallResult> for ApplyPackageResponse {
    fn from(result: InstallResult) -> Self {
        Self {
            install: result.install.into(),
            summary: result.summary.into(),
            applied: result.applied.into_iter().map(Into::into).collect(),
        }
    }
}
