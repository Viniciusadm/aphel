use aphel_sdk::notifications::NotificationKindExt;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::{
    DeliverableNotification, NewNotification, NotificationFilter, NotificationRepository,
    NotificationView,
};
use crate::domain::{NotificationCursor, NotificationKind};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::repositories::mapper::user_reference;

const NOTIFICATION_SELECT: &str = "SELECT n.id, n.user_id, n.kind, n.title, n.body, n.record_type, n.record_id, \
     n.actor_id, au.display_name AS actor_name, n.link, n.data, n.read_at, n.emailed_at, n.created_at \
     FROM notifications n LEFT JOIN users au ON au.id = n.actor_id ";

fn notification_from_row(row: &MySqlRow) -> Result<NotificationView, AppError> {
    let kind: String = row.try_get("kind")?;
    let record_type: Option<String> = row.try_get("record_type")?;
    let record_id: Option<String> = row.try_get("record_id")?;
    let record = match (record_type, record_id) {
        (Some(record_type), Some(record_id)) => Some(
            RecordRef::parse(&record_type, &record_id)
                .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        ),
        _ => None,
    };
    let Json(data): Json<Value> = row.try_get("data")?;
    let read_at: Option<NaiveDateTime> = row.try_get("read_at")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    Ok(NotificationView {
        id: uuid_column(row, "id")?,
        kind: NotificationKind::parse(&kind)
            .ok_or_else(|| mapper::invalid_column("kind", &kind))?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        record,
        actor: user_reference(row, "actor_id", "actor_name")?,
        link: row.try_get("link")?,
        data,
        read_at: read_at.map(mapper::utc),
        created_at: mapper::utc(created_at),
    })
}

#[derive(Clone)]
pub struct MySqlNotificationRepository {
    pool: MySqlPool,
}

impl MySqlNotificationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const INBOX_FILTER: &str = "n.user_id = ? AND n.in_app = TRUE AND (? = FALSE OR n.read_at IS NULL)";

#[async_trait]
impl NotificationRepository for MySqlNotificationRepository {
    type Tx = Tx;

    async fn insert(&self, tx: &mut Tx, notification: &NewNotification) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT INTO notifications (id, user_id, kind, title, body, record_type, record_id, \
             actor_id, link, data, dedupe_key, in_app, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(notification.id.to_string())
        .bind(notification.user_id.as_str())
        .bind(notification.kind.as_str())
        .bind(truncate(&notification.title, 255))
        .bind(&notification.body)
        .bind(
            notification
                .record
                .as_ref()
                .map(|record| record.record_type.as_str().to_owned()),
        )
        .bind(
            notification
                .record
                .as_ref()
                .map(|record| record.record_id.to_string()),
        )
        .bind(notification.actor_id.as_ref().map(UserId::as_str))
        .bind(&notification.link)
        .bind(Json(&notification.data))
        .bind(&notification.dedupe_key)
        .bind(notification.in_app)
        .bind(mapper::naive(notification.created_at))
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => Ok(true),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    async fn list(
        &self,
        scope: &AccessScope,
        filter: NotificationFilter,
        after: Option<NotificationCursor>,
        limit: u32,
    ) -> Result<Vec<NotificationView>, AppError> {
        let cursor_filter = if after.is_some() {
            " AND (n.created_at < ? OR (n.created_at = ? AND n.id < ?))"
        } else {
            ""
        };
        let sql = format!(
            "{NOTIFICATION_SELECT} WHERE {INBOX_FILTER}{cursor_filter} ORDER BY n.created_at DESC, n.id DESC LIMIT ?"
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(scope.user_id().as_str())
            .bind(filter.unread_only);
        if let Some(cursor) = after {
            let created_at = mapper::naive(cursor.created_at);
            query = query
                .bind(created_at)
                .bind(created_at)
                .bind(cursor.id.to_string());
        }
        let rows = query.bind(limit).fetch_all(&self.pool).await?;
        rows.iter().map(notification_from_row).collect()
    }

    async fn count_unread(&self, scope: &AccessScope) -> Result<u64, AppError> {
        let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM notifications n WHERE {INBOX_FILTER}"
        )))
        .bind(scope.user_id().as_str())
        .bind(true)
        .fetch_one(&self.pool)
        .await?;
        Ok(u64::try_from(total).unwrap_or_default())
    }

    async fn mark_read(
        &self,
        scope: &AccessScope,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM notifications WHERE user_id = ? AND id = ? AND in_app = TRUE",
        )
        .bind(scope.user_id().as_str())
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        if exists.is_none() {
            return Ok(false);
        }
        sqlx::query(
            "UPDATE notifications SET read_at = ? \
             WHERE user_id = ? AND id = ? AND read_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(scope.user_id().as_str())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(true)
    }

    async fn mark_all_read(
        &self,
        scope: &AccessScope,
        now: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE notifications SET read_at = ? \
             WHERE user_id = ? AND in_app = TRUE AND read_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(scope.user_id().as_str())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    async fn find_deliverable(
        &self,
        id: Uuid,
    ) -> Result<Option<DeliverableNotification>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{NOTIFICATION_SELECT} WHERE n.id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let emailed_at: Option<NaiveDateTime> = row.try_get("emailed_at")?;
            Ok(DeliverableNotification {
                user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                view: notification_from_row(&row)?,
                emailed_at: emailed_at.map(mapper::utc),
            })
        })
        .transpose()
    }

    async fn mark_emailed(&self, id: Uuid, now: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE notifications SET emailed_at = ? WHERE id = ? AND emailed_at IS NULL")
            .bind(mapper::naive(now))
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
