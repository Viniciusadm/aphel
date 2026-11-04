use serde::Serialize;
use utoipa::ToSchema;
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SsoLoginStatus {
    pub provider: String,
    pub label: String,
    #[schema(example = "https://crm.example.com/api/v1/public/google/callback")]
    pub callback_url: String,
    pub configured: bool,
    pub domains: Vec<String>,
    pub require_sso: bool,
}

pub use super::login::login_status;
