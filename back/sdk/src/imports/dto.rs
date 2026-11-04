use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKeyDto {
    None,
    Email,
    Phone,
    Domain,
    Name,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateStrategyDto {
    Skip,
    Update,
    Create,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImportOptionsDto {
    pub duplicate_key: DuplicateKeyDto,
    pub duplicate_strategy: DuplicateStrategyDto,
    pub create_missing_references: bool,
    #[serde(default)]
    pub distribution_rule_id: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, ToSchema)]
pub struct ImportTotalsResponse {
    pub total: u64,
    pub pending: u64,
    pub imported: u64,
    pub updated: u64,
    pub skipped: u64,
    pub failed: u64,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, ToSchema)]
pub struct ImportTargetResponse {
    pub can_create_reference: bool,
    #[schema(example = "email")]
    pub key: String,
    #[schema(example = "Email")]
    pub label: String,
    #[schema(example = "email")]
    pub field_type: String,
    pub required: bool,
    pub is_custom: bool,
    #[schema(example = "organizations.organization")]
    pub relation_target: Option<String>,
    pub multiple: bool,
    #[schema(example = "name")]
    pub satisfied_by: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, ToSchema)]
pub struct ImportPreviewRow {
    pub row_index: u32,
    pub cells: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, ToSchema)]
pub struct ImportJobResponse {
    pub id: String,
    #[schema(example = "people.person")]
    pub record_type: String,
    #[schema(example = "draft")]
    pub status: String,
    pub file_id: Option<String>,
    pub privacy_redacted_at: Option<String>,
    pub file_name: String,
    #[schema(example = "utf-8")]
    pub encoding: String,
    #[schema(example = ";")]
    pub delimiter: String,
    pub headers: Vec<String>,
    pub preview: Vec<ImportPreviewRow>,
    pub row_count: u32,
    pub mapping: Vec<Option<String>>,
    pub options: ImportOptionsDto,
    pub totals: ImportTotalsResponse,
    pub error_file_id: Option<String>,
    pub retry_of: Option<String>,
    pub failure_reason: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub targets: Vec<ImportTargetResponse>,
    pub duplicate_keys: Vec<DuplicateKeyDto>,
}
