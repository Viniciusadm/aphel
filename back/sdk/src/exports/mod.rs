pub mod encode;
pub use crate::files::{
    GeneratedFile, StoredGeneratedFile, discard_generated_file, read_file_content,
    store_generated_file,
};
pub use aphel_contract_exports::*;
use aphel_kernel::Ctx;
use aphel_kernel::application::query::{ArchivedFilter, Filter, SortSpec};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, PermissionSet, RoleKey};
use aphel_kernel::errors::AppError;
pub use encode::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    pub sort: Vec<SortSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    pub archived: ArchivedFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestExportCommand {
    pub record_type: String,
    pub format: ExportFormat,
    pub query: ExportQuery,
    pub columns: Vec<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormatDto {
    Csv,
    Xlsx,
}

impl From<ExportFormatDto> for ExportFormat {
    fn from(format: ExportFormatDto) -> Self {
        match format {
            ExportFormatDto::Csv => Self::Csv,
            ExportFormatDto::Xlsx => Self::Xlsx,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportJobResponse {
    pub id: String,
    #[schema(example = "people.person")]
    pub record_type: String,
    #[schema(example = "xlsx")]
    pub format: String,
    #[schema(example = "queued")]
    pub status: String,
    pub columns: Vec<String>,
    pub file_id: Option<String>,
    pub row_count: Option<u64>,
    pub failure_reason: Option<String>,
    pub requested_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
}

pub fn system_scope(user_id: UserId) -> Result<AccessScope, AppError> {
    Ok(AccessScope::new(
        user_id,
        RoleKey::new("system")?,
        Vec::new(),
        PermissionSet::default(),
    ))
}

#[async_trait::async_trait]
pub trait ExportJobs: Send + Sync {
    async fn request(
        &self,
        ctx: &Ctx<'_>,
        command: RequestExportCommand,
    ) -> Result<ExportJobResponse, AppError>;
}

pub async fn request_export(
    ctx: &Ctx<'_>,
    command: RequestExportCommand,
) -> Result<ExportJobResponse, AppError> {
    let service = ctx.services().get::<dyn ExportJobs>().ok_or_else(|| {
        AppError::unavailable_code("exports_unavailable", "record exports are not enabled")
    })?;
    service.request(ctx, command).await
}
