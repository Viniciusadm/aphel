use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::access::{AccessRepository, requester_scope};
use crate::domain::access::AccessScope;
use crate::domain::access::{AccessGrantView, Permission, PermissionGrant, RecordScope, RoleKey};
use crate::domain::custom_objects::{OBJECT_ACTIONS, object_permission, object_record_type};
use crate::domain::{TeamId, UserId};
use crate::errors::AppError;
use crate::infrastructure::api_tokens::MySqlApiTokenRepository;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlAccessRepository {
    pool: MySqlPool,
}

impl MySqlAccessRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

pub async fn build_requester_scope(
    pool: &MySqlPool,
    user_id: &UserId,
    api_token_id: Option<Uuid>,
) -> Result<Option<AccessScope>, AppError> {
    requester_scope(
        &MySqlApiTokenRepository::new(pool.clone()),
        &MySqlAccessRepository::new(pool.clone()),
        user_id,
        api_token_id,
        Utc::now(),
    )
    .await
}

#[async_trait]
impl AccessRepository for MySqlAccessRepository {
    async fn load_access(&self, user_id: &UserId) -> Result<Option<AccessGrantView>, AppError> {
        let Some(role) = sqlx::query(
            "SELECT r.id, r.`key` AS role_key, r.is_system \
             FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE u.id = ? AND u.status = 'active'",
        )
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let role_id: String = role.try_get("id")?;

        let grants =
            sqlx::query("SELECT permission, scope FROM role_permissions WHERE role_id = ?")
                .bind(&role_id)
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(|row| {
                    let permission: String = row.try_get("permission")?;
                    let scope: String = row.try_get("scope")?;
                    Ok(PermissionGrant {
                        permission: Permission::parse(permission.clone()).map_err(|_| {
                            mapper::invalid_column("role_permissions.permission", &permission)
                        })?,
                        scope: RecordScope::parse(&scope).ok_or_else(|| {
                            mapper::invalid_column("role_permissions.scope", &scope)
                        })?,
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?;

        let team_ids = sqlx::query_scalar::<_, String>(
            "WITH RECURSIVE scoped_teams (id) AS ( \
               SELECT t.id FROM teams t JOIN team_memberships tm ON tm.team_id = t.id \
               WHERE tm.user_id = ? \
               UNION \
               SELECT child.id FROM teams child JOIN scoped_teams parent ON child.parent_team_id = parent.id \
             ) SELECT id FROM scoped_teams ORDER BY id",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|id| mapper::typed_id(id, TeamId::new))
        .collect::<Result<Vec<_>, AppError>>()?;

        let catalog_extension = custom_object_permissions(&self.pool).await?;
        let enabled_modules = enabled_modules(&self.pool).await?;
        let modules = crate::modules::app();
        let grants = grants
            .into_iter()
            .filter(|grant| {
                modules
                    .catalog()
                    .permission_owner(&grant.permission)
                    .is_none_or(|owner| enabled_modules.iter().any(|key| key == owner))
            })
            .collect();

        Ok(Some(AccessGrantView {
            role_key: RoleKey::new(role.try_get::<String, _>("role_key")?)?,
            role_is_system: role.try_get("is_system")?,
            grants,
            team_ids,
            catalog_extension,
            enabled_modules,
        }))
    }
}

pub async fn enabled_modules(pool: &MySqlPool) -> Result<Vec<String>, AppError> {
    let app = crate::modules::app();
    let mut keys: Vec<String> = crate::modules::enabled::stored_enabled(pool)
        .await?
        .into_iter()
        .filter(|key| app.has_module(key))
        .collect();
    for entry in app.modules() {
        if entry.manifest.required
            && entry.is_compatible()
            && !keys.iter().any(|key| key == entry.key())
        {
            keys.push(entry.key().to_owned());
        }
    }
    keys.sort();
    Ok(keys)
}

pub(crate) async fn custom_object_permissions(
    pool: &MySqlPool,
) -> Result<Vec<Permission>, AppError> {
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT `key` FROM custom_objects WHERE is_active ORDER BY position, `key`",
    )
    .fetch_all(pool)
    .await?;
    Ok(keys
        .iter()
        .flat_map(|key| {
            let record_type = object_record_type(key);
            OBJECT_ACTIONS
                .iter()
                .filter_map(move |action| object_permission(&record_type, action))
        })
        .collect())
}
