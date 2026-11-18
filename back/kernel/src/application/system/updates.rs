use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemRelease {
    pub version: String,
    pub url: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemUpdateOperation {
    pub id: String,
    pub version: String,
    pub phase: String,
    pub actor: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub backup_id: Option<String>,
    pub message_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemUpdateStatus {
    pub enabled: bool,
    pub managed_by_console: bool,
    pub current_version: String,
    pub checked_at: Option<String>,
    pub latest: Option<SystemRelease>,
    pub operation: Option<SystemUpdateOperation>,
    pub history: Vec<SystemUpdateOperation>,
}

impl Default for SystemUpdateStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            managed_by_console: false,
            current_version: super::APHEL_VERSION.to_owned(),
            checked_at: None,
            latest: None,
            operation: None,
            history: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplySystemUpdate {
    pub operation_id: String,
    pub confirm: bool,
}
