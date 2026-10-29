use crate::domain::UserId;
use crate::domain::access::{
    AccessGrantView, AccessScope, Permission, PermissionSet, RecordScope, RecordScopeFilter,
};
use crate::errors::AppError;

pub const PERMISSION_DENIED: &str = "permission_denied";
pub const INSUFFICIENT_SCOPE: &str = "insufficient_scope";

pub trait Authorizer {
    fn require(&self, permission: &Permission) -> Result<RecordScope, AppError>;
    fn record_filter(&self, permission: &Permission) -> Result<RecordScopeFilter, AppError>;
}

impl Authorizer for AccessScope {
    fn require(&self, permission: &Permission) -> Result<RecordScope, AppError> {
        self.scope_for(permission)
            .ok_or_else(|| denied(self, permission))
    }

    fn record_filter(&self, permission: &Permission) -> Result<RecordScopeFilter, AppError> {
        self.filter_for(permission)
            .ok_or_else(|| denied(self, permission))
    }
}

fn denied(scope: &AccessScope, permission: &Permission) -> AppError {
    if let Some(grant) = scope.api_token()
        && !grant.scopes.allows(permission)
    {
        return AppError::forbidden_code(
            INSUFFICIENT_SCOPE,
            format!("the API token does not have the scope for {permission}"),
        );
    }
    AppError::forbidden_code(
        PERMISSION_DENIED,
        format!("missing permission {permission}"),
    )
}

pub fn build_access_scope(user_id: UserId, access: AccessGrantView) -> AccessScope {
    let permissions = if access.role_is_system && access.role_key.is_admin() {
        PermissionSet::everything().with_catalog_extension(access.catalog_extension)
    } else {
        PermissionSet::from_grants(access.grants).with_catalog_extension(access.catalog_extension)
    };
    AccessScope::new(user_id, access.role_key, access.team_ids, permissions)
        .with_enabled_modules(access.enabled_modules)
}
