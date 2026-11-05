use chrono::{DateTime, Utc};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::compose::SenderProfile;
use crate::domain::EmailStatus;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct OutboundStore {
    pool: MySqlPool,
}

impl OutboundStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl OutboundStore {
    pub(crate) async fn transition_status(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        from: EmailStatus,
        to: EmailStatus,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE email_messages SET status = ?, updated_at = ? \
             WHERE id = ? AND status = ? AND direction = 'outbound'",
        )
        .bind(to.as_str())
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .bind(from.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn sender_profile(
        &self,
        user: &UserId,
    ) -> Result<Option<SenderProfile>, AppError> {
        let row = sqlx::query("SELECT display_name, email FROM users WHERE id = ? LIMIT 1")
            .bind(user.as_str())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| {
            Ok(SenderProfile {
                display_name: row.try_get("display_name")?,
                email: row.try_get("email")?,
            })
        })
        .transpose()
    }

    pub(crate) async fn replace_tracked_links(
        &self,
        tx: &mut MySqlTx,
        message_id: Uuid,
        urls: &[String],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM email_tracked_links WHERE message_id = ?")
            .bind(message_id.to_string())
            .execute(&mut **tx)
            .await?;
        if urls.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO email_tracked_links (id, message_id, position, url, created_at) ",
        );
        builder.push_values(urls.iter().enumerate(), |mut values, (position, url)| {
            values
                .push_bind(Uuid::now_v7().to_string())
                .push_bind(message_id.to_string())
                .push_bind(u32::try_from(position).unwrap_or(u32::MAX))
                .push_bind(url.clone())
                .push_bind(mapper::naive(now));
        });
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn tracked_link(
        &self,
        message_id: Uuid,
        position: u32,
    ) -> Result<Option<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT url FROM email_tracked_links WHERE message_id = ? AND position = ?",
        )
        .bind(message_id.to_string())
        .bind(position)
        .fetch_optional(&self.pool)
        .await?)
    }
}
