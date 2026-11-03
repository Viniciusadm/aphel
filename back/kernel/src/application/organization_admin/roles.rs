use std::collections::BTreeSet;

use crate::application::access::Authorizer;
use crate::application::organization_admin::use_cases::{
    ROLE_IN_USE, ROLE_KEY_TAKEN, ROLE_NOT_FOUND, SYSTEM_ROLE_LOCKED, bounded_name, name_message,
    new_id, require_any,
};
use crate::application::organization_admin::{
    AdminChange, ROLE_CREATED, ROLE_DELETED, ROLE_UPDATED,
};
use crate::application::organization_admin::{
    CreateRoleCommand, GrantInput, ListWindow, Listing, OrganizationAdminPorts,
    OrganizationAdminUseCases, RoleChanges, RoleRepository, RoleView, UpdateRoleCommand,
};
use crate::application::organization_profile::NewRole;
use crate::application::security::{NewSecurityEvent, SecurityEventKind, record_quietly};
use crate::application::shared::ensure_version;
use crate::application::shared::{RecordType, TransactionManager};
use crate::domain::RoleId;
use crate::domain::access::{
    AccessScope, Permission, PermissionDefinition, PermissionGrant, RecordScope, RoleKey,
    permission_catalog,
};
use crate::errors::{AppError, FieldViolation};

const MAX_ROLE_KEY_LENGTH: usize = 64;
const KEY_ATTEMPTS: usize = 20;

impl<P> OrganizationAdminUseCases<P>
where
    P: OrganizationAdminPorts,
{
    pub fn permission_catalog(
        &self,
        scope: &AccessScope,
    ) -> Result<Vec<PermissionDefinition>, AppError> {
        require_any(
            scope,
            &[Permission::SETTINGS_MANAGE, Permission::USERS_MANAGE],
        )?;
        Ok(organization_catalog(scope))
    }

    pub async fn list_roles(
        &self,
        scope: &AccessScope,
        window: ListWindow,
    ) -> Result<Listing<RoleView>, AppError> {
        require_any(
            scope,
            &[Permission::SETTINGS_MANAGE, Permission::USERS_MANAGE],
        )?;
        self.ports.roles().list(window).await
    }

    pub async fn get_role(&self, scope: &AccessScope, role_id: &str) -> Result<RoleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.require_role(role_id).await
    }

    pub async fn create_role(
        &self,
        scope: &AccessScope,
        command: CreateRoleCommand,
    ) -> Result<RoleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;

        let mut violations = Vec::new();
        let name = bounded_name(&command.name);
        if name.is_none() {
            violations.push(FieldViolation::new("name", name_message()));
        }
        let explicit_key = match command.key.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(key) => {
                let parsed = RoleKey::new(key).ok().filter(|key| !is_reserved_key(key));
                if parsed.is_none() {
                    violations.push(key_violation());
                }
                parsed
            }
        };
        let grants = validate_grants(
            &organization_catalog(scope),
            command.permissions,
            &mut violations,
        );
        let Some(name) = name.filter(|_| violations.is_empty()) else {
            return Err(AppError::invalid_fields("invalid role", violations));
        };

        let key = match explicit_key {
            Some(key) => {
                if self.ports.roles().key_exists(&key).await? {
                    return Err(key_taken());
                }
                key
            }
            None => self.available_key(&name).await?,
        };

        let role_id = RoleId::new(new_id())?;
        let payload = serde_json::json!({ "key": key.as_str(), "name": name.as_str() });
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .roles()
            .create(
                &mut tx,
                NewRole {
                    id: role_id.clone(),
                    key,
                    name,
                    is_system: false,
                    grants,
                },
            )
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(ROLE_CREATED, RecordType::ROLE, role_id.as_str(), payload)?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.require_role(role_id.as_str()).await
    }

    pub async fn update_role(
        &self,
        scope: &AccessScope,
        role_id: &str,
        command: UpdateRoleCommand,
    ) -> Result<RoleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.require_role(role_id).await?;
        ensure_version(command.expected_updated_at, current.updated_at)?;

        let mut violations = Vec::new();
        let name = bounded_name(command.name.as_deref().unwrap_or(&current.role.name));
        if name.is_none() {
            violations.push(FieldViolation::new("name", name_message()));
        }
        let key = match command.key.as_deref().map(str::trim) {
            None | Some("") => current.role.key.clone(),
            Some(key) if key == current.role.key.as_str() => current.role.key.clone(),
            Some(_) if current.role.is_system => return Err(system_role_locked()),
            Some(key) => match RoleKey::new(key) {
                Ok(key) if !is_reserved_key(&key) => key,
                _ => {
                    violations.push(key_violation());
                    current.role.key.clone()
                }
            },
        };
        let grants = match command.permissions {
            Some(_) if current.role.is_system_admin() => return Err(system_role_locked()),
            Some(permissions) => Some(validate_grants(
                &organization_catalog(scope),
                permissions,
                &mut violations,
            )),
            None => None,
        };
        let Some(name) = name.filter(|_| violations.is_empty()) else {
            return Err(AppError::invalid_fields("invalid role", violations));
        };
        if key != current.role.key && self.ports.roles().key_exists(&key).await? {
            return Err(key_taken());
        }

        let permissions_changed = grants.is_some();
        let payload = serde_json::json!({
            "key": key.as_str(),
            "name": name.as_str(),
            "permissions_changed": permissions_changed,
        });
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .roles()
            .update(&mut tx, &current.role.id, RoleChanges { name, key, grants })
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                ROLE_UPDATED,
                RecordType::ROLE,
                current.role.id.as_str(),
                payload,
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        let updated = self.require_role(role_id).await?;
        if permissions_changed {
            record_quietly(
                &self.security_events,
                NewSecurityEvent::new(SecurityEventKind::PermissionChanged, self.now()).metadata(
                    serde_json::json!({
                        "change": "role_permissions",
                        "role_id": updated.role.id.as_str(),
                        "role_name": updated.role.name,
                        "actor_user_id": scope.user_id().as_str(),
                    }),
                ),
            )
            .await;
        }
        Ok(updated)
    }

    pub async fn delete_role(&self, scope: &AccessScope, role_id: &str) -> Result<(), AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.require_role(role_id).await?;
        if current.role.is_system {
            return Err(system_role_locked());
        }
        if current.member_count > 0 {
            return Err(AppError::conflict_code(
                ROLE_IN_USE,
                "the role is assigned to members of the organization",
            ));
        }
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.roles().delete(&mut tx, &current.role.id).await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                ROLE_DELETED,
                RecordType::ROLE,
                current.role.id.as_str(),
                serde_json::json!({ "key": current.role.key.as_str(), "name": current.role.name }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        Ok(())
    }

    async fn require_role(&self, role_id: &str) -> Result<RoleView, AppError> {
        let Ok(role_id) = RoleId::new(role_id) else {
            return Err(role_not_found());
        };
        self.ports
            .roles()
            .find(&role_id)
            .await?
            .ok_or_else(role_not_found)
    }

    async fn available_key(&self, name: &str) -> Result<RoleKey, AppError> {
        let base = match role_key_base(name) {
            base if base.starts_with(RESERVED_KEY_PREFIX) => format!("role_{base}")
                .chars()
                .take(MAX_ROLE_KEY_LENGTH)
                .collect(),
            base => base,
        };
        for attempt in 1..=KEY_ATTEMPTS {
            let candidate = if attempt == 1 {
                base.clone()
            } else {
                let suffix = format!("_{attempt}");
                let prefix: String = base
                    .chars()
                    .take(MAX_ROLE_KEY_LENGTH - suffix.len())
                    .collect();
                format!("{prefix}{suffix}")
            };
            let key = RoleKey::new(candidate)?;
            if !self.ports.roles().key_exists(&key).await? {
                return Ok(key);
            }
        }
        Err(key_taken())
    }
}

pub fn role_key_base(name: &str) -> String {
    let mut key = String::new();
    for character in name.chars().map(fold_accent) {
        if character.is_ascii_alphanumeric() {
            key.push(character.to_ascii_lowercase());
        } else if !key.is_empty() && !key.ends_with('_') {
            key.push('_');
        }
    }
    let key: String = key.chars().take(MAX_ROLE_KEY_LENGTH).collect();
    let key = key.trim_matches('_').to_owned();
    if key.is_empty() {
        "role".to_owned()
    } else {
        key
    }
}

fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        other => other,
    }
}

pub fn organization_catalog(scope: &AccessScope) -> Vec<PermissionDefinition> {
    permission_catalog()
        .into_iter()
        .chain(scope.permissions().catalog_extension().iter().cloned())
        .collect()
}

fn validate_grants(
    catalog: &[PermissionDefinition],
    inputs: Vec<GrantInput>,
    violations: &mut Vec<FieldViolation>,
) -> Vec<PermissionGrant> {
    let mut seen = BTreeSet::new();
    let mut grants = Vec::new();
    for (index, input) in inputs.into_iter().enumerate() {
        let field = format!("permissions[{index}].permission");
        let Some(definition) = catalog
            .iter()
            .find(|definition| definition.permission.as_str() == input.permission)
        else {
            violations.push(FieldViolation::new(field, "is not a known permission"));
            continue;
        };
        if !seen.insert(definition.permission.clone()) {
            violations.push(FieldViolation::new(field, "is duplicated"));
            continue;
        }
        grants.push(PermissionGrant {
            permission: definition.permission.clone(),
            scope: if definition.record_scoped {
                input.scope
            } else {
                RecordScope::Organization
            },
        });
    }
    grants
}

const RESERVED_KEY_PREFIX: &str = "platform_";

fn is_reserved_key(key: &RoleKey) -> bool {
    key.as_str().starts_with(RESERVED_KEY_PREFIX)
}

fn key_violation() -> FieldViolation {
    FieldViolation::new(
        "key",
        format!(
            "must have up to {MAX_ROLE_KEY_LENGTH} lowercase letters, digits or underscores and must not start with `{RESERVED_KEY_PREFIX}`"
        ),
    )
}

fn key_taken() -> AppError {
    AppError::conflict_code(ROLE_KEY_TAKEN, "another role already uses this key")
}

fn system_role_locked() -> AppError {
    AppError::conflict_code(
        SYSTEM_ROLE_LOCKED,
        "system roles cannot be deleted, renamed by key or, for the administrator, have permissions changed",
    )
}

fn role_not_found() -> AppError {
    AppError::not_found_code(ROLE_NOT_FOUND, "role was not found")
}
