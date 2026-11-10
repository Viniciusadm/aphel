use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::access::{AccessRepository, access_catalog_generation, requester_scope};
use crate::domain::access::AccessScope;
use crate::domain::access::{
    AccessGrantView, Permission, PermissionDefinition, PermissionGrant, RecordScope, RoleKey,
};
use crate::domain::custom_objects::{OBJECT_ACTIONS, object_permission, object_record_type};
use crate::domain::{TeamId, UserId};
use crate::errors::AppError;
use crate::infrastructure::api_tokens::MySqlApiTokenRepository;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::runtime_permissions::active_runtime_permissions;

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
        let Some(row) = sqlx::query(
            "SELECT r.`key` AS role_key, r.is_system, \
             (SELECT JSON_ARRAYAGG(JSON_ARRAY(rp.permission, rp.scope)) \
              FROM role_permissions rp WHERE rp.role_id = r.id) AS grants, \
             (WITH RECURSIVE scoped_teams (id) AS ( \
                SELECT t.id FROM teams t JOIN team_memberships tm ON tm.team_id = t.id \
                WHERE tm.user_id = ? \
                UNION \
                SELECT child.id FROM teams child JOIN scoped_teams parent ON child.parent_team_id = parent.id \
              ) SELECT JSON_ARRAYAGG(id) FROM scoped_teams) AS team_ids \
             FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE u.id = ? AND u.status = 'active'",
        )
        .bind(user_id.as_str())
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let stored_grants: Option<Json<Vec<(String, String)>>> = row.try_get("grants")?;
        let grants = stored_grants
            .map(|grants| grants.0)
            .unwrap_or_default()
            .into_iter()
            .map(|(permission, scope)| {
                Ok(PermissionGrant {
                    permission: Permission::parse(permission.clone()).map_err(|_| {
                        mapper::invalid_column("role_permissions.permission", &permission)
                    })?,
                    scope: RecordScope::parse(&scope)
                        .ok_or_else(|| mapper::invalid_column("role_permissions.scope", &scope))?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        let stored_teams: Option<Json<Vec<String>>> = row.try_get("team_ids")?;
        let mut team_ids = stored_teams.map(|teams| teams.0).unwrap_or_default();
        team_ids.sort();
        team_ids.dedup();
        let team_ids = team_ids
            .into_iter()
            .map(|id| mapper::typed_id(id, TeamId::new))
            .collect::<Result<Vec<_>, AppError>>()?;

        let catalog = access_catalog(&self.pool).await?;
        let catalog_extension = catalog.permissions.clone();
        let enabled_modules = catalog.enabled_modules.clone();
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
            role_key: RoleKey::new(row.try_get::<String, _>("role_key")?)?,
            role_is_system: row.try_get("is_system")?,
            grants,
            team_ids,
            catalog_extension,
            enabled_modules,
        }))
    }
}

const ACCESS_CATALOG_TTL: Duration = Duration::from_secs(2);

struct AccessCatalog {
    loaded_at: Instant,
    generation: u64,
    permissions: Vec<PermissionDefinition>,
    enabled_modules: Vec<String>,
}

static ACCESS_CATALOG: LazyLock<RwLock<Option<Arc<AccessCatalog>>>> =
    LazyLock::new(RwLock::default);

async fn access_catalog(pool: &MySqlPool) -> Result<Arc<AccessCatalog>, AppError> {
    let generation = access_catalog_generation();
    let cached = ACCESS_CATALOG.read().ok().and_then(|cache| cache.clone());
    if let Some(cached) = cached
        && cached.generation == generation
        && cached.loaded_at.elapsed() < ACCESS_CATALOG_TTL
    {
        return Ok(cached);
    }
    let mut permissions: Vec<_> = custom_object_permissions(pool)
        .await?
        .into_iter()
        .map(PermissionDefinition::record)
        .collect();
    permissions.extend(active_runtime_permissions(pool).await?);
    let catalog = Arc::new(AccessCatalog {
        loaded_at: Instant::now(),
        generation,
        permissions,
        enabled_modules: enabled_modules(pool).await?,
    });
    if access_catalog_generation() == generation
        && let Ok(mut cache) = ACCESS_CATALOG.write()
    {
        *cache = Some(catalog.clone());
    }
    Ok(catalog)
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
    let objects: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT `key`, module_key FROM custom_objects WHERE is_active ORDER BY position, `key`",
    )
    .fetch_all(pool)
    .await?;
    Ok(objects
        .iter()
        .flat_map(|(key, module_key)| {
            let record_type = match module_key {
                Some(owner) => format!(
                    "{owner}.{}",
                    key.strip_prefix(&format!("{owner}_")).unwrap_or(key)
                ),
                None => object_record_type(key),
            };
            OBJECT_ACTIONS
                .iter()
                .filter_map(move |action| object_permission(&record_type, action))
        })
        .collect())
}
