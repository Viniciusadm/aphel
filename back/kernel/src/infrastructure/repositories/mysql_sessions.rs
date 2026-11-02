use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySqlPool, Row};

use crate::application::auth::{NewSession, SessionRecord, SessionRepository, SessionStage};
use crate::domain::{SessionId, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlSessionRepository {
    pool: MySqlPool,
}

impl MySqlSessionRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for MySqlSessionRepository {
    async fn create(&self, session: NewSession) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, stage, ip, user_agent, created_at, last_seen_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(session.user_id.as_str())
        .bind(&session.token_hash)
        .bind(session.stage.as_str())
        .bind(session.ip.as_deref())
        .bind(session.user_agent.as_deref())
        .bind(mapper::naive(session.created_at))
        .bind(mapper::naive(session.created_at))
        .bind(mapper::naive(session.expires_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<SessionRecord>, AppError> {
        let row = sqlx::query(
            "SELECT id, user_id, stage, two_factor_attempts, last_seen_at, expires_at, revoked_at FROM sessions WHERE token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            let revoked_at: Option<NaiveDateTime> = row.try_get("revoked_at")?;
            let stage: String = row.try_get("stage")?;
            Ok(SessionRecord {
                id: mapper::typed_id(row.try_get("id")?, SessionId::new)?,
                user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                stage: SessionStage::parse(&stage)
                    .ok_or_else(|| mapper::invalid_column("sessions.stage", &stage))?,
                two_factor_attempts: row.try_get("two_factor_attempts")?,
                last_seen_at: mapper::utc(row.try_get("last_seen_at")?),
                expires_at: mapper::utc(row.try_get("expires_at")?),
                revoked_at: revoked_at.map(mapper::utc),
            })
        })
        .transpose()
    }

    async fn touch(&self, id: &SessionId, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE id = ?")
            .bind(mapper::naive(at))
            .bind(id.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke(&self, id: &SessionId, at: DateTime<Utc>) -> Result<(), AppError> {
        let mut tx: crate::database::transaction::MySqlTx = self.pool.begin().await?.into();
        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(mapper::naive(at))
            .bind(id.as_str())
            .execute(&mut *tx)
            .await?;
        crate::application::security::session_revoked(&mut tx, &self.pool, Some(id), None, None)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn record_two_factor_failure(&self, id: &SessionId) -> Result<u32, AppError> {
        sqlx::query(
            "UPDATE sessions SET two_factor_attempts = two_factor_attempts + 1 WHERE id = ?",
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await?;
        let attempts: u32 =
            sqlx::query_scalar("SELECT two_factor_attempts FROM sessions WHERE id = ?")
                .bind(id.as_str())
                .fetch_one(&self.pool)
                .await?;
        Ok(attempts)
    }

    async fn revoke_all_for_user(
        &self,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let mut tx: crate::database::transaction::MySqlTx = self.pool.begin().await?.into();
        let result = sqlx::query(
            "UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(user_id.as_str())
        .execute(&mut *tx)
        .await?;
        crate::application::security::session_revoked(
            &mut tx,
            &self.pool,
            None,
            Some(user_id),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(result.rows_affected())
    }
}
