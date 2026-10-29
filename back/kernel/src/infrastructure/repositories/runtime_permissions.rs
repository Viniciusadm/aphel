use sqlx::MySqlPool;

use crate::database::transaction::MySqlTx;
use crate::domain::access::{Permission, PermissionDefinition};
use crate::errors::AppError;

pub struct RuntimeGrant<'a> {
    pub permission: &'a Permission,
    pub roles: &'a [String],
    pub label: &'a str,
    pub description: Option<&'a str>,
}

pub async fn active_runtime_permissions(
    pool: &MySqlPool,
) -> Result<Vec<PermissionDefinition>, AppError> {
    let keys: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT permission, label, description FROM runtime_permissions WHERE is_active ORDER BY permission",
    )
    .fetch_all(pool)
    .await?;
    Ok(keys
        .into_iter()
        .filter_map(|(key, label, description)| {
            Permission::parse(key)
                .ok()
                .map(|permission| PermissionDefinition {
                    permission,
                    record_scoped: true,
                    label,
                    description,
                })
        })
        .collect())
}

pub async fn replace_runtime_permissions(
    tx: &mut MySqlTx,
    owner: &str,
    grants: &[RuntimeGrant<'_>],
    active: bool,
) -> Result<(), AppError> {
    let previous: Vec<String> =
        sqlx::query_scalar("SELECT permission FROM runtime_permissions WHERE owner = ?")
            .bind(owner)
            .fetch_all(&mut **tx)
            .await?;
    for permission in previous.iter().filter(|permission| {
        !grants
            .iter()
            .any(|grant| grant.permission.as_str() == *permission)
    }) {
        sqlx::query("DELETE FROM role_permissions WHERE permission = ?")
            .bind(permission)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("DELETE FROM runtime_permissions WHERE owner = ?")
        .bind(owner)
        .execute(&mut **tx)
        .await?;
    for grant in grants {
        let taken: Option<String> =
            sqlx::query_scalar("SELECT owner FROM runtime_permissions WHERE permission = ?")
                .bind(grant.permission.as_str())
                .fetch_optional(&mut **tx)
                .await?;
        if taken.is_some() {
            return Err(AppError::conflict_code(
                "permission_already_registered",
                format!(
                    "the permission {} is already registered by another owner",
                    grant.permission.as_str()
                ),
            ));
        }
        sqlx::query(
            "INSERT INTO runtime_permissions (permission, owner, is_active, label, description) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(grant.permission.as_str())
        .bind(owner)
        .bind(active)
        .bind(grant.label)
        .bind(grant.description)
        .execute(&mut **tx)
        .await?;
        let newly_registered = !previous
            .iter()
            .any(|permission| permission == grant.permission.as_str());
        if newly_registered {
            for role in grant.roles {
                sqlx::query(
                    "INSERT IGNORE INTO role_permissions (role_id, permission, scope) \
                     SELECT id, ?, 'organization' FROM roles WHERE `key` = ?",
                )
                .bind(grant.permission.as_str())
                .bind(role)
                .execute(&mut **tx)
                .await?;
            }
        }
    }
    Ok(())
}

pub async fn set_runtime_permissions_active(
    tx: &mut MySqlTx,
    owner: &str,
    active: bool,
) -> Result<(), AppError> {
    sqlx::query("UPDATE runtime_permissions SET is_active = ? WHERE owner = ?")
        .bind(active)
        .bind(owner)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn remove_runtime_permissions(tx: &mut MySqlTx, owner: &str) -> Result<(), AppError> {
    sqlx::query(
        "DELETE rp FROM role_permissions rp \
         JOIN runtime_permissions p ON p.permission = rp.permission WHERE p.owner = ?",
    )
    .bind(owner)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM runtime_permissions WHERE owner = ?")
        .bind(owner)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
