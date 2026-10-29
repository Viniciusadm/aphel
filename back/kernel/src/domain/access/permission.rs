use std::borrow::Cow;
use std::fmt;

use crate::domain::DomainValidationError;

const MAX_PERMISSION_LENGTH: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Permission(Cow<'static, str>);

impl Permission {
    pub const CONTACTS_READ: Self = Self::from_static("contacts.read");
    pub const CONTACTS_CREATE: Self = Self::from_static("contacts.create");
    pub const CONTACTS_UPDATE: Self = Self::from_static("contacts.update");
    pub const CONTACTS_DELETE: Self = Self::from_static("contacts.delete");
    pub const CONTACTS_EXPORT: Self = Self::from_static("contacts.export");
    pub const CONTACTS_ARCHIVE: Self = Self::from_static("contacts.archive");
    pub const COMPANIES_READ: Self = Self::from_static("companies.read");
    pub const COMPANIES_CREATE: Self = Self::from_static("companies.create");
    pub const COMPANIES_UPDATE: Self = Self::from_static("companies.update");
    pub const COMPANIES_DELETE: Self = Self::from_static("companies.delete");
    pub const COMPANIES_EXPORT: Self = Self::from_static("companies.export");
    pub const COMPANIES_ARCHIVE: Self = Self::from_static("companies.archive");
    pub const DEALS_READ: Self = Self::from_static("deals.read");
    pub const DEALS_CREATE: Self = Self::from_static("deals.create");
    pub const DEALS_UPDATE: Self = Self::from_static("deals.update");
    pub const DEALS_DELETE: Self = Self::from_static("deals.delete");
    pub const DEALS_EXPORT: Self = Self::from_static("deals.export");
    pub const DEALS_ARCHIVE: Self = Self::from_static("deals.archive");
    pub const ACTIVITIES_READ: Self = Self::from_static("activities.read");
    pub const ACTIVITIES_CREATE: Self = Self::from_static("activities.create");
    pub const ACTIVITIES_UPDATE: Self = Self::from_static("activities.update");
    pub const ACTIVITIES_DELETE: Self = Self::from_static("activities.delete");
    pub const ACTIVITIES_EXPORT: Self = Self::from_static("activities.export");
    pub const NOTES_READ: Self = Self::from_static("notes.read");
    pub const NOTES_CREATE: Self = Self::from_static("notes.create");
    pub const NOTES_UPDATE: Self = Self::from_static("notes.update");
    pub const NOTES_DELETE: Self = Self::from_static("notes.delete");
    pub const EMAILS_READ: Self = Self::from_static("emails.read");
    pub const EMAILS_SEND: Self = Self::from_static("emails.send");
    pub const SEQUENCES_READ: Self = Self::from_static("sequences.read");
    pub const SEQUENCES_ENROLL: Self = Self::from_static("sequences.enroll");
    pub const SCHEDULING_MANAGE: Self = Self::from_static("scheduling.manage");
    pub const GOALS_MANAGE: Self = Self::from_static("goals.manage");
    pub const PRODUCTS_READ: Self = Self::from_static("products.read");
    pub const PRODUCTS_CREATE: Self = Self::from_static("products.create");
    pub const PRODUCTS_UPDATE: Self = Self::from_static("products.update");
    pub const PRODUCTS_DELETE: Self = Self::from_static("products.delete");
    pub const QUOTES_READ: Self = Self::from_static("quotes.read");
    pub const QUOTES_CREATE: Self = Self::from_static("quotes.create");
    pub const QUOTES_UPDATE: Self = Self::from_static("quotes.update");
    pub const QUOTES_DELETE: Self = Self::from_static("quotes.delete");
    pub const QUOTES_SEND: Self = Self::from_static("quotes.send");
    pub const AUTOMATIONS_MANAGE: Self = Self::from_static("automations.manage");
    pub const LEAD_DISTRIBUTION_MANAGE: Self = Self::from_static("lead_distribution.manage");
    pub const SEQUENCES_MANAGE: Self = Self::from_static("sequences.manage");
    pub const APPROVALS_MANAGE: Self = Self::from_static("approvals.manage");
    pub const LIFECYCLE_MANAGE: Self = Self::from_static("lifecycle.manage");
    pub const SCORING_MANAGE: Self = Self::from_static("scoring.manage");
    pub const SETTINGS_MANAGE: Self = Self::from_static("settings.manage");
    pub const USERS_MANAGE: Self = Self::from_static("users.manage");
    pub const AUDIT_READ: Self = Self::from_static("audit.read");
    pub const SYSTEM_MANAGE: Self = Self::from_static("system.manage");
    pub const PRIVACY_MANAGE: Self = Self::from_static("privacy.manage");
    pub const ORGANIZATION_EXPORT: Self = Self::from_static("organization.export");
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
                    && segment
                        .chars()
                        .all(|character| character.is_ascii_lowercase() || character == '_')
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
        })
        .chain(
            administrative
                .into_iter()
                .map(|permission| PermissionDefinition {
                    permission,
                    record_scoped: false,
                }),
        )
        .collect()
}

impl PermissionDefinition {
    pub fn record(permission: Permission) -> Self {
        Self {
            permission,
            record_scoped: true,
        }
    }

    pub fn administrative(permission: Permission) -> Self {
        Self {
            permission,
            record_scoped: false,
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
