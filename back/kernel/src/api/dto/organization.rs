use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::application::organization_profile::UpdateOrganizationCommand;
use crate::domain::organization_profile::Organization;

#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationResponse {
    pub name: String,
    pub logo_file_id: Option<String>,
    #[schema(example = "#4f46e5")]
    pub primary_color: Option<String>,
    #[schema(example = "pt-BR")]
    pub locale: String,
    #[schema(example = "America/Sao_Paulo")]
    pub timezone: String,
    #[schema(example = "BRL")]
    pub currency: String,
}

impl From<Organization> for OrganizationResponse {
    fn from(organization: Organization) -> Self {
        Self {
            name: organization.name.as_str().to_owned(),
            logo_file_id: organization.logo_file_id,
            primary_color: organization.primary_color,
            locale: organization.locale,
            timezone: organization.timezone,
            currency: organization.currency,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationRequest {
    pub name: String,
    pub primary_color: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub currency: String,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable = true)]
    pub logo_file_id: Option<Option<String>>,
}

impl UpdateOrganizationRequest {
    pub fn into_command(self, ip: Option<String>) -> UpdateOrganizationCommand {
        UpdateOrganizationCommand {
            name: self.name,
            primary_color: self.primary_color,
            locale: self.locale,
            timezone: self.timezone,
            currency: self.currency,
            logo_file_id: self.logo_file_id,
            ip,
        }
    }
}
