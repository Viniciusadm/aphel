use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::application::query::{Filter, SortSpec};
use crate::application::views::SavedView;

#[derive(Debug, Clone, PartialEq)]
pub struct CreateViewCommand {
    pub name: String,
    pub visibility: String,
    pub team_id: Option<String>,
    pub view_type: String,
    pub filter: Option<Filter>,
    pub sort: Vec<SortSpec>,
    pub columns: Vec<String>,
    pub settings: Value,
    pub is_default: bool,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpdateViewCommand {
    pub name: Option<String>,
    pub visibility: Option<String>,
    pub team_id: Option<Option<String>>,
    pub view_type: Option<String>,
    pub filter: Option<Option<Filter>>,
    pub sort: Option<Vec<SortSpec>>,
    pub columns: Option<Vec<String>>,
    pub settings: Option<Value>,
    pub is_default: Option<bool>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavedViewEntry {
    pub view: SavedView,
    pub is_user_default: bool,
    pub can_edit: bool,
}
