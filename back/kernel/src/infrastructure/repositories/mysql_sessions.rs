use std::sync::LazyLock;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{AssertSqlSafe, MySqlPool, Row};

use crate::application::auth::{NewSession, SessionRecord, SessionRepository, SessionStage};
use crate::domain::users::User;
use crate::domain::{SessionId, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::mysql_users::{USER_COLUMNS, user_from_row};

#[derive(Clone)]
pub struct MySqlSessionRepository {
    pool: MySqlPool,
}

impl MySqlSessionRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn session_from_row(row: &MySqlRow, prefix: &str) -> Result<SessionRecord, AppError> {
    let column = |name: &str| format!("{prefix}{name}");
    let revoked_at: Option<NaiveDateTime> = row.try_get(column("revoked_at").as_str())?;
    let stage: String = row.try_get(column("stage").as_str())?;
    Ok(SessionRecord {
        id: mapper::typed_id(row.try_get(column("id").as_str())?, SessionId::new)?,
        user_id: mapper::typed_id(row.try_get(column("user_id").as_str())?, UserId::new)?,
        stage: SessionStage::parse(&stage)
            .ok_or_else(|| mapper::invalid_column("sessions.stage", &stage))?,
        two_factor_attempts: row.try_get(column("two_factor_attempts").as_str())?,
        last_seen_at: mapper::utc(row.try_get(column("last_seen_at").as_str())?),
        expires_at: mapper::utc(row.try_get(column("expires_at").as_str())?),
        revoked_at: revoked_at.map(mapper::utc),
    })
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

        row.as_ref()
            .map(|row| session_from_row(row, ""))
            .transpose()
    }

    async fn find_with_user_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<(SessionRecord, User)>, AppError> {
        static SQL: LazyLock<String> = LazyLock::new(|| {
            let user_columns = USER_COLUMNS
                .split(", ")
                .map(|column| format!("u.{column}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "SELECT s.id AS session_id, s.user_id AS session_user_id, s.stage AS session_stage, \
                 s.two_factor_attempts AS session_two_factor_attempts, \
                 s.last_seen_at AS session_last_seen_at, s.expires_at AS session_expires_at, \
                 s.revoked_at AS session_revoked_at, {user_columns} \
                 FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ?"
            )
        });
        let row = sqlx::query(AssertSqlSafe(SQL.as_str()))
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref()
            .map(|row| Ok((session_from_row(row, "session_")?, user_from_row(row)?)))
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
        let mut tx: crate::database::transaction::Tx = self.pool.begin().await?.into();
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
        let mut tx: crate::database::transaction::Tx = self.pool.begin().await?.into();
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
