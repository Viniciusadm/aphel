use sqlx::MySqlPool;

pub(crate) async fn installer_answers(
    pool: &MySqlPool,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let document: Option<Option<serde_json::Value>> = sqlx::query_scalar(
        "SELECT JSON_EXTRACT(settings, '$.installer') FROM organization_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(document.flatten())
}

pub(crate) async fn ensure_role(
    pool: &MySqlPool,
    key: &str,
    name: &str,
) -> Result<String, sqlx::Error> {
    sqlx::query("INSERT IGNORE INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, FALSE)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(key)
        .bind(name)
        .execute(pool)
        .await?;
    sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
        .bind(key)
        .fetch_one(pool)
        .await
}

pub(crate) async fn grant_permission(
    pool: &MySqlPool,
    role_id: &str,
    permission: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT IGNORE INTO role_permissions (role_id, permission, scope) VALUES (?, ?, 'organization')",
    )
    .bind(role_id)
    .bind(permission)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn record_configuration_error(
    pool: &MySqlPool,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    match error {
        None => {
            sqlx::query(
                "UPDATE organization_settings SET settings = JSON_REMOVE(COALESCE(settings, JSON_OBJECT()), '$.installer.configuration_error') WHERE id = 1",
            )
            .execute(pool)
            .await?;
        }
        Some(message) => {
            sqlx::query(
                "UPDATE organization_settings SET settings = JSON_SET(COALESCE(settings, JSON_OBJECT()), '$.installer.configuration_error', ?) WHERE id = 1",
            )
            .bind(message)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
