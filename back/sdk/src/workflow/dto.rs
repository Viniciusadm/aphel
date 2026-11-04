use super::{Pipeline, PipelineStage, StageKind, StageView};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
fn instant(value: chrono::DateTime<chrono::Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = StageKind)]
pub enum StageKindSchema {
    Open,
    Won,
    Lost,
}

impl From<StageKind> for StageKindSchema {
    fn from(kind: StageKind) -> Self {
        match kind {
            StageKind::Open => Self::Open,
            StageKind::Won => Self::Won,
            StageKind::Lost => Self::Lost,
        }
    }
}

impl From<StageKindSchema> for StageKind {
    fn from(kind: StageKindSchema) -> Self {
        match kind {
            StageKindSchema::Open => Self::Open,
            StageKindSchema::Won => Self::Won,
            StageKindSchema::Lost => Self::Lost,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PipelineStageResponse {
    pub id: String,
    pub pipeline_id: String,
    #[schema(example = "Proposal")]
    pub name: String,
    pub position: i32,
    #[schema(example = 50)]
    pub probability: u8,
    #[schema(example = 7)]
    pub rotting_days: Option<u16>,
    pub kind: StageKindSchema,
    pub is_active: bool,
    #[schema(example = json!(["owner_id", "due_at"]))]
    pub entry_required_fields: Vec<String>,
    pub exit_required_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_count: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PipelineStage> for PipelineStageResponse {
    fn from(stage: PipelineStage) -> Self {
        Self {
            id: stage.id.to_string(),
            pipeline_id: stage.pipeline_id.to_string(),
            name: stage.name,
            position: stage.position,
            probability: stage.probability,
            rotting_days: stage.rotting_days,
            kind: stage.kind.into(),
            is_active: stage.is_active,
            entry_required_fields: stage.entry_required_fields,
            exit_required_fields: stage.exit_required_fields,
            record_count: None,
            created_at: instant(stage.created_at),
            updated_at: instant(stage.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PipelineSummaryResponse {
    pub id: String,
    #[schema(example = "Sales pipeline")]
    pub name: String,
    pub is_default: bool,
    pub is_active: bool,
}

impl From<Pipeline> for PipelineSummaryResponse {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            is_default: pipeline.is_default,
            is_active: pipeline.is_active,
        }
    }
}

impl From<StageView> for PipelineStageResponse {
    fn from(view: StageView) -> Self {
        Self {
            record_count: Some(view.record_count),
            ..view.stage.into()
        }
    }
}
