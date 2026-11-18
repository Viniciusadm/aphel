use aphel_sdk::AppError;
use aphel_sdk::notifications::{DigestSource, DueTask};
use aphel_sdk::platform::database::Db;
use aphel_sdk::points::UserId;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

pub(crate) struct OpenTasks;

#[async_trait]
impl DigestSource for OpenTasks {
    async fn open_tasks_due(
        &self,
        pool: &Db,
        owner: Option<&UserId>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<DueTask>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT a.id, a.owner_id, a.title, a.due_at FROM tasks a \
             JOIN users m ON m.id = a.owner_id \
             WHERE a.status = 'open' AND a.deleted_at IS NULL AND m.status = 'active' AND a.due_at > ",
        );
        builder
            .push_bind(from.naive_utc())
            .push(" AND a.due_at <= ")
            .push_bind(to.naive_utc());
        if let Some(owner) = owner {
            builder
                .push(" AND a.owner_id = ")
                .push_bind(owner.as_str().to_owned());
        }
        builder
            .push(" ORDER BY a.due_at, a.id LIMIT ")
            .push_bind(limit);
        let rows = builder.build().fetch_all(pool).await?;
        rows.iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                let owner_id: String = row.try_get("owner_id")?;
                let due_at: NaiveDateTime = row.try_get("due_at")?;
                Ok(DueTask {
                    id: Uuid::parse_str(&id)
                        .map_err(|_| AppError::Internal("invalid activity id".to_owned()))?,
                    owner_id: UserId::new(owner_id)
                        .map_err(|_| AppError::Internal("invalid owner id".to_owned()))?,
                    title: row.try_get("title")?,
                    due_at: due_at.and_utc(),
                })
            })
            .collect()
    }
}
