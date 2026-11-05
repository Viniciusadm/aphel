use std::collections::HashMap;

use chrono::{DateTime, Utc};

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_sdk::platform::domain::access::{AccessScope, Permission};

pub use aphel_sdk::platform::application::shared::parse_uuid as parse_id;

pub const PIPELINE_NOT_FOUND: &str = "pipeline_not_found";
pub(crate) const PIPELINE_STAGE_NOT_FOUND: &str = "pipeline_stage_not_found";
pub(crate) const LOST_REASON_NOT_FOUND: &str = "lost_reason_not_found";
pub(crate) const STAGE_HAS_RECORDS: &str = "stage_has_records";
pub(crate) const CLOSING_STAGES_REQUIRED: &str = "closing_stages_required";
pub(crate) const DEFAULT_PIPELINE_REQUIRED: &str = "default_pipeline_required";
pub(crate) const PIPELINE_HAS_RECORDS: &str = "pipeline_has_records";
pub(crate) const PIPELINE_LIMIT_REACHED: &str = "pipeline_limit_reached";
pub(crate) const LAST_LOST_REASON: &str = "last_lost_reason";

pub(crate) const PIPELINE_CREATED: &str = "pipeline.created";
pub(crate) const PIPELINE_UPDATED: &str = "pipeline.updated";
pub(crate) const PIPELINE_REORDERED: &str = "pipeline.reordered";
pub(crate) const PIPELINE_DEFAULT_CHANGED: &str = "pipeline.default_changed";
pub(crate) const PIPELINE_ACTIVATED: &str = "pipeline.activated";
pub(crate) const PIPELINE_DEACTIVATED: &str = "pipeline.deactivated";
pub(crate) const PIPELINE_DELETED: &str = "pipeline.deleted";
pub(crate) const PIPELINE_STAGE_CREATED: &str = "pipeline_stage.created";
pub(crate) const PIPELINE_STAGE_UPDATED: &str = "pipeline_stage.updated";
pub(crate) const PIPELINE_STAGE_REORDERED: &str = "pipeline_stage.reordered";
pub(crate) const PIPELINE_STAGE_DEACTIVATED: &str = "pipeline_stage.deactivated";
pub(crate) const PIPELINE_STAGE_DELETED: &str = "pipeline_stage.deleted";
pub(crate) const LOST_REASON_CREATED: &str = "lost_reason.created";
pub(crate) const LOST_REASON_UPDATED: &str = "lost_reason.updated";
pub(crate) const LOST_REASON_REORDERED: &str = "lost_reason.reordered";

pub(crate) const MAX_STAGES_PER_PIPELINE: usize = 50;
pub(crate) const MAX_PIPELINES: usize = 50;
pub(crate) const MAX_LOST_REASONS: usize = 100;
pub(crate) const MAX_PIPELINE_NAME_LENGTH: usize = 140;
pub(crate) const MAX_REQUIRED_FIELDS: usize = 30;
pub(crate) const MAX_PIPELINE_TEAMS: usize = 50;

pub use aphel_sdk::workflow::{LostReason, Pipeline, PipelineStage, StageKind};

pub(crate) fn normalize_name(value: &str) -> Option<String> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > MAX_PIPELINE_NAME_LENGTH {
        return None;
    }
    Some(name.to_owned())
}

pub use aphel_sdk::workflow::valid_probability;

pub(crate) fn keeps_closing_stages(stages: &[PipelineStage]) -> bool {
    let active = |kind: StageKind| {
        stages
            .iter()
            .any(|stage| stage.is_active && stage.kind == kind)
    };
    active(StageKind::Won) && active(StageKind::Lost) && active(StageKind::Open)
}

pub use aphel_sdk::workflow::first_active_of_kind;

pub(crate) fn map_stage_by_kind<'a>(
    target_stages: &'a [PipelineStage],
    source: &PipelineStage,
) -> Option<&'a PipelineStage> {
    first_active_of_kind(target_stages, source.kind)
}

pub(crate) fn normalize_field_keys(keys: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::with_capacity(keys.len());
    for key in keys.iter().map(|key| key.trim()) {
        if !key.is_empty() && !normalized.iter().any(|existing| existing == key) {
            normalized.push(key.to_owned());
        }
    }
    normalized
}

pub(crate) fn open_stage_insert_index(stages: &[PipelineStage]) -> usize {
    stages
        .iter()
        .rposition(|stage| stage.kind == StageKind::Open)
        .map_or(0, |index| index + 1)
}

pub use aphel_sdk::workflow::sees_every_pipeline;

pub use aphel_sdk::workflow::pipeline_visible;

pub(crate) fn visible_pipelines(scope: &AccessScope, workflow: Vec<Pipeline>) -> Vec<Pipeline> {
    workflow
        .into_iter()
        .filter(|pipeline| pipeline_visible(scope, pipeline))
        .collect()
}

pub use aphel_sdk::workflow::pipeline_restriction;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdatePipelineCommand {
    pub(crate) name: Option<String>,
    pub(crate) team_ids: Option<Vec<String>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NewStageInput {
    pub(crate) name: String,
    pub(crate) probability: Option<i64>,
    pub(crate) kind: Option<StageKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreatePipelineCommand {
    pub(crate) name: String,
    pub(crate) team_ids: Vec<String>,
    pub(crate) stages: Vec<NewStageInput>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PipelineRemovalCommand {
    pub(crate) target_pipeline_id: Option<String>,
    pub(crate) stage_mapping: HashMap<String, String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateStageCommand {
    pub(crate) name: String,
    pub(crate) probability: Option<i64>,
    pub(crate) rotting_days: Option<i64>,
    pub(crate) kind: Option<StageKind>,
    pub(crate) entry_required_fields: Vec<String>,
    pub(crate) exit_required_fields: Vec<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateStageCommand {
    pub name: Option<String>,
    pub probability: Option<i64>,
    pub rotting_days: Option<Option<i64>>,
    pub kind: Option<StageKind>,
    pub entry_required_fields: Option<Vec<String>>,
    pub exit_required_fields: Option<Vec<String>>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReorderCommand {
    pub(crate) ids: Vec<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct StageRemovalCommand {
    pub(crate) target_stage_id: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateLostReasonCommand {
    pub(crate) name: String,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateLostReasonCommand {
    pub(crate) name: Option<String>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

pub use aphel_sdk::workflow::{PipelineView, StageView};

pub use aphel_sdk::workflow::StageUsage;

pub use aphel_sdk::workflow::pipeline_ref;

pub use aphel_sdk::workflow::stage_ref;

pub(crate) fn require_records_reader(scope: &AccessScope) -> Result<(), AppError> {
    if scope
        .require(&Permission::from_static("workflow.read"))
        .is_ok()
        || scope.require(&Permission::SETTINGS_MANAGE).is_ok()
    {
        return Ok(());
    }
    Err(AppError::forbidden_code(
        PERMISSION_DENIED,
        "missing one of the permissions records.read, settings.manage",
    ))
}

pub fn pipeline_not_found() -> AppError {
    AppError::not_found_code(PIPELINE_NOT_FOUND, "pipeline was not found")
}

pub(crate) fn stage_not_found() -> AppError {
    AppError::not_found_code(PIPELINE_STAGE_NOT_FOUND, "pipeline stage was not found")
}

pub(crate) fn lost_reason_not_found() -> AppError {
    AppError::not_found_code(LOST_REASON_NOT_FOUND, "lost reason was not found")
}
