use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};

use crate::application::security::{AccountSessionRepository, ActiveSession};
use crate::domain::{SessionId, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const FULL_STAGE: &str = "full";
macro_rules! active_sessions_sql {
    ($suffix:literal) => {
        concat!(
            "SELECT id, ip, user_agent, created_at, last_seen_at, expires_at FROM sessions ",
            "WHERE user_id = ? AND stage = ? AND revoked_at IS NULL AND expires_at > ?",
            $suffix
        )
    };
}

#[derive(Clone)]
pub(crate) struct MySqlAccountSessionRepository {
    pool: MySqlPool,
}

impl MySqlAccountSessionRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn active_session(row: MySqlRow) -> Result<ActiveSession, AppError> {
    Ok(ActiveSession {
        id: mapper::typed_id(row.try_get("id")?, SessionId::new)?,
        ip: row.try_get("ip")?,
        user_agent: row.try_get("user_agent")?,
        created_at: mapper::utc(row.try_get("created_at")?),
        last_seen_at: mapper::utc(row.try_get("last_seen_at")?),
        expires_at: mapper::utc(row.try_get("expires_at")?),
    })
}

#[async_trait]
impl AccountSessionRepository for MySqlAccountSessionRepository {
    async fn list_active(
        &self,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<Vec<ActiveSession>, AppError> {
        let rows = sqlx::query(active_sessions_sql!(" ORDER BY last_seen_at DESC, id DESC"))
            .bind(user_id.as_str())
            .bind(FULL_STAGE)
            .bind(mapper::naive(now))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(active_session).collect()
    }

    async fn find_active(
        &self,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<ActiveSession>, AppError> {
        let row = sqlx::query(active_sessions_sql!(" AND id = ?"))
            .bind(user_id.as_str())
            .bind(FULL_STAGE)
            .bind(mapper::naive(now))
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(active_session).transpose()
    }

    async fn revoke(
        &self,
        user_id: &UserId,
        session_id: &SessionId,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let mut tx: crate::database::transaction::MySqlTx = self.pool.begin().await?.into();
        let result = sqlx::query(
            "UPDATE sessions SET revoked_at = ? WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(session_id.as_str())
        .bind(user_id.as_str())
        .execute(&mut *tx)
        .await?;
        let revoked = result.rows_affected() > 0;
        if revoked {
            crate::application::security::session_revoked(
                &mut tx,
                &self.pool,
                Some(session_id),
                None,
                None,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(revoked)
    }

    async fn revoke_others(
        &self,
        user_id: &UserId,
        keep: &SessionId,
        at: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let mut tx: crate::database::transaction::MySqlTx = self.pool.begin().await?.into();
        let result = sqlx::query(
            "UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND id <> ? AND revoked_at IS NULL AND expires_at > ?",
        )
        .bind(mapper::naive(at))
        .bind(user_id.as_str())
        .bind(keep.as_str())
        .bind(mapper::naive(at))
        .execute(&mut *tx)
        .await?;
        crate::application::security::session_revoked(
            &mut tx,
            &self.pool,
            None,
            Some(user_id),
            Some(keep),
        )
        .await?;
        tx.commit().await?;
        Ok(result.rows_affected())
    }
}
