use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{Follow, FollowOrigin, RecordFollower};
use aphel_kernel::Ctx;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::users::UserStatus;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

fn origin(value: &str) -> Result<FollowOrigin, AppError> {
    FollowOrigin::parse(value).ok_or_else(|| mapper::invalid_column("origin", value))
}

#[derive(Clone)]
pub(crate) struct FollowerStore {
    pool: MySqlPool,
}

impl FollowerStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn followers(
        &self,
        record: &RecordRef,
    ) -> Result<Vec<RecordFollower>, AppError> {
        let rows = sqlx::query(
            "SELECT f.user_id, u.display_name, f.origin, f.created_at FROM record_followers f \
             JOIN users u ON u.id = f.user_id \
             WHERE f.record_type = ? AND f.record_id = ? \
             AND u.status = ? \
             ORDER BY f.created_at, f.id",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(UserStatus::Active.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let origin_value: String = row.try_get("origin")?;
                let created_at: NaiveDateTime = row.try_get("created_at")?;
                Ok(RecordFollower {
                    user: UserReference {
                        id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                        display_name: row.try_get("display_name")?,
                    },
                    origin: origin(&origin_value)?,
                    followed_at: mapper::utc(created_at),
                })
            })
            .collect()
    }

    pub(crate) async fn follower_ids(&self, record: &RecordRef) -> Result<Vec<UserId>, AppError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT f.user_id FROM record_followers f \
             JOIN users m ON m.id = f.user_id \
             WHERE f.record_type = ? AND f.record_id = ? AND m.status = ? \
             ORDER BY f.created_at, f.id",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(UserStatus::Active.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }

    pub(crate) async fn follow(
        &self,
        record: &RecordRef,
        user_id: &UserId,
        origin: FollowOrigin,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO record_followers (id, record_type, record_id, user_id, origin, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(user_id.as_str())
        .bind(origin.as_str())
        .bind(mapper::naive(at))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn unfollow(
        &self,
        record: &RecordRef,
        user_id: &UserId,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "DELETE FROM record_followers WHERE record_type = ? \
             AND record_id = ? AND user_id = ?",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(user_id.as_str())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn followed(
        &self,
        user_id: &UserId,
        record_type: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Follow>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT record_type, record_id, origin, created_at FROM record_followers \
             WHERE ",
        );
        builder
            .push("user_id = ")
            .push_bind(user_id.as_str().to_owned());
        if let Some(record_type) = record_type {
            builder
                .push(" AND record_type = ")
                .push_bind(record_type.to_owned());
        }
        builder
            .push(" ORDER BY created_at DESC, id DESC LIMIT ")
            .push_bind(limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let record_type: String = row.try_get("record_type")?;
                let record_id: String = row.try_get("record_id")?;
                let origin_value: String = row.try_get("origin")?;
                let created_at: NaiveDateTime = row.try_get("created_at")?;
                Ok(Follow {
                    record: RecordRef::parse(&record_type, &record_id)?,
                    origin: origin(&origin_value)?,
                    followed_at: mapper::utc(created_at),
                })
            })
            .collect()
    }
}
