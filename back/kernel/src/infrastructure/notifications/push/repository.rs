use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::files::sha256_hex;
use crate::application::notifications::push::{
    NewPushSubscription, PushSubscriptionRepository, PushTarget,
};
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct MySqlPushSubscriptionRepository {
    pool: MySqlPool,
}

impl MySqlPushSubscriptionRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn endpoint_hash(endpoint: &str) -> String {
    sha256_hex(endpoint.as_bytes())
}

#[async_trait]
impl PushSubscriptionRepository for MySqlPushSubscriptionRepository {
    async fn upsert(&self, subscription: &NewPushSubscription) -> Result<(), AppError> {
        let now = mapper::naive(subscription.now);
        sqlx::query(
            "INSERT INTO push_subscriptions (id, user_id, session_id, endpoint, endpoint_hash, p256dh, auth, user_agent, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE user_id = VALUES(user_id), session_id = VALUES(session_id), endpoint = VALUES(endpoint), \
             p256dh = VALUES(p256dh), auth = VALUES(auth), user_agent = VALUES(user_agent), updated_at = VALUES(updated_at)",
        )
        .bind(subscription.id.to_string())
        .bind(subscription.user_id.as_str())
        .bind(subscription.session_id.as_str())
        .bind(&subscription.endpoint)
        .bind(endpoint_hash(&subscription.endpoint))
        .bind(&subscription.p256dh)
        .bind(&subscription.auth)
        .bind(subscription.user_agent.as_deref())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove(&self, user_id: &UserId, endpoint: &str) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM push_subscriptions WHERE endpoint_hash = ? AND user_id = ?")
                .bind(endpoint_hash(endpoint))
                .bind(user_id.as_str())
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn active_targets(
        &self,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<Vec<PushTarget>, AppError> {
        let now = mapper::naive(now);
        sqlx::query(
            "DELETE p FROM push_subscriptions p JOIN sessions s ON s.id = p.session_id \
             WHERE p.user_id = ? AND (s.revoked_at IS NOT NULL OR s.expires_at <= ? OR s.user_id <> p.user_id)",
        )
        .bind(user_id.as_str())
        .bind(now)
        .execute(&self.pool)
        .await?;
        let rows = sqlx::query(
            "SELECT p.id, p.endpoint, p.p256dh, p.auth FROM push_subscriptions p \
             JOIN sessions s ON s.id = p.session_id \
             WHERE p.user_id = ? AND s.user_id = p.user_id AND s.revoked_at IS NULL AND s.expires_at > ? \
             ORDER BY p.id",
        )
        .bind(user_id.as_str())
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                Ok(PushTarget {
                    id: Uuid::parse_str(&id)
                        .map_err(|_| mapper::invalid_column("push_subscriptions.id", &id))?,
                    endpoint: row.try_get("endpoint")?,
                    p256dh: row.try_get("p256dh")?,
                    auth: row.try_get("auth")?,
                })
            })
            .collect()
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM push_subscriptions WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_used(&self, id: Uuid, now: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE push_subscriptions SET last_used_at = ? WHERE id = ?")
            .bind(mapper::naive(now))
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn claim_notification(
        &self,
        notification_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE notifications SET pushed_at = ? WHERE id = ? AND pushed_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(notification_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn release_notification(&self, notification_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE notifications SET pushed_at = NULL WHERE id = ?")
            .bind(notification_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
