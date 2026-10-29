use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};

use crate::application::auth::{
    NewUserToken, UserTokenPurpose, UserTokenRecord, UserTokenRepository,
};
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlUserTokenRepository {
    pool: MySqlPool,
}

impl MySqlUserTokenRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserTokenRepository for MySqlUserTokenRepository {
    async fn create(&self, token: NewUserToken) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO user_tokens (id, user_id, purpose, token_hash, payload, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&token.id)
        .bind(token.user_id.as_str())
        .bind(token.purpose.as_str())
        .bind(&token.token_hash)
        .bind(token.payload.map(Json))
        .bind(mapper::naive(token.expires_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_hash(
        &self,
        purpose: UserTokenPurpose,
        token_hash: &str,
    ) -> Result<Option<UserTokenRecord>, AppError> {
        let row = sqlx::query(
            "SELECT id, user_id, purpose, payload, expires_at, used_at FROM user_tokens WHERE token_hash = ? AND purpose = ?",
        )
        .bind(token_hash)
        .bind(purpose.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            let purpose: String = row.try_get("purpose")?;
            let payload: Option<Json<Value>> = row.try_get("payload")?;
            let used_at: Option<NaiveDateTime> = row.try_get("used_at")?;
            Ok(UserTokenRecord {
                id: row.try_get("id")?,
                user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                purpose: UserTokenPurpose::parse(&purpose)
                    .ok_or_else(|| mapper::invalid_column("user_tokens.purpose", &purpose))?,
                payload: payload.map(|Json(value)| value),
                expires_at: mapper::utc(row.try_get("expires_at")?),
                used_at: used_at.map(mapper::utc),
            })
        })
        .transpose()
    }

    async fn mark_used(&self, id: &str, at: DateTime<Utc>) -> Result<bool, AppError> {
        let result =
            sqlx::query("UPDATE user_tokens SET used_at = ? WHERE id = ? AND used_at IS NULL")
                .bind(mapper::naive(at))
                .bind(id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() == 1)
    }

    async fn invalidate_for_user(
        &self,
        user_id: &UserId,
        purpose: UserTokenPurpose,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE user_tokens SET used_at = ? WHERE user_id = ? AND purpose = ? AND used_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(user_id.as_str())
        .bind(purpose.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
