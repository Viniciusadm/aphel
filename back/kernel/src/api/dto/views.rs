use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::api::dto::query::{FilterNodeSchema, SortSpecSchema};
use crate::application::query::{Filter, SortSpec};
use crate::application::views::{CreateViewCommand, SavedViewEntry, UpdateViewCommand};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ViewVisibility)]
pub enum ViewVisibilitySchema {
    Private,
    Team,
    Organization,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ViewType)]
pub enum ViewTypeSchema {
    Table,
    Kanban,
    Calendar,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateViewRequest {
    #[schema(example = "My hot leads")]
    pub name: String,
    #[serde(default = "private_visibility")]
    #[schema(value_type = Option<ViewVisibilitySchema>)]
    pub visibility: String,
    pub team_id: Option<String>,
    #[serde(default = "table_view")]
    #[schema(value_type = Option<ViewTypeSchema>)]
    pub view_type: String,
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub filter: Option<Filter>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<SortSpecSchema>>)]
    pub sort: Vec<SortSpec>,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub settings: Value,
    #[serde(default)]
    pub is_default: bool,
}

fn private_visibility() -> String {
    "private".to_owned()
}

fn table_view() -> String {
    "table".to_owned()
}

impl CreateViewRequest {
    pub fn into_command(self, ip: Option<String>) -> CreateViewCommand {
        CreateViewCommand {
            name: self.name,
            visibility: self.visibility,
            team_id: self.team_id,
            view_type: self.view_type,
            filter: self.filter,
            sort: self.sort,
            columns: self.columns,
            settings: self.settings,
            is_default: self.is_default,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateViewRequest {
    pub name: Option<String>,
    #[schema(value_type = Option<ViewVisibilitySchema>)]
    pub visibility: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub team_id: Option<Option<String>>,
    #[schema(value_type = Option<ViewTypeSchema>)]
    pub view_type: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub filter: Option<Option<Filter>>,
    #[schema(value_type = Option<Vec<SortSpecSchema>>)]
    pub sort: Option<Vec<SortSpec>>,
    pub columns: Option<Vec<String>>,
    #[schema(value_type = Option<Object>)]
    pub settings: Option<Value>,
    pub is_default: Option<bool>,
}

impl UpdateViewRequest {
    pub fn into_command(
        self,
        expected_updated_at: Option<chrono::DateTime<Utc>>,
        ip: Option<String>,
    ) -> UpdateViewCommand {
        UpdateViewCommand {
            name: self.name,
            visibility: self.visibility,
            team_id: self.team_id,
            view_type: self.view_type,
            filter: self.filter,
            sort: self.sort,
            columns: self.columns,
            settings: self.settings,
            is_default: self.is_default,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SavedViewResponse {
    pub id: String,
    pub record_type: String,
    pub name: String,
    pub owner_id: Option<String>,
    #[schema(value_type = ViewVisibilitySchema)]
    pub visibility: String,
    pub team_id: Option<String>,
    #[schema(value_type = ViewTypeSchema)]
    pub view_type: String,
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub filter: Option<Filter>,
    #[schema(value_type = Vec<SortSpecSchema>)]
    pub sort: Vec<SortSpec>,
    pub columns: Vec<String>,
    #[schema(value_type = Object)]
    pub settings: Map<String, Value>,
    pub is_default: bool,
    pub is_user_default: bool,
    pub can_edit: bool,
    pub position: i32,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
}

fn timestamp(value: chrono::DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl From<SavedViewEntry> for SavedViewResponse {
    fn from(entry: SavedViewEntry) -> Self {
        let view = entry.view;
        Self {
            id: view.id.to_string(),
            record_type: view.record_type.as_str().to_owned(),
            name: view.name,
            owner_id: view.owner_id.map(|id| id.into_inner()),
            visibility: view.visibility.as_str().to_owned(),
            team_id: view.team_id.map(|id| id.into_inner()),
            view_type: view.view_type.as_str().to_owned(),
            filter: view.filter,
            sort: view.sort,
            columns: view.columns,
            settings: view.settings,
            is_default: view.is_default,
            is_user_default: entry.is_user_default,
            can_edit: entry.can_edit,
            position: view.position,
            created_at: timestamp(view.created_at),
            updated_at: timestamp(view.updated_at),
        }
    }
}
