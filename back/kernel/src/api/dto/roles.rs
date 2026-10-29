use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::auth::{PermissionGrantDto, RecordScopeDto};
use crate::api::dto::members::timestamp;
use crate::application::organization_admin::{
    CreateRoleCommand, GrantInput, RoleView, UpdateRoleCommand,
};
use crate::domain::access::{PermissionDefinition, permission_catalog};

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionDefinitionResponse {
    #[schema(example = "contacts.read")]
    pub permission: String,
    pub record_scoped: bool,
}

impl From<PermissionDefinition> for PermissionDefinitionResponse {
    fn from(definition: PermissionDefinition) -> Self {
        Self {
            permission: definition.permission.as_str().to_owned(),
            record_scoped: definition.record_scoped,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: String,
    #[schema(example = "sales_rep")]
    pub key: String,
    pub name: String,
    pub is_system: bool,
    pub grants_everything: bool,
    pub member_count: u64,
    pub permissions: Vec<PermissionGrantDto>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
}

impl From<RoleView> for RoleResponse {
    fn from(view: RoleView) -> Self {
        let grants_everything = view.role.is_system_admin();
        let permissions = if grants_everything {
            permission_catalog()
                .into_iter()
                .map(|definition| PermissionGrantDto {
                    permission: definition.permission.as_str().to_owned(),
                    scope: RecordScopeDto::Organization,
                })
                .collect()
        } else {
            view.grants
                .into_iter()
                .map(|grant| PermissionGrantDto {
                    permission: grant.permission.as_str().to_owned(),
                    scope: grant.scope.into(),
                })
                .collect()
        };
        Self {
            id: view.role.id.into_inner(),
            key: view.role.key.as_str().to_owned(),
            name: view.role.name,
            is_system: view.role.is_system,
            grants_everything,
            member_count: view.member_count,
            permissions,
            updated_at: timestamp(view.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PermissionGrantRequest {
    #[schema(example = "contacts.read")]
    pub permission: String,
    pub scope: RecordScopeDto,
}

impl From<PermissionGrantRequest> for GrantInput {
    fn from(request: PermissionGrantRequest) -> Self {
        Self {
            permission: request.permission,
            scope: request.scope.into(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    #[schema(example = "Agent")]
    pub name: String,
    #[schema(example = "agent")]
    pub key: Option<String>,
    #[serde(default)]
    pub permissions: Vec<PermissionGrantRequest>,
}

impl From<CreateRoleRequest> for CreateRoleCommand {
    fn from(request: CreateRoleRequest) -> Self {
        Self {
            name: request.name,
            key: request.key,
            permissions: request.permissions.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    #[schema(example = "Senior agent")]
    pub name: Option<String>,
    #[schema(example = "senior_agent")]
    pub key: Option<String>,
    pub permissions: Option<Vec<PermissionGrantRequest>>,
}

impl UpdateRoleRequest {
    pub fn into_command(self, expected_updated_at: Option<DateTime<Utc>>) -> UpdateRoleCommand {
        UpdateRoleCommand {
            name: self.name,
            key: self.key,
            permissions: self
                .permissions
                .map(|permissions| permissions.into_iter().map(Into::into).collect()),
            expected_updated_at,
        }
    }
}
