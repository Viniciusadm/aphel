use aphel_kernel::domain::{UserId, users::UserStatus};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::tasks::{DueTaskReminder, TaskReminderSource};
use chrono::{DateTime, Utc};
use sqlx::Row;
pub(crate) struct Reminders;
#[async_trait::async_trait]
impl TaskReminderSource for Reminders {
    async fn due(
        &self,
        pool: &Db,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DueTaskReminder>, AppError> {
        if !(1..=1000).contains(&limit) {
            return Err(AppError::Validation(
                "task reminder limit must be between 1 and 1000".into(),
            ));
        }
        let rows = sqlx::query(
            "SELECT a.id, a.owner_id, a.title, a.due_at, a.reminder_at, a.reminder_minutes_before \
             FROM tasks a \
             JOIN users m ON m.id = a.owner_id \
             WHERE a.status = 'open' AND a.reminder_sent_at IS NULL AND a.reminder_at > ? AND a.reminder_at <= ? \
             AND a.deleted_at IS NULL AND a.due_at IS NOT NULL AND m.status = ? \
             ORDER BY a.reminder_at, a.id LIMIT ?",
        )
        .bind(mapper::naive(since))
        .bind(mapper::naive(now))
        .bind(UserStatus::Active.as_str())
        .bind(limit)
        .fetch_all(pool)
        .await?;
        rows.iter()
            .map(|row| {
                let minutes: Option<u32> = row.try_get("reminder_minutes_before")?;
                Ok(DueTaskReminder {
                    id: uuid_column(row, "id")?,
                    owner_id: mapper::typed_id(row.try_get("owner_id")?, UserId::new)?,
                    title: row.try_get("title")?,
                    due_at: mapper::instant(row, "due_at")?,
                    reminder_at: mapper::instant(row, "reminder_at")?,
                    minutes_before: minutes.unwrap_or_default(),
                })
            })
            .collect()
    }

    async fn mark_sent(
        &self,
        pool: &Db,
        reminder: &DueTaskReminder,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET reminder_sent_at = ?, updated_at = updated_at \
             WHERE id = ? AND reminder_at = ? AND reminder_sent_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(reminder.id.to_string())
        .bind(mapper::naive(reminder.reminder_at))
        .execute(pool)
        .await?;
        Ok(())
    }
}
