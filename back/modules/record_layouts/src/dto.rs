use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::{FormLayout, FormLayoutEntry, SaveLayoutCommand};
use aphel_kernel::api::dto::query::FilterNodeSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = LayoutContext)]
pub enum LayoutContextSchema {
    Create,
    Edit,
    View,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = FormLayout)]
pub struct FormLayoutSchema {
    pub tabs: Vec<LayoutTabSchema>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = LayoutTab)]
pub struct LayoutTabSchema {
    #[schema(example = "main")]
    pub id: String,
    #[serde(default)]
    #[schema(example = "Main")]
    pub label: String,
    pub sections: Vec<LayoutSectionSchema>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = LayoutSection)]
pub struct LayoutSectionSchema {
    #[schema(example = "contact-data")]
    pub id: String,
    #[serde(default)]
    #[schema(example = "Contact details")]
    pub label: String,
    pub columns: Vec<LayoutColumnSchema>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = LayoutColumn)]
pub struct LayoutColumnSchema {
    pub fields: Vec<LayoutFieldSchema>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = LayoutField)]
pub struct LayoutFieldSchema {
    #[schema(example = "email")]
    pub key: String,
    pub visible_when: Option<FilterNodeSchema>,
    pub required_when: Option<FilterNodeSchema>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveLayoutRequest {
    #[schema(value_type = FormLayoutSchema)]
    pub layout: FormLayout,
}

impl SaveLayoutRequest {
    pub fn into_command(self, ip: Option<String>) -> SaveLayoutCommand {
        SaveLayoutCommand {
            layout: self.layout,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormLayoutResponse {
    pub record_type: String,
    #[schema(value_type = LayoutContextSchema)]
    pub context: String,
    #[schema(value_type = Option<FormLayoutSchema>)]
    pub layout: Option<FormLayout>,
    pub is_custom: bool,
    pub updated_by: Option<String>,
    pub updated_at: Option<String>,
}

impl From<FormLayoutEntry> for FormLayoutResponse {
    fn from(entry: FormLayoutEntry) -> Self {
        Self {
            record_type: entry.record_type.as_str().to_owned(),
            context: entry.context.as_str().to_owned(),
            is_custom: entry.layout.is_some(),
            layout: entry.layout,
            updated_by: entry.updated_by.map(|id| id.into_inner()),
            updated_at: entry
                .updated_at
                .map(|instant| instant.to_rfc3339_opts(SecondsFormat::Secs, true)),
        }
    }
}
