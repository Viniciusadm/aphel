use std::borrow::Cow;
use std::fmt;

use crate::domain::DomainValidationError;

const MAX_PERMISSION_LENGTH: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Permission(Cow<'static, str>);

impl Permission {
    pub const SETTINGS_MANAGE: Self = Self::from_static("settings.manage");
    pub const USERS_MANAGE: Self = Self::from_static("users.manage");
    pub const AUDIT_READ: Self = Self::from_static("audit.read");
    pub const SYSTEM_MANAGE: Self = Self::from_static("system.manage");
    pub const PRIVACY_MANAGE: Self = Self::from_static("privacy.manage");
    pub const RECORDS_SHARE: Self = Self::from_static("records.share");

    pub const fn from_static(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        let mut segments = value.split('.');
        let valid = value.len() <= MAX_PERMISSION_LENGTH
            && value.split('.').count() >= 2
            && segments.all(|segment| {
                !segment.is_empty()
                    && segment.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '_'
                    })
            });
        if !valid {
            return Err(DomainValidationError::Invalid("permission"));
        }
        Ok(Self(Cow::Owned(value)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn grants_read_only(&self) -> bool {
        self.0.ends_with(".read") || self.0.ends_with(".export")
    }

    pub fn grants_shared_edit(&self) -> bool {
        self.0.ends_with(".update") || self.0.ends_with(".archive")
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDefinition {
    pub permission: Permission,
    pub record_scoped: bool,
    pub label: Option<String>,
    pub description: Option<String>,
}

pub fn builtin_permission_catalog() -> Vec<PermissionDefinition> {
    let record: [Permission; 0] = [];
    let administrative = [
        Permission::SETTINGS_MANAGE,
        Permission::USERS_MANAGE,
        Permission::AUDIT_READ,
        Permission::SYSTEM_MANAGE,
        Permission::RECORDS_SHARE,
    ];

    record
        .into_iter()
        .map(|permission| PermissionDefinition {
            permission,
            record_scoped: true,
            label: None,
            description: None,
        })
        .chain(
            administrative
                .into_iter()
                .map(|permission| PermissionDefinition {
                    permission,
                    record_scoped: false,
                    label: None,
                    description: None,
                }),
        )
        .collect()
}

impl PermissionDefinition {
    pub fn record(permission: Permission) -> Self {
        Self {
            permission,
            record_scoped: true,
            label: None,
            description: None,
        }
    }

    pub fn administrative(permission: Permission) -> Self {
        Self {
            permission,
            record_scoped: false,
            label: None,
            description: None,
        }
    }
}

pub fn permission_catalog() -> Vec<PermissionDefinition> {
    crate::modules::app()
        .catalog()
        .permissions()
        .cloned()
        .collect()
}

pub fn is_known_permission(permission: &Permission) -> bool {
    crate::modules::app()
        .catalog()
        .permissions()
        .any(|definition| &definition.permission == permission)
}
