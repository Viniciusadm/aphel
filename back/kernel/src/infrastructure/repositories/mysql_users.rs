use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{AssertSqlSafe, MySqlPool, Row};

use crate::application::auth::UserRepository;
use crate::domain::users::{User, UserStatus};
use crate::domain::{NonEmptyString, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

pub(crate) const USER_COLUMNS: &str = "id, email, password_hash, display_name, email_verified_at, status, locale, timezone, avatar_file_id, last_login_at, created_at, updated_at";

#[derive(Clone)]
pub struct MySqlUserRepository {
    pool: MySqlPool,
}

impl MySqlUserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn find_by_column(&self, column: &str, value: &str) -> Result<Option<User>, AppError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE {column} = ?");
        let row = sqlx::query(AssertSqlSafe(sql))
            .bind(value)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(user_from_row).transpose()
    }

    async fn execute(
        &self,
        query: sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    ) -> Result<(), AppError> {
        query.execute(&self.pool).await?;
        Ok(())
    }
}

pub(crate) fn user_from_row(row: &MySqlRow) -> Result<User, AppError> {
    let status: String = row.try_get("status")?;
    let password_hash: Option<String> = row.try_get("password_hash")?;
    Ok(User {
        id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
        email: mapper::email(row.try_get("email")?)?,
        password_hash: password_hash
            .filter(|hash| !hash.is_empty())
            .map(|hash| mapper::non_empty(hash, "user.password_hash"))
            .transpose()?,
        display_name: mapper::non_empty(row.try_get("display_name")?, "user.display_name")?,
        email_verified_at: mapper::opt_datetime(row.try_get("email_verified_at")?)?,
        status: UserStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("users.status", &status))?,
        locale: row.try_get("locale")?,
        timezone: row.try_get("timezone")?,
        avatar_file_id: row.try_get("avatar_file_id")?,
        last_login_at: mapper::opt_datetime(row.try_get("last_login_at")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, AppError> {
        self.find_by_column("id", id.as_str()).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        self.find_by_column("email", email).await
    }

    async fn set_password(&self, id: &UserId, password_hash: &str) -> Result<(), AppError> {
        self.execute(
            sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
                .bind(password_hash)
                .bind(id.as_str()),
        )
        .await
    }

    async fn mark_email_verified(&self, id: &UserId, at: DateTime<Utc>) -> Result<(), AppError> {
        self.execute(
            sqlx::query(
                "UPDATE users SET email_verified_at = COALESCE(email_verified_at, ?) WHERE id = ?",
            )
            .bind(mapper::naive(at))
            .bind(id.as_str()),
        )
        .await
    }

    async fn record_login(&self, id: &UserId, at: DateTime<Utc>) -> Result<(), AppError> {
        self.execute(
            sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
                .bind(mapper::naive(at))
                .bind(id.as_str()),
        )
        .await
    }

    async fn complete_account(
        &self,
        id: &UserId,
        display_name: &NonEmptyString,
        password_hash: &str,
        verified_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        self.execute(
            sqlx::query(
                "UPDATE users SET display_name = ?, password_hash = ?, email_verified_at = COALESCE(email_verified_at, ?) WHERE id = ?",
            )
            .bind(display_name.as_str())
            .bind(password_hash)
            .bind(mapper::naive(verified_at))
            .bind(id.as_str()),
        )
        .await
    }

    async fn activate(&self, id: &UserId, joined_at: DateTime<Utc>) -> Result<(), AppError> {
        self.execute(
            sqlx::query(
                "UPDATE users SET status = 'active', joined_at = COALESCE(joined_at, ?) WHERE id = ? AND status = 'invited'",
            )
            .bind(mapper::naive(joined_at))
            .bind(id.as_str()),
        )
        .await
    }
}
