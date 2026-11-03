use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, ReferenceKind, contract,
    extension_point, reference,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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
    pub rotting: bool,
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
            rotting: false,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageKind {
    Open,
    Won,
    Lost,
}

impl StageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Won => "won",
            Self::Lost => "lost",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "won" => Some(Self::Won),
            "lost" => Some(Self::Lost),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub id: Uuid,
    pub name: String,
    pub is_default: bool,
    pub position: i32,
    pub is_active: bool,
    pub team_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Pipeline {
    pub fn is_restricted(&self) -> bool {
        !self.team_ids.is_empty()
    }

    pub fn visible_to_teams<'a>(&self, mut team_ids: impl Iterator<Item = &'a str>) -> bool {
        !self.is_restricted()
            || team_ids.any(|team| {
                self.team_ids
                    .iter()
                    .any(|allowed| allowed.to_string() == team)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStage {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub name: String,
    pub position: i32,
    pub probability: u8,
    pub rotting_days: Option<u16>,
    pub kind: StageKind,
    pub is_active: bool,
    pub entry_required_fields: Vec<String>,
    pub exit_required_fields: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LostReason {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn valid_probability(value: i64) -> Option<u8> {
    u8::try_from(value)
        .ok()
        .filter(|probability| *probability <= 100)
}
pub fn first_active_of_kind(stages: &[PipelineStage], kind: StageKind) -> Option<&PipelineStage> {
    stages
        .iter()
        .filter(|stage| stage.is_active && stage.kind == kind)
        .min_by_key(|stage| (stage.position, stage.id))
}
pub fn pipeline_ref(pipeline: &Pipeline) -> Value {
    json!({ "id": pipeline.id.to_string(), "name": pipeline.name })
}
pub fn stage_ref(stage: &PipelineStage) -> Value {
    json!({ "id": stage.id.to_string(), "name": stage.name })
}

pub const DAYS_IN_STAGE_FIELD: &str = "days_in_stage";
pub const IS_ROTTING_FIELD: &str = "is_rotting";
pub const ROTTING_AT_FIELD: &str = "rotting_at";
pub const STAGE_ENTERED_COLUMN: &str = "stage_entered_at";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RottingState {
    pub days_in_stage: i64,
    pub rotting_days: Option<u16>,
    pub rotting_at: Option<DateTime<Utc>>,
    pub is_rotting: bool,
}

impl RottingState {
    pub fn of(
        is_open: bool,
        stage_entered_at: DateTime<Utc>,
        rotting_days: Option<u16>,
        now: DateTime<Utc>,
    ) -> Self {
        let rotting_at = rotting_days
            .filter(|_| is_open)
            .map(|days| stage_entered_at + Duration::days(i64::from(days)));
        Self {
            days_in_stage: (now - stage_entered_at).num_days(),
            rotting_days,
            rotting_at,
            is_rotting: rotting_at.is_some_and(|at| at <= now),
        }
    }
}
