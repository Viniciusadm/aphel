use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::organizations::{
    NewOrganization, NewOwner, NewRole, OrganizationRegistration,
};
use crate::domain::RoleId;
use crate::domain::access::system_roles;
use crate::errors::AppError;

pub fn organization_registration(
    organization: NewOrganization,
    owner: NewOwner,
    joined_at: DateTime<Utc>,
) -> Result<OrganizationRegistration, AppError> {
    let roles: Vec<NewRole> = system_roles()
        .into_iter()
        .map(|role| {
            Ok(NewRole {
                id: RoleId::new(Uuid::now_v7().to_string())?,
                key: role.key,
                name: role.name.to_owned(),
                is_system: true,
                grants: role.grants,
            })
        })
        .collect::<Result<_, AppError>>()?;
    let owner_role_id = roles
        .iter()
        .find(|role| role.key.is_admin())
        .map(|role| role.id.clone())
        .ok_or_else(|| AppError::Internal("admin system role is missing".to_owned()))?;
    Ok(OrganizationRegistration {
        organization,
        owner,
        roles,
        owner_role_id,
        joined_at,
    })
}
