use crate::{ConflictDecision, PackageIssue, PackageMetadata, PlanAction, PlanItem};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub const INVALID_PACKAGE: &str = "invalid_package";

#[derive(Debug, Clone, PartialEq)]
pub enum PackageSource {
    Package(Value),
    Template(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportCommand {
    pub sections: Option<Vec<String>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplyCommand {
    pub source: PackageSource,
    pub decisions: HashMap<String, ConflictDecision>,
    pub fingerprint: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct PlanSummary {
    pub create: u64,
    pub update: u64,
    pub skip_conflict: u64,
    pub unchanged: u64,
}

impl PlanSummary {
    pub fn of(items: &[PlanItem]) -> Self {
        let mut summary = Self::default();
        for item in items {
            match item.action {
                PlanAction::Create => summary.create += 1,
                PlanAction::Update => summary.update += 1,
                PlanAction::SkipConflict => summary.skip_conflict += 1,
                PlanAction::Unchanged => summary.unchanged += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackagePlan {
    pub package: Option<PackageMetadata>,
    pub template_key: Option<String>,
    pub sections: Vec<String>,
    pub skipped_sections: Vec<String>,
    pub skipped_fields: Vec<String>,
    pub valid: bool,
    pub issues: Vec<PackageIssue>,
    pub items: Vec<PlanItem>,
    pub fingerprint: String,
    pub summary: PlanSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateSummary {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub locale: String,
    pub sections: Vec<String>,
}
