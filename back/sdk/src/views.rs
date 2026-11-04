use crate::{AppError, Ctx};
pub use aphel_contract_views::*;
use aphel_kernel::application::query::{Filter, SortSpec};
use aphel_kernel::application::shared::RecordType;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq)]
pub struct SavedViewSnapshot {
    pub id: Uuid,
    pub record_type: RecordType,
    pub name: String,
    pub filter: Option<Filter>,
    pub sort: Vec<SortSpec>,
    pub settings: serde_json::Map<String, serde_json::Value>,
}

#[async_trait::async_trait]
pub trait SavedViewReader: Send + Sync {
    async fn visible(&self, ctx: &Ctx<'_>, id: Uuid)
    -> Result<Option<SavedViewSnapshot>, AppError>;
}

pub async fn visible_view(ctx: &Ctx<'_>, id: Uuid) -> Result<Option<SavedViewSnapshot>, AppError> {
    let Some(reader) = ctx.services().get::<dyn SavedViewReader>() else {
        return Ok(None);
    };
    reader.visible(ctx, id).await
}
