use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::domain::UserId;

pub(crate) const STAGE_CHECKLIST_INCOMPLETE: &str = "stage_checklist_incomplete";
pub(crate) const CHECKLIST_ITEM_NOT_FOUND: &str = "checklist_item_not_found";
pub(crate) const DEAL_NOT_FOUND: &str = "deal_not_found";

pub(crate) const STAGE_GUIDANCE_UPDATED: &str = "pipeline.stage_guidance_updated";
pub(crate) const DEAL_CHECKLIST_ITEM_CHECKED: &str = "deal.checklist_item_checked";
pub(crate) const DEAL_CHECKLIST_ITEM_UNCHECKED: &str = "deal.checklist_item_unchecked";

pub(crate) const CHECKLIST_PATH_PREFIX: &str = "checklist.";
pub(crate) const MAX_GUIDANCE_LENGTH: usize = 20_000;
pub(crate) const MAX_CHECKLIST_LABEL_LENGTH: usize = 200;
pub(crate) const MAX_CHECKLIST_ITEMS: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuidanceText {
    pub(crate) stage_id: Uuid,
    pub(crate) guidance: String,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChecklistItem {
    pub(crate) id: Uuid,
    pub(crate) stage_id: Uuid,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) is_required: bool,
    pub(crate) position: i32,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ChecklistItem {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "key": self.key,
            "label": self.label,
            "required": self.is_required,
            "position": self.position,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChecklistProgress {
    pub(crate) item_id: Uuid,
    pub(crate) done_by: Option<UserId>,
    pub(crate) done_by_name: Option<String>,
    pub(crate) done_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageGuidance {
    pub(crate) stage_id: Uuid,
    pub(crate) pipeline_id: Uuid,
    pub(crate) stage_name: String,
    pub(crate) guidance: String,
    pub(crate) items: Vec<ChecklistItem>,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

impl StageGuidance {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "guidance": self.guidance,
            "items": self.items.iter().map(ChecklistItem::snapshot).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DealChecklistItem {
    pub(crate) item: ChecklistItem,
    pub(crate) progress: Option<ChecklistProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DealChecklist {
    pub(crate) deal_id: Uuid,
    pub(crate) guidance: StageGuidance,
    pub(crate) items: Vec<DealChecklistItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ChecklistItemInput {
    pub(crate) id: Option<String>,
    pub(crate) key: Option<String>,
    pub(crate) label: String,
    pub(crate) required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SaveStageGuidanceCommand {
    pub(crate) guidance: Option<String>,
    pub(crate) items: Option<Vec<ChecklistItemInput>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ToggleChecklistItemCommand {
    pub(crate) done: bool,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DealLocation {
    pub(crate) pipeline_id: Uuid,
    pub(crate) stage_id: Uuid,
}
