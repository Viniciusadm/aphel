use crate::configuration::{PackageIssue, parse_section};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn stage_item_key(pipeline_key: &str, stage_key: &str) -> String {
    format!("{pipeline_key}/{stage_key}")
}

pub const PIPELINES_SECTION: &str = "workflow";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackagePipeline {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub default: bool,
    pub stages: Vec<PackageStage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageStage {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub probability: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotting_days: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry_required_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exit_required_fields: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn parse_pipelines(content: &Value) -> Result<Vec<PackagePipeline>, PackageIssue> {
    parse_section(content)
}
