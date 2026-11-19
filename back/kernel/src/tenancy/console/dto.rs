use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::api::dto::common::timestamp;
use crate::application::installer::modules::InstallerProfile;
use crate::tenancy::TenantSummary;
use crate::tenancy::console::operators::Operator;

#[derive(Debug, Serialize, ToSchema)]
pub struct OperatorResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    #[schema(example = "active")]
    pub status: String,
    #[schema(format = DateTime)]
    pub last_login_at: Option<String>,
    #[schema(format = DateTime)]
    pub created_at: String,
}

impl From<Operator> for OperatorResponse {
    fn from(operator: Operator) -> Self {
        Self {
            id: operator.id.to_string(),
            email: operator.email,
            display_name: operator.display_name,
            status: operator.status,
            last_login_at: operator.last_login_at.map(timestamp),
            created_at: timestamp(operator.created_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    #[schema(min_length = 8, max_length = 128)]
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteOperatorRequest {
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitationResponse {
    pub operator: OperatorResponse,
    pub invite_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitationDetailsResponse {
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptInvitationRequest {
    #[schema(min_length = 8, max_length = 128)]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantHostResponse {
    pub host: String,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[schema(example = "active")]
    pub status: String,
    pub database_name: String,
    pub database_bytes: Option<u64>,
    #[schema(format = DateTime)]
    pub last_write_at: Option<String>,
    #[schema(format = DateTime)]
    pub migrated_at: Option<String>,
    pub last_error: Option<String>,
    #[schema(format = DateTime)]
    pub created_at: String,
    pub hosts: Vec<TenantHostResponse>,
    pub plugin_management: bool,
}

impl From<TenantSummary> for TenantResponse {
    fn from(summary: TenantSummary) -> Self {
        Self {
            id: summary.tenant.id.to_string(),
            slug: summary.tenant.slug,
            name: summary.tenant.name,
            status: summary.tenant.status.as_str().to_owned(),
            database_name: summary.tenant.database_name,
            database_bytes: summary.database_bytes,
            last_write_at: summary.last_write_at.map(timestamp),
            migrated_at: summary.migrated_at.map(timestamp),
            last_error: summary.last_error,
            created_at: timestamp(summary.created_at),
            plugin_management: summary.tenant.plugin_management,
            hosts: summary
                .hosts
                .into_iter()
                .map(|host| TenantHostResponse {
                    host: host.host,
                    is_primary: host.is_primary,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TenantFeaturesRequest {
    pub plugin_management: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    #[schema(example = "acme")]
    pub slug: String,
    pub name: String,
    #[schema(example = "acme.example.com")]
    pub host: String,
    pub admin_name: String,
    pub admin_email: String,
    #[schema(min_length = 8, max_length = 128)]
    pub admin_password: String,
    pub blueprint: Option<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[schema(example = "America/Sao_Paulo")]
    pub timezone: String,
    #[schema(example = "BRL")]
    pub currency: String,
    #[schema(example = "pt-BR")]
    pub locale: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct HostRequest {
    #[schema(example = "app.acme.example.com")]
    pub host: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteTenantRequest {
    pub confirm: String,
    pub export: bool,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct QueueTotalsResponse {
    pub ready: u64,
    pub scheduled: u64,
    pub running: u64,
    pub failed: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkerResponse {
    pub instance_id: String,
    pub hostname: Option<String>,
    pub version: String,
    #[schema(format = DateTime)]
    pub last_seen_at: String,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct TenantHealthResponse {
    pub available: bool,
    pub error: Option<String>,
    pub queue: QueueTotalsResponse,
    pub oldest_ready_age_seconds: Option<i64>,
    pub workers: Vec<WorkerResponse>,
    pub pending_migrations: u64,
    pub plugin_backend_errors: Vec<String>,
    pub recent_failures: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogModuleResponse {
    pub requires_configuration: bool,
    pub key: String,
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogResponse {
    pub server_public_ip: Option<String>,
    pub modules: Vec<CatalogModuleResponse>,
    pub blueprints: Vec<InstallerProfile>,
    pub export_available: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEntryResponse {
    pub id: String,
    pub operator_email: Option<String>,
    pub action: String,
    pub tenant_id: Option<String>,
    #[schema(value_type = Object)]
    pub details: Value,
    pub ip: Option<String>,
    #[schema(format = DateTime)]
    pub occurred_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = Object)]
pub struct PluginPayload(pub serde_json::Value);
