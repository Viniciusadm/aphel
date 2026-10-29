use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::notifications::{NotificationPreferenceRepository, PreferenceSetting};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::domain::notifications::{NotificationChannel, NotificationKind};
use crate::domain::users::UserStatus;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlNotificationPreferenceRepository {
    pool: MySqlPool,
}

impl MySqlNotificationPreferenceRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationPreferenceRepository for MySqlNotificationPreferenceRepository {
    async fn stored(&self, user_id: &UserId) -> Result<Vec<PreferenceSetting>, AppError> {
        let rows = sqlx::query(
            "SELECT kind, channel, enabled FROM notification_preferences WHERE user_id = ?",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        let mut settings = Vec::new();
        for row in rows {
            let kind: String = row.try_get("kind")?;
            let channel: String = row.try_get("channel")?;
            if let (Some(kind), Some(channel)) = (
                NotificationKind::parse(&kind),
                NotificationChannel::parse(&channel),
            ) {
                settings.push(PreferenceSetting {
                    kind,
                    channel,
                    enabled: row.try_get("enabled")?,
                });
            }
        }
        Ok(settings)
    }

    async fn save(
        &self,
        scope: &AccessScope,
        settings: &[PreferenceSetting],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if settings.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO notification_preferences (user_id, kind, channel, enabled, updated_at) ",
        );
        builder.push_values(settings, |mut row, setting| {
            row.push_bind(scope.user_id().as_str().to_owned())
                .push_bind(setting.kind.as_str())
                .push_bind(setting.channel.as_str())
                .push_bind(setting.enabled)
                .push_bind(mapper::naive(now));
        });
        builder.push(
            " ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), updated_at = VALUES(updated_at)",
        );
        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    async fn subscribers(
        &self,
        kind: NotificationKind,
        channel: NotificationChannel,
    ) -> Result<Vec<UserId>, AppError> {
        let ids: Vec<String> = if kind.default_enabled(channel) {
            sqlx::query_scalar(
                "SELECT m.id FROM users m \
                 LEFT JOIN notification_preferences p ON p.user_id = m.id AND p.kind = ? AND p.channel = ? \
                 WHERE m.status = ? AND (p.enabled IS NULL OR p.enabled = TRUE) \
                 ORDER BY m.id",
            )
            .bind(kind.as_str())
            .bind(channel.as_str())
            .bind(UserStatus::Active.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                "SELECT p.user_id FROM notification_preferences p \
                 JOIN users m ON m.id = p.user_id \
                 WHERE p.kind = ? AND p.channel = ? AND p.enabled = TRUE AND m.status = ? \
                 ORDER BY p.user_id",
            )
            .bind(kind.as_str())
            .bind(channel.as_str())
            .bind(UserStatus::Active.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        ids.into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }
}
