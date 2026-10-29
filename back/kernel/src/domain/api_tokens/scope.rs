use std::collections::BTreeSet;
use std::fmt;

use crate::domain::DomainValidationError;
use crate::domain::access::{Permission, permission_catalog};
use crate::domain::custom_objects::is_custom_record_type;

const READ_ACTION: &str = "read";
const MAX_SCOPES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScopeAccess {
    Read,
    Write,
}

impl ScopeAccess {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            _ => None,
        }
    }

    fn for_action(action: &str) -> Self {
        if action == READ_ACTION {
            Self::Read
        } else {
            Self::Write
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ApiTokenScope {
    resource: String,
    access: ScopeAccess,
}

impl ApiTokenScope {
    pub fn new(resource: impl Into<String>, access: ScopeAccess) -> Self {
        Self {
            resource: resource.into(),
            access,
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        let (resource, access) = value
            .trim()
            .split_once(':')
            .ok_or(DomainValidationError::Invalid("scope"))?;
        let access = ScopeAccess::parse(access).ok_or(DomainValidationError::Invalid("scope"))?;
        let scope = Self::new(resource, access);
        if !is_custom_record_type(scope.resource()) && !scope_catalog().contains(&scope) {
            return Err(DomainValidationError::Invalid("scope"));
        }
        Ok(scope)
    }

    pub fn for_permission(permission: &Permission) -> Option<Self> {
        let (resource, action) = permission.as_str().rsplit_once('.')?;
        Some(Self::new(resource, ScopeAccess::for_action(action)))
    }

    pub fn resource(&self) -> &str {
        &self.resource
    }

    pub fn access(&self) -> ScopeAccess {
        self.access
    }
}

impl fmt::Display for ApiTokenScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.resource, self.access.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApiTokenScopes(BTreeSet<ApiTokenScope>);

impl ApiTokenScopes {
    pub fn parse<'a>(
        values: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, DomainValidationError> {
        let scopes = values
            .into_iter()
            .map(ApiTokenScope::parse)
            .collect::<Result<BTreeSet<_>, _>>()?;
        if scopes.is_empty() || scopes.len() > MAX_SCOPES {
            return Err(DomainValidationError::Invalid("scopes"));
        }
        Ok(Self(scopes))
    }

    pub fn allows(&self, permission: &Permission) -> bool {
        ApiTokenScope::for_permission(permission).is_some_and(|scope| self.0.contains(&scope))
    }

    pub fn iter(&self) -> impl Iterator<Item = &ApiTokenScope> {
        self.0.iter()
    }

    pub fn to_strings(&self) -> Vec<String> {
        self.0.iter().map(ToString::to_string).collect()
    }
}

pub fn scope_catalog() -> Vec<ApiTokenScope> {
    permission_catalog()
        .iter()
        .filter_map(|definition| ApiTokenScope::for_permission(&definition.permission))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
