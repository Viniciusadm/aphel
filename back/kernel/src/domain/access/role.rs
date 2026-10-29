use crate::domain::DomainValidationError;
use crate::domain::access::{Permission, RecordScope, permission_catalog};

const MAX_ROLE_KEY_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleKey(String);

impl RoleKey {
    pub const ADMIN: &'static str = "admin";
    pub const MANAGER: &'static str = "manager";
    pub const MEMBER: &'static str = "member";
    pub const SYSTEM: &'static str = "system";

    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_ROLE_KEY_LENGTH
            && value.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            });
        if !valid {
            return Err(DomainValidationError::Invalid("role_key"));
        }
        Ok(Self(value))
    }

    pub fn admin() -> Self {
        Self(Self::ADMIN.to_owned())
    }

    pub fn system() -> Self {
        Self(Self::SYSTEM.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_admin(&self) -> bool {
        self.0 == Self::ADMIN
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionGrant {
    pub permission: Permission,
    pub scope: RecordScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemRole {
    pub key: RoleKey,
    pub name: &'static str,
    pub grants: Vec<PermissionGrant>,
}

pub fn system_roles() -> Vec<SystemRole> {
    let catalog = permission_catalog();
    let declared: Vec<crate::modules::PermissionGrant> = crate::modules::app()
        .catalog()
        .grants
        .iter()
        .map(|contribution| contribution.item.clone())
        .collect();
    let admin = catalog
        .iter()
        .map(|definition| PermissionGrant {
            permission: definition.permission.clone(),
            scope: RecordScope::Organization,
        })
        .collect();
    let role_grants = |role: &str, default_scope: RecordScope| -> Vec<PermissionGrant> {
        catalog
            .iter()
            .filter_map(|definition| {
                let grants: Vec<&crate::modules::PermissionGrant> = declared
                    .iter()
                    .filter(|grant| grant.permission == definition.permission)
                    .collect();
                if grants.is_empty() {
                    let default = definition.record_scoped
                        && (role == RoleKey::MANAGER
                            || !definition.permission.as_str().ends_with(".export"));
                    return default.then(|| PermissionGrant {
                        permission: definition.permission.clone(),
                        scope: default_scope,
                    });
                }
                let grant = grants.iter().find(|grant| grant.roles.contains(&role))?;
                let scope = if !definition.record_scoped || grant.organization_wide {
                    RecordScope::Organization
                } else {
                    default_scope
                };
                Some(PermissionGrant {
                    permission: definition.permission.clone(),
                    scope,
                })
            })
            .collect()
    };
    let manager = role_grants(RoleKey::MANAGER, RecordScope::Team);
    let member = role_grants(RoleKey::MEMBER, RecordScope::Own);

    vec![
        SystemRole {
            key: RoleKey(RoleKey::ADMIN.to_owned()),
            name: "Administrador",
            grants: admin,
        },
        SystemRole {
            key: RoleKey(RoleKey::MANAGER.to_owned()),
            name: "Gerente",
            grants: manager,
        },
        SystemRole {
            key: RoleKey(RoleKey::MEMBER.to_owned()),
            name: "Colaborador",
            grants: member,
        },
    ]
}
