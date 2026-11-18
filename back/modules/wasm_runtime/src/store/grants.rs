use uuid::Uuid;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;

pub(crate) async fn role_id(tx: &mut Tx, key: &str) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
        .bind(key)
        .fetch_optional(&mut **tx)
        .await?)
}

pub(crate) async fn create_role(tx: &mut Tx, key: &str, name: &str) -> Result<bool, AppError> {
    Ok(
        sqlx::query(
            "INSERT IGNORE INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, FALSE)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(key)
        .bind(name)
        .execute(&mut **tx)
        .await?
        .rows_affected()
            > 0,
    )
}

pub(crate) async fn grant(
    tx: &mut Tx,
    role_id: &str,
    permission: &str,
    scope: &str,
) -> Result<u64, AppError> {
    Ok(sqlx::query(
        "INSERT IGNORE INTO role_permissions (role_id, permission, scope) VALUES (?, ?, ?)",
    )
    .bind(role_id)
    .bind(permission)
    .bind(scope)
    .execute(&mut **tx)
    .await?
    .rows_affected())
}

pub(crate) async fn grant_entity_to_system_roles(
    tx: &mut Tx,
    record_type: &str,
    scope: &str,
) -> Result<u64, AppError> {
    Ok(sqlx::query(
        "INSERT IGNORE INTO role_permissions (role_id, permission, scope) \
         SELECT r.id, CONCAT(?, '.', p.action), ? \
         FROM roles r JOIN ( \
           SELECT 'read' AS action UNION ALL SELECT 'create' UNION ALL SELECT 'update' \
           UNION ALL SELECT 'delete' UNION ALL SELECT 'archive' UNION ALL SELECT 'export' \
         ) p \
         WHERE r.is_system = TRUE \
         AND (r.`key` = 'manager' OR (r.`key` = 'member' AND p.action <> 'export'))",
    )
    .bind(record_type)
    .bind(scope)
    .execute(&mut **tx)
    .await?
    .rows_affected())
}

pub(crate) async fn role_permissions(
    pool: &Db,
    key: &str,
) -> Result<Option<Vec<String>>, AppError> {
    let id: Option<String> = sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    let Some(id) = id else {
        return Ok(None);
    };
    Ok(Some(
        sqlx::query_scalar::<_, String>(
            "SELECT permission FROM role_permissions WHERE role_id = ?",
        )
        .bind(id)
        .fetch_all(pool)
        .await?,
    ))
}
