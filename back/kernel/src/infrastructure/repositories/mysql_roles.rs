use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::organization_admin::{
    ListWindow, Listing, ROLE_IN_USE, ROLE_KEY_TAKEN, RoleChanges, RoleRepository, RoleSummary,
    RoleView,
};
use crate::application::organization_profile::NewRole;
use crate::database::transaction::MySqlTx;
use crate::domain::RoleId;
use crate::domain::access::{Permission, PermissionGrant, RecordScope, RoleKey};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

macro_rules! role_select {
    () => {
        "SELECT r.id, r.`key`, r.name, r.is_system, r.updated_at, \
     (SELECT COUNT(*) FROM users u WHERE u.role_id = r.id) AS member_count \
     FROM roles r"
    };
}

#[derive(Clone)]
pub struct MySqlRoleRepository {
    pool: MySqlPool,
}

impl MySqlRoleRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn with_grants(&self, rows: Vec<MySqlRow>) -> Result<Vec<RoleView>, AppError> {
        let mut roles = rows
            .iter()
            .map(role_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        if roles.is_empty() {
            return Ok(roles);
        }

        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT role_id, permission, scope FROM role_permissions WHERE role_id IN (",
        );
        let mut separated = builder.separated(", ");
        for role in &roles {
            separated.push_bind(role.role.id.as_str());
        }
        builder.push(") ORDER BY permission");

        let mut grants: BTreeMap<String, Vec<PermissionGrant>> = BTreeMap::new();
        for row in builder.build().fetch_all(&self.pool).await? {
            let permission: String = row.try_get("permission")?;
            let scope: String = row.try_get("scope")?;
            grants
                .entry(row.try_get("role_id")?)
                .or_default()
                .push(PermissionGrant {
                    permission: Permission::parse(permission.clone()).map_err(|_| {
                        mapper::invalid_column("role_permissions.permission", &permission)
                    })?,
                    scope: RecordScope::parse(&scope)
                        .ok_or_else(|| mapper::invalid_column("role_permissions.scope", &scope))?,
                });
        }
        for role in &mut roles {
            role.grants = grants.remove(role.role.id.as_str()).unwrap_or_default();
        }
        Ok(roles)
    }
}

fn role_from_row(row: &MySqlRow) -> Result<RoleView, AppError> {
    let member_count: i64 = row.try_get("member_count")?;
    Ok(RoleView {
        role: RoleSummary {
            id: mapper::typed_id(row.try_get("id")?, RoleId::new)?,
            key: RoleKey::new(row.try_get::<String, _>("key")?)?,
            name: row.try_get("name")?,
            is_system: row.try_get("is_system")?,
        },
        grants: Vec::new(),
        member_count: u64::try_from(member_count).unwrap_or_default(),
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

fn map_key_conflict(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.is_unique_violation()
    {
        return AppError::conflict_code(ROLE_KEY_TAKEN, "another role already uses this key");
    }
    AppError::from(error)
}

async fn insert_grants(
    transaction: &mut sqlx::MySqlConnection,
    role_id: &RoleId,
    grants: &[PermissionGrant],
) -> Result<(), AppError> {
    if grants.is_empty() {
        return Ok(());
    }
    let mut builder =
        QueryBuilder::<MySql>::new("INSERT INTO role_permissions (role_id, permission, scope) ");
    builder.push_values(grants, |mut values, grant| {
        values
            .push_bind(role_id.as_str())
            .push_bind(grant.permission.as_str())
            .push_bind(grant.scope.as_str());
    });
    builder.build().execute(&mut *transaction).await?;
    Ok(())
}

fn role_missing() -> AppError {
    AppError::NotFound("role was not found".to_owned())
}

#[async_trait]
impl RoleRepository for MySqlRoleRepository {
    type Tx = MySqlTx;

    async fn list(&self, window: ListWindow) -> Result<Listing<RoleView>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&self.pool)
            .await?;
        let rows = sqlx::query(concat!(
            role_select!(),
            " \
             ORDER BY r.is_system DESC, FIELD(r.`key`, 'admin', 'manager', 'member'), r.name, r.id \
             LIMIT ? OFFSET ?"
        ))
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: self.with_grants(rows).await?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn find(&self, role_id: &RoleId) -> Result<Option<RoleView>, AppError> {
        let rows = sqlx::query(concat!(role_select!(), " WHERE r.id = ?"))
            .bind(role_id.as_str())
            .fetch_all(&self.pool)
            .await?;
        Ok(self.with_grants(rows).await?.pop())
    }

    async fn key_exists(&self, key: &RoleKey) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM roles WHERE `key` = ?)")
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await?;
        Ok(exists)
    }

    async fn create(&self, tx: &mut MySqlTx, role: NewRole) -> Result<(), AppError> {
        sqlx::query("INSERT INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, ?)")
            .bind(role.id.as_str())
            .bind(role.key.as_str())
            .bind(&role.name)
            .bind(role.is_system)
            .execute(&mut **tx)
            .await
            .map_err(map_key_conflict)?;
        insert_grants(&mut *tx, &role.id, &role.grants).await?;
        Ok(())
    }

    async fn update(
        &self,
        tx: &mut MySqlTx,
        role_id: &RoleId,
        changes: RoleChanges,
    ) -> Result<(), AppError> {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM roles WHERE id = ? FOR UPDATE")
                .bind(role_id.as_str())
                .fetch_optional(&mut **tx)
                .await?;
        if exists.is_none() {
            return Err(role_missing());
        }
        sqlx::query(
            "UPDATE roles SET name = ?, `key` = ?, updated_at = UTC_TIMESTAMP(6) \
             WHERE id = ?",
        )
        .bind(&changes.name)
        .bind(changes.key.as_str())
        .bind(role_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(map_key_conflict)?;
        if let Some(grants) = &changes.grants {
            sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
                .bind(role_id.as_str())
                .execute(&mut **tx)
                .await?;
            insert_grants(&mut *tx, role_id, grants).await?;
        }
        Ok(())
    }

    async fn delete(&self, tx: &mut MySqlTx, role_id: &RoleId) -> Result<(), AppError> {
        let deleted = sqlx::query("DELETE FROM roles WHERE id = ? AND is_system = FALSE")
            .bind(role_id.as_str())
            .execute(&mut **tx)
            .await
            .map_err(|error| {
                if let sqlx::Error::Database(database_error) = &error
                    && database_error.is_foreign_key_violation()
                {
                    return AppError::conflict_code(
                        ROLE_IN_USE,
                        "the role is assigned to members of the organization",
                    );
                }
                AppError::from(error)
            })?;
        if deleted.rows_affected() != 1 {
            return Err(role_missing());
        }
        Ok(())
    }
}
