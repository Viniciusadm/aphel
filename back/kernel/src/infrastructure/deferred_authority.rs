use crate::application::access::{AccessRepository, build_access_scope};
use crate::domain::{
    UserId,
    access::{AccessScope, Permission, PermissionGrant, PermissionSet, RecordScope},
};
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, types::Json};

#[async_trait]
pub trait DeferredPluginPolicy: Send + Sync {
    async fn reauthorize(
        &self,
        pool: &MySqlPool,
        key: &str,
        scope: AccessScope,
    ) -> Result<AccessScope, AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredAuthority {
    user_id: String,
    plugin: Option<String>,
    automation: Option<String>,
    execution: Option<crate::application::shared::ExecutionContext>,
    token_id: Option<String>,
    permissions: Vec<(String, String)>,
    teams: Vec<String>,
}

impl DeferredAuthority {
    pub fn capture(scope: &AccessScope) -> Self {
        Self {
            user_id: scope.user_id().as_str().to_owned(),
            plugin: scope.plugin_key().map(str::to_owned),
            automation: scope.automation_id().map(str::to_owned),
            execution: crate::application::shared::ExecutionContext::current(),
            token_id: scope.api_token_id().map(str::to_owned),
            permissions: scope
                .permissions()
                .restricted(|_| true)
                .grants()
                .map(|grant| {
                    (
                        grant.permission.to_string(),
                        grant.scope.as_str().to_owned(),
                    )
                })
                .collect(),
            teams: scope
                .team_ids()
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        }
    }

    pub async fn run<F: std::future::Future>(&self, future: F) -> F::Output {
        match &self.execution {
            Some(execution) => execution.scope(future).await,
            None => future.await,
        }
    }

    pub async fn resolve(&self, pool: &MySqlPool) -> Result<AccessScope, AppError> {
        let user = UserId::new(self.user_id.clone())?;
        let access = MySqlAccessRepository::new(pool.clone())
            .load_access(&user)
            .await?
            .ok_or_else(|| {
                AppError::forbidden_code(
                    "plugin_authority_inactive",
                    "the original authority is no longer active",
                )
            })?;
        let current = build_access_scope(user.clone(), access);
        let grants = self.permissions.iter().filter_map(|(key, ceiling)| {
            let permission = Permission::parse(key.clone()).ok()?;
            let current_scope = current.scope_for(&permission)?;
            Some(PermissionGrant {
                permission,
                scope: current_scope.min(RecordScope::parse(ceiling)?),
            })
        });
        let mut scope = AccessScope::new(
            user,
            current.role_key().clone(),
            current
                .team_ids()
                .iter()
                .filter(|id| self.teams.iter().any(|saved| saved == id.as_str()))
                .cloned()
                .collect(),
            PermissionSet::from_grants(grants),
        )
        .with_enabled_modules(current.enabled_modules().to_vec());
        if let Some(automation) = &self.automation {
            scope = scope.acting_as("automation", automation);
        }
        if let Some(plugin) = &self.plugin {
            let policy = crate::modules::app()
                .services()
                .require::<dyn DeferredPluginPolicy>()?;
            scope = policy.reauthorize(pool, plugin, scope).await?;
        }
        if let Some(token) = &self.token_id {
            let scopes: Option<Json<Vec<String>>> = sqlx::query_scalar("SELECT scopes FROM api_tokens WHERE id = ? AND created_by = ? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > UTC_TIMESTAMP(6))")
                .bind(token).bind(&self.user_id).fetch_optional(pool).await?;
            let Json(scopes) = scopes.ok_or_else(|| {
                AppError::forbidden_code(
                    "plugin_authority_inactive",
                    "the original API token is revoked or expired",
                )
            })?;
            scope = scope.through_api_token(
                token,
                crate::domain::api_tokens::ApiTokenScopes::parse(
                    scopes.iter().map(String::as_str),
                )?,
            );
        }
        Ok(scope)
    }
}
