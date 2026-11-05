use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::NewDemoVisitor;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::domain::access::RecordScope;
use aphel_sdk::platform::infrastructure::repositories::mapper;

pub(crate) async fn ensure_visitor(
    pool: &MySqlPool,
    visitor: &NewDemoVisitor,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let role_id: Option<String> = sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
        .bind(&visitor.role_key)
        .fetch_optional(&mut *tx)
        .await?;
    let role_id = match role_id {
        Some(id) => {
            sqlx::query("UPDATE roles SET name = ? WHERE id = ?")
                .bind(&visitor.role_name)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
            id
        }
        None => {
            let id = Uuid::now_v7().to_string();
            sqlx::query("INSERT INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, FALSE)")
                .bind(&id)
                .bind(&visitor.role_key)
                .bind(&visitor.role_name)
                .execute(&mut *tx)
                .await?;
            id
        }
    };
    sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
        .bind(&role_id)
        .execute(&mut *tx)
        .await?;
    if !visitor.permissions.is_empty() {
        let mut insert =
            QueryBuilder::new("INSERT INTO role_permissions (role_id, permission, scope) ");
        insert.push_values(&visitor.permissions, |mut row, permission| {
            row.push_bind(&role_id)
                .push_bind(permission.as_str())
                .push_bind(RecordScope::Organization.as_str());
        });
        insert.build().execute(&mut *tx).await?;
    }

    let verified_at = mapper::naive(visitor.verified_at);
    let existing = sqlx::query("SELECT id FROM users WHERE email = ?")
        .bind(visitor.email.as_str())
        .fetch_optional(&mut *tx)
        .await?;
    match existing {
        Some(row) => {
            let id: String = row.try_get("id")?;
            sqlx::query(
                "UPDATE users SET password_hash = ?, display_name = ?, status = 'active', role_id = ?, email_verified_at = ? WHERE id = ?",
            )
            .bind(&visitor.password_hash)
            .bind(&visitor.display_name)
            .bind(&role_id)
            .bind(verified_at)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            for statement in [
                "DELETE FROM user_two_factor WHERE user_id = ?",
                "DELETE FROM two_factor_recovery_codes WHERE user_id = ?",
                "DELETE FROM sessions WHERE user_id = ?",
                "DELETE FROM api_tokens WHERE created_by = ?",
            ] {
                sqlx::query(statement).bind(&id).execute(&mut *tx).await?;
            }
        }
        None => {
            sqlx::query(
                "INSERT INTO users (id, email, password_hash, display_name, status, role_id, joined_at, email_verified_at) \
                 VALUES (?, ?, ?, ?, 'active', ?, ?, ?)",
            )
            .bind(visitor.user_id.as_str())
            .bind(visitor.email.as_str())
            .bind(&visitor.password_hash)
            .bind(&visitor.display_name)
            .bind(&role_id)
            .bind(verified_at)
            .bind(verified_at)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
