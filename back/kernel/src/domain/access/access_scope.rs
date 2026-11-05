use std::collections::BTreeMap;

use crate::domain::access::{
    Permission, PermissionDefinition, PermissionGrant, RecordScope, RoleKey, permission_catalog,
};
use crate::domain::api_tokens::ApiTokenScopes;
use crate::domain::{TeamId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionSet {
    grants: BTreeMap<Permission, RecordScope>,
    grants_everything: bool,
    catalog_extension: Vec<PermissionDefinition>,
}

impl PermissionSet {
    pub fn from_grants(grants: impl IntoIterator<Item = PermissionGrant>) -> Self {
        let mut set = Self::default();
        for grant in grants {
            set.grant(grant.permission, grant.scope);
        }
        set
    }

    pub fn everything() -> Self {
        Self {
            grants: BTreeMap::new(),
            grants_everything: true,
            catalog_extension: Vec::new(),
        }
    }

    pub fn with_catalog_extension(mut self, permissions: Vec<PermissionDefinition>) -> Self {
        self.catalog_extension = permissions;
        self
    }

    pub fn catalog_extension(&self) -> &[PermissionDefinition] {
        &self.catalog_extension
    }

    pub fn grant(&mut self, permission: Permission, scope: RecordScope) {
        let current = self.grants.entry(permission).or_insert(scope);
        if scope > *current {
            *current = scope;
        }
    }

    pub fn scope_for(&self, permission: &Permission) -> Option<RecordScope> {
        if self.grants_everything {
            return Some(RecordScope::Organization);
        }
        self.grants.get(permission).copied()
    }

    pub fn grants_everything(&self) -> bool {
        self.grants_everything
    }

    pub fn restricted(&self, allows: impl Fn(&Permission) -> bool) -> Self {
        let grants: Vec<PermissionGrant> = if self.grants_everything {
            permission_catalog()
                .into_iter()
                .map(|definition| definition.permission)
                .chain(
                    self.catalog_extension
                        .iter()
                        .map(|definition| definition.permission.clone()),
                )
                .map(|permission| PermissionGrant {
                    permission,
                    scope: RecordScope::Organization,
                })
                .collect()
        } else {
            self.grants().collect()
        };
        Self::from_grants(grants.into_iter().filter(|grant| allows(&grant.permission)))
    }

    pub fn grants(&self) -> impl Iterator<Item = PermissionGrant> + '_ {
        self.grants
            .iter()
            .map(|(permission, scope)| PermissionGrant {
                permission: permission.clone(),
                scope: *scope,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareReach {
    None,
    Read,
    Edit,
}

impl ShareReach {
    pub fn for_permission(permission: &Permission) -> Self {
        if permission.grants_read_only() {
            Self::Read
        } else if permission.grants_shared_edit() {
            Self::Edit
        } else {
            Self::None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordScopeFilter {
    Organization,
    Owner {
        user_id: UserId,
        shares: ShareReach,
    },
    Teams {
        user_id: UserId,
        team_ids: Vec<TeamId>,
        shares: ShareReach,
    },
}

impl RecordScopeFilter {
    pub fn shares(&self) -> ShareReach {
        match self {
            Self::Organization => ShareReach::None,
            Self::Owner { shares, .. } | Self::Teams { shares, .. } => *shares,
        }
    }

    pub fn include_memberships(&self) -> bool {
        self.shares() == ShareReach::Read
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessScope {
    user_id: UserId,
    role_key: RoleKey,
    team_ids: Vec<TeamId>,
    permissions: PermissionSet,
    api_token: Option<ApiTokenGrant>,
    service: Option<(String, String)>,
    enabled_modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiTokenGrant {
    pub token_id: String,
    pub scopes: ApiTokenScopes,
}

impl AccessScope {
    pub fn new(
        user_id: UserId,
        role_key: RoleKey,
        team_ids: Vec<TeamId>,
        permissions: PermissionSet,
    ) -> Self {
        Self {
            user_id,
            role_key,
            team_ids,
            permissions,
            api_token: None,
            service: None,
            enabled_modules: Vec::new(),
        }
    }

    pub fn system() -> Self {
        Self::new(
            UserId::system(),
            RoleKey::system(),
            Vec::new(),
            PermissionSet::everything(),
        )
    }

    pub fn is_system(&self) -> bool {
        self.user_id == UserId::system()
    }

    pub fn with_enabled_modules(mut self, modules: Vec<String>) -> Self {
        self.enabled_modules = modules;
        self
    }

    pub fn enabled_modules(&self) -> &[String] {
        &self.enabled_modules
    }

    pub fn module_enabled(&self, key: &str) -> bool {
        self.enabled_modules.iter().any(|module| module == key)
    }

    pub fn acting_as(mut self, kind: impl Into<String>, id: impl Into<String>) -> Self {
        self.service = Some((kind.into(), id.into()));
        self
    }

    pub fn service(&self) -> Option<(&str, &str)> {
        self.service
            .as_ref()
            .map(|(kind, id)| (kind.as_str(), id.as_str()))
    }

    pub fn automation_id(&self) -> Option<&str> {
        self.service()
            .filter(|(kind, _)| *kind == "automation")
            .map(|(_, id)| id)
    }

    pub fn restricted(mut self, allows: impl Fn(&Permission) -> bool) -> Self {
        self.permissions = self.permissions.restricted(allows);
        self
    }

    pub fn plugin_key(&self) -> Option<&str> {
        self.service()
            .filter(|(kind, _)| *kind == "plugin")
            .map(|(_, id)| id)
    }

    pub fn through_api_token(
        mut self,
        token_id: impl Into<String>,
        scopes: ApiTokenScopes,
    ) -> Self {
        self.permissions = self
            .permissions
            .restricted(|permission| scopes.allows(permission));
        self.api_token = Some(ApiTokenGrant {
            token_id: token_id.into(),
            scopes,
        });
        self
    }

    pub fn api_token(&self) -> Option<&ApiTokenGrant> {
        self.api_token.as_ref()
    }

    pub fn api_token_id(&self) -> Option<&str> {
        self.api_token.as_ref().map(|grant| grant.token_id.as_str())
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn role_key(&self) -> &RoleKey {
        &self.role_key
    }

    pub fn team_ids(&self) -> &[TeamId] {
        &self.team_ids
    }

    pub fn permissions(&self) -> &PermissionSet {
        &self.permissions
    }

    pub fn scope_for(&self, permission: &Permission) -> Option<RecordScope> {
        self.permissions.scope_for(permission)
    }

    pub fn filter_for(&self, permission: &Permission) -> Option<RecordScopeFilter> {
        let shares = ShareReach::for_permission(permission);
        self.scope_for(permission).map(|scope| match scope {
            RecordScope::Organization => RecordScopeFilter::Organization,
            RecordScope::Team => RecordScopeFilter::Teams {
                user_id: self.user_id.clone(),
                team_ids: self.team_ids.clone(),
                shares,
            },
            RecordScope::Own => RecordScopeFilter::Owner {
                user_id: self.user_id.clone(),
                shares,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessGrantView {
    pub role_key: RoleKey,
    pub role_is_system: bool,
    pub grants: Vec<PermissionGrant>,
    pub team_ids: Vec<TeamId>,
    pub catalog_extension: Vec<PermissionDefinition>,
    pub enabled_modules: Vec<String>,
}
