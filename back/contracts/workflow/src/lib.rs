use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, ReferenceKind, contract,
    extension_point, reference,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const CONTRACT: Contract = contract!("workflow", "1.0.0");
pub const STAGED: ExtensionPoint<StagedMapping> = extension_point!("workflow.staged");
pub const STAGE: ReferenceKind = reference!("workflow.stage");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "workflow.staged",
            KeyKind::ExtensionPoint,
            "Entity stage, pipeline and completion field mappings",
        ),
        KeyEntry::new("workflow.stage", KeyKind::Reference, "A workflow stage"),
    ],
};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StagedMapping {
    pub stage_field: String,
    pub pipeline_field: String,
    pub entered_at_field: String,
    pub state_field: Option<String>,
    pub active_state: Option<String>,
    pub completed_states: Vec<String>,
}
impl Default for StagedMapping {
    fn default() -> Self {
        Self {
            stage_field: "stage_id".to_owned(),
            pipeline_field: "pipeline_id".to_owned(),
            entered_at_field: "stage_entered_at".to_owned(),
            state_field: None,
            active_state: None,
            completed_states: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct StageUsage {
    pub total: u64,
    pub active: u64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UsageTarget {
    Stage(Uuid),
    Pipeline(Uuid),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageLocation {
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReference {
    pub id: Uuid,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMove {
    pub from: WorkflowReference,
    pub to: WorkflowReference,
    pub target_pipeline_id: Uuid,
    pub pipelines: Option<(WorkflowReference, WorkflowReference)>,
}
