use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct InstallerStatusResponse {
    pub available: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct InstallRequest {
    #[schema(example = "Acme Realty")]
    pub organization_name: String,
    #[schema(example = "America/Sao_Paulo")]
    pub timezone: String,
    #[schema(example = "BRL")]
    pub currency: String,
    #[schema(example = "pt-BR")]
    pub locale: String,
    #[schema(example = "Ana Souza")]
    pub admin_name: String,
    #[schema(example = "ana@example.com")]
    pub admin_email: String,
    #[schema(min_length = 8, max_length = 128)]
    pub admin_password: String,
    #[schema(example = "https://crm.example.com")]
    pub public_base_url: Option<String>,
}
